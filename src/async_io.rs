use crate::{event::*, handler::Handler, session::AsRawSs, JobID};
use std::{
    future::Future,
    task::{Poll, Waker},
};
use visa_sys as vs;

use super::{Instrument, Result};

fn terminate_async(ss: &Instrument, job_id: JobID) -> Result<()> {
    wrap_raw_error_in_unsafe!(vs::viTerminate(ss.as_raw_ss(), vs::VI_NULL as _, job_id.0))?;
    Ok(())
}

fn assign_waker(job_id: JobID, waker: Waker) {
    todo!()
}

//static mut WAKER_MAP: HashMap<JobID, Waker> = ;

// if called multiple times, might re wake a future
// add a called history check, but might hit reused id
//  better should only be installed once, including input and output
fn call_back(s: &Instrument, t: &Event) -> Result<usize> {
    // check job id, result status, and call corresponding waker
    todo!()
}

pub struct AsyncRead<'a> {
    ss: &'a Instrument,
    job_id: Option<JobID>,
    handler: Option<Handler<'a, fn(&Instrument, &Event) -> Result<usize>>>,
    buf: &'a mut [u8],
}

impl<'a> Future for AsyncRead<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        loop {
            match (self_mut.handler.as_mut(), self_mut.job_id) {
                (None, None) => {
                    self_mut.ss.enable_event(
                        EventKind::IoCompletion,
                        Mechanism::Handler,
                        EventFilter::Null,
                    )?;
                    self_mut.handler.replace(
                        self_mut
                            .ss
                            .install_handler(EventKind::IoCompletion, call_back as _)?,
                    );
                    self_mut
                        .job_id
                        .replace(self_mut.ss.read_async(self_mut.buf)?);
                }
                (Some(h), Some(id)) => {
                    if let Ok(r) = h.receiver().try_recv() {
                        return Poll::Ready(r);
                    } else {
                        assign_waker(id, cx.waker().clone());
                        return Poll::Pending;
                    }
                }
                _ => unreachable!(),
            };
        }
    }
}

impl<'a> Drop for AsyncRead<'a> {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        if let Some(id) = self.job_id {
            #[allow(unused_must_use)]
            terminate_async(self.ss, id);
        }
    }
}

pub struct AsyncWrite<'a> {
    ss: &'a Instrument,
    job_id: Option<JobID>,
    handler: Option<Handler<'a, fn(&Instrument, &Event) -> Result<usize>>>,
    buf: &'a [u8],
}

impl<'a> Future for AsyncWrite<'a> {
    type Output = Result<usize>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let self_mut = self.get_mut();
        loop {
            match (self_mut.handler.as_mut(), self_mut.job_id) {
                (None, None) => {
                    self_mut.ss.enable_event(
                        EventKind::IoCompletion,
                        Mechanism::Handler,
                        EventFilter::Null,
                    )?;
                    self_mut.handler.replace(
                        self_mut
                            .ss
                            .install_handler(EventKind::IoCompletion, call_back as _)?,
                    );
                    self_mut
                        .job_id
                        .replace(self_mut.ss.write_async(self_mut.buf)?);
                }
                (Some(h), Some(id)) => {
                    if let Ok(r) = h.receiver().try_recv() {
                        return Poll::Ready(r);
                    } else {
                        assign_waker(id, cx.waker().clone());
                        return Poll::Pending;
                    }
                }
                _ => unreachable!(),
            };
        }
    }
}

impl<'a> Drop for AsyncWrite<'a> {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        if let Some(id) = self.job_id {
            #[allow(unused_must_use)]
            terminate_async(self.ss, id);
        }
    }
}
