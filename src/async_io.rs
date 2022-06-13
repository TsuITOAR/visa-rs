use crate::{
    enums::{AttrKind, HasAttribute},
    event::{self, *},
    session::{AsRawSs, AsSs, BorrowedSs, FromRawSs},
    JobID,
};
use std::{
    future::Future,
    ptr::NonNull,
    sync::{
        mpsc::{Receiver, Sender, TryRecvError},
        Arc, Mutex, Weak,
    },
    task::{Poll, Waker},
};
use visa_sys as vs;

use super::{Instrument, Result};

fn terminate_async(ss: BorrowedSs<'_>, job_id: JobID) -> Result<()> {
    wrap_raw_error_in_unsafe!(vs::viTerminate(ss.as_raw_ss(), vs::VI_NULL as _, job_id.0))?;
    Ok(())
}

struct AsyncIoHandler<'b> {
    instr: BorrowedSs<'b>,
    job_id: JobID,
    rec: Receiver<Result<usize>>,
    waker: Arc<Mutex<Waker>>,
    callback: NonNull<AsyncIoCallbackPack>,
}

impl<'b> AsyncIoHandler<'b> {
    fn new(instr: BorrowedSs<'b>, job_id: JobID, waker: Arc<Mutex<Waker>>) -> Result<Self> {
        let (callback, rec) = AsyncIoCallbackPack::new(Arc::downgrade(&waker), job_id);
        let callback = NonNull::new(Box::into_raw(Box::new(callback))).unwrap();
        super::wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            instr.as_raw_ss(),
            EventKind::IoCompletion as _,
            Some(AsyncIoCallbackPack::call_in_c),
            callback.as_ptr() as _
        ))?;
        Ok(Self {
            instr,
            job_id,
            rec,
            waker,
            callback,
        })
    }
    fn update_waker(&self, waker: &Waker) {
        let mut old_waker = self.waker.lock().unwrap();
        if !self.waker.lock().unwrap().will_wake(waker) {
            *old_waker = waker.clone();
        }
    }
}

impl<'b> Drop for AsyncIoHandler<'b> {
    fn drop(&mut self) {
        #[allow(unused_unsafe)]
        unsafe {
            if let Err(e) = terminate_async(self.instr, self.job_id) {
                log::warn!("terminating async io: {}", e)
            };
            if let Err(e) = wrap_raw_error_in_unsafe!(vs::viUninstallHandler(
                self.instr.as_raw_ss(),
                EventKind::IoCompletion as _,
                Some(AsyncIoCallbackPack::call_in_c),
                self.callback.as_ptr() as _,
            )) {
                log::warn!("uninstalling handler: {}", e)
            };
            Box::from_raw(self.callback.as_ptr());
        }
    }
}

struct AsyncIoCallbackPack {
    sender: Sender<Result<usize>>,
    waker: Weak<Mutex<Waker>>,
    job_id: JobID,
}

impl AsyncIoCallbackPack {
    fn new(waker: Weak<Mutex<Waker>>, id: JobID) -> (Self, Receiver<Result<usize>>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        (
            Self {
                sender,
                waker,
                job_id: id,
            },
            receiver,
        )
    }
    fn call(&mut self, _instr: &Instrument, event: &event::Event) -> vs::ViStatus {
        fn check_job_id(s: &mut AsyncIoCallbackPack, event: &event::Event) -> Result<bool> {
            debug_assert_eq!(
                event.get_attr(AttrKind::AttrEventType)?.as_u64() as vs::ViEvent,
                EventKind::IoCompletion as vs::ViEvent,
            );
            let job_id = event.get_attr(AttrKind::AttrJobId)?.as_u64() as _;
            let job_id = JobID(job_id);
            Ok(job_id == s.job_id)
        }

        match check_job_id(self, event) {
            Ok(false) => return vs::VI_SUCCESS as _,
            Err(e) => log::error!("error checking job id in async io callback:\n {}", e),
            Ok(true) => (),
        }

        fn get_ret(event: &event::Event) -> Result<usize> {
            #[allow(unused_unsafe)]
            wrap_raw_error_in_unsafe!(event.get_attr(AttrKind::AttrStatus)?.as_u64() as i32)?;
            let ret: usize = event.get_attr(AttrKind::AttrRetCount)?.as_u64() as _;
            Ok(ret)
        }
        self.sender
            .send(get_ret(event))
            .expect("send result to channel");
        self.waker.upgrade().expect("as long as handler not dropped, upgrade is successful, only when this function will be called").lock().unwrap().wake_by_ref();
        vs::VI_SUCCESS_NCHAIN as _
        //Normally, an application should always return VI_SUCCESS from all callback handlers. If a specific handler does not want other handlers to be invoked for the given event for the given session, it should return VI_SUCCESS_NCHAIN. No return value from a handler on one session will affect callbacks on other sessions. Future versions of VISA (or specific implementations of VISA) may take actions based on other return values, so a user should return VI_SUCCESS from handlers unless there is a specific reason to do otherwise.
    }
    unsafe extern "C" fn call_in_c(
        instr: vs::ViSession,
        event_type: vs::ViEventType,
        event: vs::ViEvent,
        user_data: *mut std::ffi::c_void,
    ) -> vs::ViStatus {
        let pack: &mut Self = &mut *(user_data as *mut Self);
        let instr = Instrument::from_raw_ss(instr);
        let event = event::Event::new(event, event_type);
        let ret = pack.call(&instr, &event);
        std::mem::forget(event); // The VISA system automatically invokes the viClose() operation on the event context when a user handler returns. Because the event context must still be valid after the user handler returns (so that VISA can free it up), an application should not invoke the viClose() operation on an event context passed to a user handler.
        std::mem::forget(instr); // ? no sure yet, in official example session not closed
        ret
    }
}

pub struct AsyncRead<'a> {
    ss: &'a Instrument,
    handler: Option<AsyncIoHandler<'a>>,
    buf: &'a mut [u8],
}

impl<'a> AsyncRead<'a> {
    pub(crate) fn new(ss: &'a Instrument, buf: &'a mut [u8]) -> Self {
        Self {
            ss,
            buf,
            handler: None,
        }
    }
}

impl<'a> Future for AsyncRead<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        loop {
            match &mut self_mut.handler {
                a @ None => {
                    let handler = AsyncIoHandler::new(
                        self_mut.ss.as_ss(),
                        self_mut.ss.visa_read_async(self_mut.buf)?,
                        Arc::new(Mutex::new(cx.waker().clone())),
                    )?;
                    *a = Some(handler);
                }
                Some(ref mut b) => match b.rec.try_recv() {
                    Ok(o) => return Poll::Ready(o),
                    Err(TryRecvError::Empty) => {
                        b.update_waker(cx.waker());
                        return Poll::Pending;
                    }
                    Err(TryRecvError::Disconnected) => {
                        unreachable!("sender side should be valid as long as handler not dropped")
                    }
                },
            };
        }
    }
}

pub struct AsyncWrite<'a> {
    ss: &'a Instrument,
    handler: Option<AsyncIoHandler<'a>>,
    buf: &'a [u8],
}

impl<'a> AsyncWrite<'a> {
    pub(crate) fn new(ss: &'a Instrument, buf: &'a [u8]) -> Self {
        Self {
            ss,
            buf,
            handler: None,
        }
    }
}

impl<'a> Future for AsyncWrite<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        loop {
            match &mut self_mut.handler {
                a @ None => {
                    let handler = AsyncIoHandler::new(
                        self_mut.ss.as_ss(),
                        self_mut.ss.visa_write_async(self_mut.buf)?,
                        Arc::new(Mutex::new(cx.waker().clone())),
                    )?;
                    *a = Some(handler);
                }
                Some(ref mut b) => match b.rec.try_recv() {
                    Ok(o) => return Poll::Ready(o),
                    Err(TryRecvError::Empty) => {
                        b.update_waker(cx.waker());
                        return Poll::Pending;
                    }
                    Err(TryRecvError::Disconnected) => {
                        unreachable!("sender side should be valid as long as handler not dropped")
                    }
                },
            };
        }
    }
}
