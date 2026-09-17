//! Asynchronous reads and writes for a VISA instrument.
//!
//! Use [`AsyncInstrument::async_read`] to read a response, [`AsyncInstrument::async_read_exact`]
//! for a fixed byte count, and [`AsyncInstrument::async_write`] to write data.
//! These futures wait for transfer completion; dropping one requests cancellation.
//!
//! Tests with NI-VISA 23.5 indicate that async I/O on macOS may block until timeout.

// VISA retains buffer pointers until IO completion, so transfers need owned storage.
// Cancellation alone does not prove the driver has stopped accessing that storage.
// Completions may arrive before submission returns a job ID; JobMap handles both orders.

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
pub struct AsyncInstrument {
    // Field order matters: close the session before dropping the callback pack.
    // Unfinished buffers are leaked if completion was never reported; the entire
    // pack is leaked if uninstall fails.
    pub(super) instr: Instrument,
    callback: Option<Box<JobMap>>,
}

impl From<AsyncInstrument> for Instrument {
    fn from(mut async_instr: AsyncInstrument) -> Self {
        // The session outlives this conversion, so the handler has to come off before
        // the callback pack is freed, otherwise visa would keep a dangling `user_data`.
        async_instr.uninstall_handler();
        let async_instr = std::mem::ManuallyDrop::new(async_instr);
        // SAFETY: `async_instr` is never dropped, so reading both fields out of it moves
        // them exactly once. The handler is uninstalled or its pack deliberately leaked,
        // which is all `Drop` does.
        let instr = unsafe { std::ptr::read(&async_instr.instr) };
        let callback = unsafe { std::ptr::read(&async_instr.callback) };
        drop(callback);
        instr
    }
}

impl AsyncInstrument {
    /// Wraps `instr` for asynchronous use.
    ///
    /// Enables termination-character and END detection. These settings remain in
    /// effect when converted back into an [`Instrument`].
    pub fn new(instr: Instrument) -> Result<Self> {
        use crate::enums::attribute::HasAttribute;
        instr.set_attr(attribute::AttrTermcharEn::VI_TRUE)?;
        instr.set_attr(attribute::AttrSuppressEndEn::VI_FALSE)?;
        let callback = Box::new(JobMap::new());
        wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(JobMap::call_in_c),
            &*callback as *const JobMap as *mut std::ffi::c_void,
        ))?;
        // From this point on, even an enable failure must uninstall the handler
        // before releasing its user-data allocation.
        let async_instr = Self {
            instr,
            callback: Some(callback),
        };
        async_instr.instr.enable_event(
            event::EventKind::EventIoCompletion,
            event::Mechanism::Handler,
        )?;
        Ok(async_instr)
    }

    fn callback(&self) -> &JobMap {
        self.callback
            .as_deref()
            .expect("callback present until teardown")
    }

    pub fn instrument(&self) -> &Instrument {
        &self.instr
    }

    /// Reads one complete response, resolving to the bytes received.
    ///
    /// Stops at the termination character or END indicator. `timeout` applies to each
    /// underlying read, not the total time to receive the response.
    ///
    /// For binary block data, whose bytes may legitimately include the termination
    /// character, use [`Self::async_read_exact`] instead.
    ///
    /// Starting another operation while one is in progress may return
    /// [`ErrorInProgress`](crate::enums::status::ErrorCode::ErrorInProgress).
    pub fn async_read(&self, timeout: Duration) -> AsyncRead<'_> {
        AsyncRead::new(self, None, timeout)
    }

    /// Reads exactly `len` bytes, resolving to them.
    ///
    /// Disables termination-character detection for binary data. Short reads are
    /// continued until `len` bytes arrive; a zero-byte read before then returns
    /// [`ErrorIo`](crate::enums::status::ErrorCode::ErrorIo).
    /// `timeout` applies to each underlying read, not the total operation.
    /// Read settings persist on the session. VISA errors and timeouts are returned immediately.
    pub fn async_read_exact(&self, len: usize, timeout: Duration) -> AsyncRead<'_> {
        AsyncRead::new(self, Some(len), timeout)
    }

    /// Writes `buf`, waiting for completion and returning the number of bytes written.
    ///
    /// Accepts inputs convertible to `Vec<u8>`, including byte slices, `Vec<u8>`, and
    /// `String`. Check the returned count for a short write.
    pub fn async_write(&self, buf: impl Into<Vec<u8>>) -> AsyncWrite<'_> {
        AsyncWrite::new(self, buf.into())
    }

    // These session attributes persist after the transfer and affect concurrent operations.
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

    // Give the callback pack ownership until completion. Submission failure drops buf.
    pub(crate) fn start_read_id(&self, mut buf: Vec<u8>, waker: &Waker) -> Result<JobID> {
        let (job_id, completion) = unsafe { self.instr.visa_read_async(buf.as_mut())? };
        self.callback().register(job_id, waker, buf);
        note_sync_completion(completion, job_id);
        Ok(job_id)
    }

    // Transfer buffer ownership as in start_read_id.
    pub(crate) fn start_write_id(&self, buf: Vec<u8>, waker: &Waker) -> Result<JobID> {
        let (job_id, completion) = unsafe { self.instr.visa_write_async(&buf)? };
        self.callback().register(job_id, waker, buf);
        note_sync_completion(completion, job_id);
        Ok(job_id)
    }

    pub(crate) fn poll_job(
        &self,
        job_id: JobID,
        cx: &Context<'_>,
    ) -> Poll<(Result<Completed>, Vec<u8>)> {
        self.callback()
            .poll_job(job_id, cx)
            .map(|(result, buf)| (result, buf.unwrap_or_default()))
    }

    // Keep the buffer until completion: viTerminate only requests cancellation.
    pub(crate) fn cancel_job(&self, job_id: JobID) {
        self.callback().cancel(job_id);
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viTerminate(
            self.instr.as_raw_ss(),
            vs::VI_NULL as _,
            job_id.0
        )) {
            log::warn!("terminating unfinished async io: {}", e)
        };
    }
    // Remove VISA's user_data reference before freeing the pack or returning the session.
    fn uninstall_handler(&mut self) {
        let Some(callback) = self.callback.as_deref() else {
            log::error!("cannot uninstall IO completion handler: callback state is missing");
            return;
        };
        if let Err(e) = self.instr.disable_event(
            event::EventKind::EventIoCompletion,
            event::Mechanism::Handler,
        ) {
            log::warn!("error disabling io completion event: {}", e)
        };
        // Derived from a shared reference on purpose: visa only compares this pointer
        // against the installed entry, and a `&mut` here would both assert uniqueness
        // against an in-flight callback and invalidate the pointer visa still holds.
        let user_data = callback as *const JobMap as *mut std::ffi::c_void;
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viUninstallHandler(
            self.instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(JobMap::call_in_c),
            user_data,
        )) {
            // VISA may still hold user_data. Leak before logging so even a panicking
            // logger cannot cause us to release memory still reachable by the driver.
            if let Some(callback) = self.callback.take() {
                let _ = Box::leak(callback);
            }
            log::warn!(
                "failed to uninstall IO completion handler: {}; leaked callback state to keep VISA's user-data pointer valid",
                e
            );
        };
    }
}

impl Drop for AsyncInstrument {
    fn drop(&mut self) {
        self.uninstall_handler();
    }
}

// VI_SUCCESS_SYNC may invoke the handler before submission returns. The completion
// count still comes from the event; JobMap parks early results for registration.
fn note_sync_completion(completion: CompletionCode, job_id: JobID) {
    if completion == CompletionCode::SuccessSync {
        log::trace!("async io for job {} completed synchronously", job_id.0);
    }
}
