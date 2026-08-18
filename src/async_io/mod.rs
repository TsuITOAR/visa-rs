//! Asynchronous transfers over a visa session.
//!
//! Visa's async operations are *completion based*: `viReadAsync`/`viWriteAsync` hand the
//! driver a raw pointer and return immediately, and the driver writes into that memory
//! until it posts an io-completion event. Two consequences shape everything here.
//!
//! The buffer must be **owned**, not borrowed, because `viTerminate`
//! only *requests* cancellation and gives no point at which the pointer is provably
//! free again.
//!
//! And the completion arrives on visa's own thread, possibly before the job id has even
//! been handed back to us, so job bookkeeping lives in a state machine keyed by job id.

mod callback;
mod future;

pub use future::{AsyncRead, AsyncWrite};

use callback::{Completed, JobMap};

use crate::{
    enums::{attribute, event, status::CompletionCode},
    session::AsRawSs,
    wrap_raw_error_in_unsafe, Instrument, JobID, Result,
};
use std::{
    task::{Context, Poll, Waker},
    time::Duration,
};
use visa_sys as vs;

/// An [`Instrument`] wired up for asynchronous transfers.
///
/// Field order matters and is load-bearing: `instr` is declared first, so on drop the
/// session is closed (which terminates anything still outstanding) *before* the callback
/// pack -- and any buffers it is still holding for cancelled jobs -- is freed.
pub struct AsyncInstrument {
    pub(super) instr: Instrument,
    callback: Box<JobMap>,
}

impl From<AsyncInstrument> for Instrument {
    fn from(mut async_instr: AsyncInstrument) -> Self {
        // The session outlives this conversion, so the handler has to come off before
        // the callback pack is freed, otherwise visa would keep a dangling `user_data`.
        async_instr.uninstall_handler();
        let async_instr = std::mem::ManuallyDrop::new(async_instr);
        // SAFETY: `async_instr` is never dropped, so reading both fields out of it moves
        // them exactly once. The handler is already uninstalled, which is all `Drop` does.
        let instr = unsafe { std::ptr::read(&async_instr.instr) };
        let callback = unsafe { std::ptr::read(&async_instr.callback) };
        drop(callback);
        instr
    }
}

impl AsyncInstrument {
    /// Wraps `instr` for asynchronous use.
    ///
    /// This sets `VI_ATTR_TERMCHAR_EN` and clears `VI_ATTR_SUPPRESS_END_EN` on the
    /// session so reads terminate on the termination character rather than only on a
    /// full count. The change is permanent for the session: it is *not* undone when the
    /// session is converted back into a plain [`Instrument`].
    pub fn new(instr: Instrument) -> Result<Self> {
        use crate::enums::attribute::HasAttribute;
        instr.set_attr(attribute::AttrTermcharEn::VI_TRUE)?;
        instr.set_attr(attribute::AttrSuppressEndEn::VI_FALSE)?;
        let mut callback = Box::new(JobMap::new());
        wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(JobMap::call_in_c),
            &mut *callback as *mut _ as _,
        ))?;
        instr.enable_event(
            event::EventKind::EventIoCompletion,
            event::Mechanism::Handler,
        )?;
        Ok(Self { instr, callback })
    }

    pub fn instrument(&self) -> &Instrument {
        &self.instr
    }

    /// Reads one complete response, resolving to the bytes received.
    ///
    /// Chunks are requested internally, so there is no length to guess: the read ends
    /// when the device does -- on its termination character or END indicator -- and the
    /// returned `Vec` is exactly what arrived. `timeout` is applied to the session as
    /// `VI_ATTR_TMO_VALUE` and bounds each chunk, not the whole call.
    ///
    /// For binary block data, whose bytes may legitimately include the termination
    /// character, use [`Self::async_read_exact`] instead.
    ///
    /// Visa allows only one asynchronous operation per session at a time, so starting a
    /// second while one is in flight resolves to
    /// [`ErrorInProgress`](crate::enums::status::ErrorCode::ErrorInProgress).
    pub fn async_read(&self, timeout: Duration) -> AsyncRead<'_> {
        AsyncRead::new(self, None, timeout)
    }

    /// Reads exactly `len` bytes, resolving to them.
    ///
    /// Unlike [`Self::async_read`] this disables `VI_ATTR_TERMCHAR_EN` for the transfer,
    /// so binary data is not cut short at a byte that happens to be the termination
    /// character. It resolves early only on error or timeout.
    pub fn async_read_exact(&self, len: usize, timeout: Duration) -> AsyncRead<'_> {
        AsyncRead::new(self, Some(len), timeout)
    }

    /// Writes all of `buf`, resolving to the number of bytes written.
    ///
    /// A visa transfer keeps reading from its buffer until the completion event arrives,
    /// so the buffer must be owned rather than borrowed. `Into<Vec<u8>>` accepts anything
    /// convenient -- `b"*IDN?\n"`, a `Vec<u8>`, a `&str`, a `String` -- and the types that
    /// already own a suitable allocation (`Vec<u8>`, `String`, `Box<[u8]>`) are moved
    /// rather than copied.
    pub fn async_write(&self, buf: impl Into<Vec<u8>>) -> AsyncWrite<'_> {
        AsyncWrite::new(self, buf.into())
    }

    /// Applies the per-call transfer settings to the session.
    ///
    /// These are session attributes, not per-operation arguments -- visa has no other
    /// place to put them -- so they persist after the transfer and are visible to any
    /// operation running concurrently on the same session.
    pub(super) fn prepare_read(&self, timeout: Duration, termchar: bool) -> Result<()> {
        use crate::enums::attribute::HasAttribute;
        let ms = timeout.as_millis().min(vs::VI_TMO_INFINITE as u128) as _;
        let tmo = attribute::AttrTmoValue::new_checked(ms).ok_or(crate::Error(
            crate::enums::status::ErrorCode::ErrorNsupAttrState,
        ))?;
        self.instr.set_attr(tmo)?;
        self.instr.set_attr(if termchar {
            attribute::AttrTermcharEn::VI_TRUE
        } else {
            attribute::AttrTermcharEn::VI_FALSE
        })?;
        Ok(())
    }

    /// Starts a read, handing `buf` to the callback pack for the duration of the
    /// transfer. If the call fails nothing was queued, so `buf` is simply dropped.
    pub(crate) fn start_read_id(&self, mut buf: Vec<u8>, waker: &Waker) -> Result<JobID> {
        let (job_id, completion) = unsafe { self.instr.visa_read_async(buf.as_mut())? };
        self.callback.register(job_id, waker, buf);
        note_sync_completion(completion, job_id);
        Ok(job_id)
    }

    /// Starts a write; see [`Self::start_read_id`].
    pub(crate) fn start_write_id(&self, buf: Vec<u8>, waker: &Waker) -> Result<JobID> {
        let (job_id, completion) = unsafe { self.instr.visa_write_async(&buf)? };
        self.callback.register(job_id, waker, buf);
        note_sync_completion(completion, job_id);
        Ok(job_id)
    }

    pub(crate) fn poll_job(
        &self,
        job_id: JobID,
        cx: &Context<'_>,
    ) -> Poll<(Result<Completed>, Vec<u8>)> {
        self.callback
            .poll_job(job_id, cx)
            .map(|(result, buf)| (result, buf.unwrap_or_default()))
    }

    /// Terminates `job_id`. The buffer stays owned by the callback pack until visa posts
    /// the completion: `viTerminate` only *requests* termination, and visa may still be
    /// writing into the buffer when it returns.
    pub(crate) fn cancel_job(&self, job_id: JobID) {
        self.callback.cancel(job_id);
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viTerminate(
            self.instr.as_raw_ss(),
            vs::VI_NULL as _,
            job_id.0
        )) {
            log::warn!("terminating unfinished async io: {}", e)
        };
    }
    /// Stops visa from delivering IO completion events to [`AsyncIoCallbackPack::call_in_c`].
    ///
    /// Must be called before the callback pack is freed, and before the session is handed
    /// back to a plain [`Instrument`]: visa holds a raw pointer to the pack as `user_data`,
    /// and leaving the event enabled with no handler installed is not a usable state.
    fn uninstall_handler(&mut self) {
        if let Err(e) = self.instr.disable_event(
            event::EventKind::EventIoCompletion,
            event::Mechanism::Handler,
        ) {
            log::warn!("error disabling io completion event: {}", e)
        };
        // Derived from a shared reference on purpose: visa only compares this pointer
        // against the installed entry, and a `&mut` here would both assert uniqueness
        // against an in-flight callback and invalidate the pointer visa still holds.
        let user_data = &*self.callback as *const JobMap as *mut std::ffi::c_void;
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viUninstallHandler(
            self.instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(JobMap::call_in_c),
            user_data,
        )) {
            log::warn!("error uninstalling handler: {}", e)
        };
    }
}

impl Drop for AsyncInstrument {
    fn drop(&mut self) {
        self.uninstall_handler();
    }
}

/// `VI_SUCCESS_SYNC` says the transfer already finished, which means visa is allowed to
/// have run the io completion handler before `viReadAsync`/`viWriteAsync` even handed the
/// job id back. There is nothing to short-circuit on this path: the byte count reaches us
/// only through the completion event either way, and if the callback did get there first
/// its result is already parked as [`JobState::Done`] for the next poll to collect. So
/// this is a trace point, not a special case -- the state machine makes the ordering
/// irrelevant.
fn note_sync_completion(completion: CompletionCode, job_id: JobID) {
    if completion == CompletionCode::SuccessSync {
        log::trace!("async io for job {} completed synchronously", job_id.0);
    }
}
