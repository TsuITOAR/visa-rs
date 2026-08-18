//! The `Future` types driving one logical transfer each.

use super::{note_sync_completion, AsyncInstrument, Completed};
use crate::{
    enums::status::{CompletionCode, ErrorCode},
    JobID, Result,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

/// How much is asked of visa in a single `viReadAsync`. A longer response is assembled
/// from several chunks, so this is a memory/round-trip tradeoff, not a limit.
const READ_CHUNK: usize = 4096;

/// Future returned by [`AsyncInstrument::async_read`] and
/// [`AsyncInstrument::async_read_exact`].
///
/// Dropping it before completion terminates the transfer, but its buffer is not freed:
/// visa may still be writing into it, so it stays owned by the session until the
/// completion event arrives. Cancelling therefore costs a buffer, never memory safety.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AsyncRead<'a> {
    ss: &'a AsyncInstrument,
    /// `None`: read until the device ends the response. `Some(n)`: read exactly `n`.
    limit: Option<usize>,
    timeout: std::time::Duration,
    out: Vec<u8>,
    id: Option<JobID>,
    prepared: bool,
    done: bool,
}

impl<'a> AsyncRead<'a> {
    pub(super) fn new(
        ss: &'a AsyncInstrument,
        limit: Option<usize>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            ss,
            limit,
            timeout,
            out: Vec::new(),
            id: None,
            prepared: false,
            done: false,
        }
    }

    fn chunk_len(&self) -> usize {
        match self.limit {
            // The length is known, so ask for all of it at once. Capping this at
            // `READ_CHUNK` would turn one large binary transfer into thousands of
            // round trips, which is precisely what `async_read_exact` exists to avoid.
            Some(n) => n.saturating_sub(self.out.len()),
            // Unknown length: a fixed chunk, looped until the device ends the response.
            None => READ_CHUNK,
        }
    }

    fn finish(&mut self) -> Poll<Result<Vec<u8>>> {
        self.done = true;
        Poll::Ready(Ok(std::mem::take(&mut self.out)))
    }
}

impl Future for AsyncRead<'_> {
    type Output = Result<Vec<u8>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        if me.done {
            // Already resolved; return rather than starting another transfer.
            return Poll::Ready(Err(ErrorCode::ErrorInvJobId.into()));
        }
        if !me.prepared {
            // A fixed-length read must not stop early on a termination character, or
            // binary block data would be cut at the first byte that happens to be one.
            if let Err(e) = me.ss.prepare_read(me.timeout, me.limit.is_none()) {
                me.done = true;
                return Poll::Ready(Err(e));
            }
            me.prepared = true;
        }
        loop {
            if me.limit == Some(me.out.len()) {
                return me.finish();
            }
            if me.id.is_none() {
                let mut buf = vec![0u8; me.chunk_len()];
                match unsafe { me.ss.instr.visa_read_async(&mut buf) } {
                    Ok((job_id, completion)) => {
                        // The buffer moves into the pack, which keeps it alive for as
                        // long as visa might write to it.
                        me.ss.callback.register(job_id, cx.waker(), buf);
                        note_sync_completion(completion, job_id);
                        me.id = Some(job_id);
                    }
                    Err(e) => {
                        // Nothing was queued, so visa never saw the buffer.
                        me.done = true;
                        return Poll::Ready(Err(e));
                    }
                }
            }
            let job_id = me.id.expect("set immediately above");
            match me.ss.poll_job(job_id, cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready((Err(e), _)) => {
                    me.id = None;
                    me.done = true;
                    return Poll::Ready(Err(e));
                }
                Poll::Ready((Ok(Completed { count, completion }), mut buf)) => {
                    me.id = None;
                    let count = count.min(buf.len());
                    if me.out.is_empty() {
                        // First chunk, and usually the only one: take the buffer rather
                        // than copying out of it.
                        buf.truncate(count);
                        me.out = buf;
                    } else {
                        me.out.extend_from_slice(&buf[..count]);
                    }
                    // `SuccessMaxCnt` means the chunk filled, so more may follow.
                    // Anything else means the device ended the response.
                    if completion != CompletionCode::SuccessMaxCnt {
                        return me.finish();
                    }
                }
            }
        }
    }
}

impl Drop for AsyncRead<'_> {
    fn drop(&mut self) {
        if let Some(job_id) = self.id.take() {
            self.ss.cancel_job(job_id);
        }
    }
}

/// Future returned by [`AsyncInstrument::async_write`]. See [`AsyncRead`] for what
/// dropping one before completion costs.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AsyncWrite<'a> {
    ss: &'a AsyncInstrument,
    buf: Option<Vec<u8>>,
    id: Option<JobID>,
    done: bool,
}

impl<'a> AsyncWrite<'a> {
    pub(super) fn new(ss: &'a AsyncInstrument, buf: Vec<u8>) -> Self {
        Self {
            ss,
            buf: Some(buf),
            id: None,
            done: false,
        }
    }
}

impl Future for AsyncWrite<'_> {
    type Output = Result<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = self.get_mut();
        if me.done {
            return Poll::Ready(Err(ErrorCode::ErrorInvJobId.into()));
        }
        if me.id.is_none() {
            let buf = me
                .buf
                .take()
                .expect("buffer held until the transfer starts");
            match unsafe { me.ss.instr.visa_write_async(&buf) } {
                Ok((job_id, completion)) => {
                    me.ss.callback.register(job_id, cx.waker(), buf);
                    note_sync_completion(completion, job_id);
                    me.id = Some(job_id);
                }
                Err(e) => {
                    me.done = true;
                    return Poll::Ready(Err(e));
                }
            }
        }
        let job_id = me.id.expect("set immediately above");
        match me.ss.poll_job(job_id, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready((result, _buf)) => {
                me.id = None;
                me.done = true;
                Poll::Ready(result.map(|c| c.count))
            }
        }
    }
}

impl Drop for AsyncWrite<'_> {
    fn drop(&mut self) {
        if let Some(job_id) = self.id.take() {
            self.ss.cancel_job(job_id);
        }
    }
}
