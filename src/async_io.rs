use crate::{
    enums::{
        attribute::{self, SpecAttr},
        event,
        status::{CompletionCode, ErrorCode},
    },
    session::{AsRawSs, FromRawSs},
    wrap_raw_error_in_unsafe, Instrument, JobID, Result,
};
use dashmap::DashMap;
use indexmap::IndexMap;
use std::{
    future::Future,
    ops::AddAssign,
    sync::{
        mpsc::{Receiver, Sender, TryRecvError},
        Arc, Mutex, Weak,
    },
    task::{Poll, Waker},
};
use visa_sys as vs;

const CANCELED_CAP: usize = 32;

pub struct AsyncInstrument {
    pub(super) instr: Instrument,
    callback: Box<AsyncIoCallbackPack>,
}

impl From<AsyncInstrument> for Instrument {
    fn from(async_instr: AsyncInstrument) -> Self {
        let async_instr = std::mem::ManuallyDrop::new(async_instr);
        // SAFETY: We intentionally prevent drop of `async_instr` and take ownership of `instr`.
        // `instr` is not used afterward, and `async_instr` is never dropped.
        unsafe { std::ptr::read(&async_instr.instr) }
    }
}

impl AsyncInstrument {
    pub fn new(instr: Instrument) -> Result<Self> {
        use crate::enums::attribute::HasAttribute;
        instr.set_attr(attribute::AttrTermcharEn::VI_TRUE)?;
        instr.set_attr(attribute::AttrSuppressEndEn::VI_FALSE)?;
        let mut callback = Box::new(AsyncIoCallbackPack::new());
        wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(AsyncIoCallbackPack::call_in_c),
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

    pub fn async_read<'a>(&'a self, buf: &'a mut [u8]) -> AsyncRead<'a> {
        AsyncRead::new(self, buf)
    }

    pub fn async_write<'a>(&'a self, buf: &'a [u8]) -> AsyncWrite<'a> {
        AsyncWrite::new(self, buf)
    }

    pub(crate) fn start_read_id(&self, buf: &mut [u8], waker: &Waker) -> Result<AsyncId> {
        let (sender, rec) = std::sync::mpsc::channel();
        let waker = Arc::new(Mutex::new(waker.clone()));
        let job_id = unsafe { self.instr.visa_read_async(buf)? };
        self.callback.as_ref().add_job(job_id, sender, &waker);
        Ok(AsyncId { rec, waker, job_id })
    }

    pub(crate) fn start_write_id(&self, buf: &[u8], waker: &Waker) -> Result<AsyncId> {
        let (sender, rec) = std::sync::mpsc::channel();
        let waker = Arc::new(Mutex::new(waker.clone()));
        let job_id = unsafe { self.instr.visa_write_async(buf)? };
        self.callback.as_ref().add_job(job_id, sender, &waker);
        Ok(AsyncId { rec, waker, job_id })
    }

    pub(crate) fn cancel_job(&self, job_id: JobID) {
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viTerminate(
            self.instr.as_raw_ss(),
            vs::VI_NULL as _,
            job_id.0
        )) {
            log::warn!("terminating unfinished async io: {}", e)
        };
        self.callback.as_ref().remove_job(job_id);
    }
}

impl Drop for AsyncInstrument {
    fn drop(&mut self) {
        if let Err(e) = wrap_raw_error_in_unsafe!(vs::viUninstallHandler(
            self.instr.as_raw_ss(),
            event::EventKind::EventIoCompletion as _,
            Some(AsyncIoCallbackPack::call_in_c),
            &mut *self.callback as *mut _ as _,
        )) {
            log::warn!("error uninstalling handler: {}", e)
        };
    }
}

// Entry for a job that sends results back to a Future
#[derive(Clone)]
struct JobEntry {
    sender: Sender<Result<usize>>,
    waker: Weak<Mutex<Waker>>,
}

// Entry for a pending job that listener is not ready so that
// the receiver is not created yet
struct JobPending {
    status: Result<usize>,
}

struct AsyncIoCallbackPack {
    jobs: DashMap<JobID, JobEntry>,
    pending: DashMap<JobID, JobPending>,
    canceled: Mutex<IndexMap<JobID, ()>>,
}

impl AsyncIoCallbackPack {
    fn try_merge_pending(&self, job_id: JobID) {
        let pending_status = self.pending.remove(&job_id).map(|(_, p)| p.status);
        let Some(pending_status) = pending_status else {
            return;
        };
        if let Some(job) = self.jobs.get(&job_id) {
            if let Err(e) = job.sender.send(pending_status) {
                log::warn!("error sending pending job result: {}", e);
            }
            if let Some(waker) = job.waker.upgrade() {
                waker.lock().unwrap().wake_by_ref();
            } else {
                log::warn!("pending job waker already dropped");
            }
        } else {
            // put back if job not exist yet
            self.pending.insert(
                job_id,
                JobPending {
                    status: pending_status,
                },
            );
        }
    }
}

impl AsyncIoCallbackPack {
    fn new() -> Self {
        Self {
            jobs: DashMap::new(),
            pending: DashMap::new(),
            canceled: Mutex::new(IndexMap::with_capacity(CANCELED_CAP)),
        }
    }
    fn add_pending(&self, job_id: JobID, status: Result<usize>) {
        if let Some(mut pending_status) = self.pending.get_mut(&job_id) {
            let pending_status = pending_status.value_mut();
            let _ = match status {
                Ok(count) => pending_status.status.as_mut().map(|x| x.add_assign(count)),
                Err(e) => {
                    pending_status.status = Err(e);
                    Ok(())
                }
            };
        } else {
            self.pending.insert(job_id, JobPending { status });
        }
        // try merge in case of race, which happens when job added after callback called but before add_pending
        self.try_merge_pending(job_id);
    }
    fn add_job(&self, job_id: JobID, sender: Sender<Result<usize>>, waker: &Arc<Mutex<Waker>>) {
        if self
            .canceled
            .lock()
            .unwrap()
            .shift_remove(&job_id)
            .is_some()
        {
            log::trace!("cleared canceled async job: {}", job_id.0);
        }
        self.jobs.insert(
            job_id,
            JobEntry {
                sender,
                waker: Arc::downgrade(waker),
            },
        );
        self.try_merge_pending(job_id);
    }
    fn remove_job(&self, job_id: JobID) {
        self.jobs.remove(&job_id);
        self.pending.remove(&job_id);
        let mut canceled = self.canceled.lock().unwrap();
        if canceled.insert(job_id, ()).is_some() {
            canceled.shift_remove(&job_id);
            canceled.insert(job_id, ());
        }
        while canceled.len() > CANCELED_CAP {
            let _ = canceled.shift_remove_index(0);
        }
    }
    fn call(&mut self, _instr: &Instrument, event: &event::Event) -> vs::ViStatus {
        log::trace!("calling user data method");

        debug_assert_eq!(
            attribute::AttrEventType::get_from(event)
                .expect("get event type")
                .into_inner(),
            event::EventKind::EventIoCompletion as vs::ViEvent,
        );
        let status = match attribute::AttrStatus::get_from(event) {
            Ok(status) => status,
            Err(e) => {
                log::error!("error checking status in async io callback: {}", e);
                return vs::VI_SUCCESS as _;
            }
        };
        let job_id = match attribute::AttrJobId::get_from(event) {
            Ok(id) => JobID(id.into_inner()),
            Err(e) => {
                log::error!("error checking job id in async io callback: {}", e);
                return vs::VI_SUCCESS as _;
            }
        };
        if self.canceled.lock().unwrap().contains_key(&job_id) {
            log::trace!("ignoring canceled async job: {}", job_id.0);
            return vs::VI_SUCCESS_NCHAIN as _;
        }
        let result = CompletionCode::try_from(status).map_err(crate::Error::from);

        let waker = match result {
            ret @ (Ok(
                CompletionCode::Success
                | CompletionCode::SuccessSync
                | CompletionCode::SuccessMaxCnt
                | CompletionCode::SuccessTermChar
                | CompletionCode::SuccessQueueEmpty
                | CompletionCode::SuccessQueueNempty
                | CompletionCode::WarnQueueOverflow,
            )
            | Err(_)) => {
                if ret == Ok(CompletionCode::WarnQueueOverflow) {
                    log::warn!("warning: queue overflow in async io");
                }
                let ret = ret
                    .and_then(|_| attribute::AttrRetCount::get_from(event))
                    .map(|x| x.into_inner() as _);
                if let Err(ref e) = ret {
                    log::error!("async io completion error: job_id={}, err={}", job_id.0, e);
                }
                if let Some((_, job)) = self.jobs.remove(&job_id) {
                    if let Err(e) = job.sender.send(ret) {
                        log::warn!("error sending job result: {}", e);
                    }
                    Some(job.waker.clone())
                } else {
                    // add to pendings
                    self.add_pending(job_id, ret);
                    None
                }
            }
            Ok(other) => {
                log::warn!("unexpected completion code for async io: {}", other);
                None
            }
        };
        log::trace!("sended results");
        if let Some(waker) = waker {
            if let Some(waker) = waker.upgrade() {
                waker.lock().unwrap().wake_by_ref();
                log::trace!("waked from job");
            } else {
                log::debug!("waker already dropped");
            }
        }
        vs::VI_SUCCESS_NCHAIN as _
        //Normally, an application should always return VI_SUCCESS from all callback handlers. If a specific handler does not want other handlers to be invoked for the given event for the given session, it should return VI_SUCCESS_NCHAIN. No return value from a handler on one session will affect callbacks on other sessions. Future versions of VISA (or specific implementations of VISA) may take actions based on other return values, so a user should return VI_SUCCESS from handlers unless there is a specific reason to do otherwise.
    }
    unsafe extern "system" fn call_in_c(
        instr: vs::ViSession,
        event_type: vs::ViEventType,
        event: vs::ViEvent,
        user_data: *mut std::ffi::c_void,
    ) -> vs::ViStatus {
        log::trace!("calling in c");
        let pack: &mut Self = &mut *(user_data as *mut Self);
        let instr = Instrument::from_raw_ss(instr);
        let event = event::Event::new(event, event_type);
        let ret = pack.call(&instr, &event);
        std::mem::forget(event); // The VISA system automatically invokes the viClose() operation on the event context when a user handler returns. Because the event context must still be valid after the user handler returns (so that VISA can free it up), an application should not invoke the viClose() operation on an event context passed to a user handler.
        std::mem::forget(instr); // ? no sure yet, in official example session not closed
        ret
    }
}

pub(crate) struct AsyncId {
    pub(crate) rec: Receiver<Result<usize>>,
    pub(crate) waker: Arc<Mutex<Waker>>,
    pub(crate) job_id: JobID,
}

pub struct AsyncRead<'a> {
    ss: &'a AsyncInstrument,
    buf: &'a mut [u8],
    id: Option<AsyncId>,
}

impl<'a> AsyncRead<'a> {
    pub(crate) fn new(ss: &'a AsyncInstrument, buf: &'a mut [u8]) -> Self {
        AsyncRead { ss, buf, id: None }
    }
}

fn get_or_try_init_id<'a>(
    id: &'a mut Option<AsyncId>,
    ss: &AsyncInstrument,
    cx: &mut std::task::Context<'_>,
    f: impl FnOnce() -> Result<JobID>,
) -> Result<&'a mut AsyncId> {
    if id.is_none() {
        log::trace!("initializing async read");
        let (sender, rec) = std::sync::mpsc::channel();
        let waker = Arc::new(Mutex::new(cx.waker().clone()));
        let job_id = f()?;
        ss.callback.as_ref().add_job(job_id, sender, &waker);
        log::trace!("initialized");
        *id = Some(AsyncId { rec, waker, job_id });
    }
    Ok(id.as_mut().expect("just initialized"))
}

impl<'a> Future for AsyncRead<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        log::trace!("polling async read");
        let id = get_or_try_init_id(&mut self_mut.id, self_mut.ss, cx, || unsafe {
            self_mut.ss.instr.visa_read_async(self_mut.buf)
        })?;
        log::trace!("polling async read loop");
        match id.rec.try_recv() {
            Ok(o) => {
                log::trace!("results returned, future ready");
                self_mut.id = None;
                Poll::Ready(o)
            }
            Err(TryRecvError::Empty) => {
                log::trace!("empty results, future pending");
                let mut old_waker = id.waker.lock().unwrap();
                if !old_waker.will_wake(cx.waker()) {
                    old_waker.clone_from(cx.waker());
                }
                Poll::Pending
            }
            Err(TryRecvError::Disconnected) => {
                self_mut.id = None;
                Poll::Ready(Err(ErrorCode::ErrorConnLost.into()))
            }
        }
    }
}

impl<'a> Drop for AsyncRead<'a> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            self.ss.callback.as_ref().remove_job(id.job_id);
            if let Err(e) = wrap_raw_error_in_unsafe!(vs::viTerminate(
                self.ss.instr.as_raw_ss(),
                vs::VI_NULL as _,
                id.job_id.0
            )) {
                log::warn!("terminating unfinished async read: {}", e)
            };
        }
    }
}

pub struct AsyncWrite<'a> {
    ss: &'a AsyncInstrument,
    buf: &'a [u8],
    id: Option<AsyncId>,
}

impl<'a> AsyncWrite<'a> {
    pub(crate) fn new(ss: &'a AsyncInstrument, buf: &'a [u8]) -> Self {
        Self { ss, buf, id: None }
    }
}

impl<'a> Future for AsyncWrite<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        log::trace!("polling async write");
        let id = get_or_try_init_id(&mut self_mut.id, self_mut.ss, cx, || unsafe {
            self_mut.ss.instr.visa_write_async(self_mut.buf)
        })?;
        match id.rec.try_recv() {
            Ok(o) => {
                log::trace!("results returned");
                self_mut.id = None;
                Poll::Ready(o)
            }
            Err(TryRecvError::Empty) => {
                log::trace!("empty results, future pending");
                let mut old_waker = id.waker.lock().unwrap();
                if !old_waker.will_wake(cx.waker()) {
                    old_waker.clone_from(cx.waker());
                }
                Poll::Pending
            }
            Err(TryRecvError::Disconnected) => {
                self_mut.id = None;
                Poll::Ready(Err(ErrorCode::ErrorConnLost.into()))
            }
        }
    }
}

impl<'a> Drop for AsyncWrite<'a> {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            self.ss.callback.as_ref().remove_job(id.job_id);
            if let Err(e) = wrap_raw_error_in_unsafe!(vs::viTerminate(
                self.ss.instr.as_raw_ss(),
                vs::VI_NULL as _,
                id.job_id.0
            )) {
                log::warn!("terminating unfinished async write: {}", e)
            };
        }
    }
}
