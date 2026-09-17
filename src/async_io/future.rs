//! The `Future` types driving one logical transfer each.

use super::{note_sync_completion, AsyncInstrument, Completed};
use crate::instrument::async_transfer_count;
use crate::{
    enums::status::{CompletionCode, ErrorCode},
    JobID, Result,
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

// Response-read chunk size: a memory/round-trip tradeoff, not a response length limit.
const READ_CHUNK: usize = 4096;

// Append a completed transfer and decide whether the logical read is finished.
fn collect_chunk(
    out: &mut Vec<u8>,
    limit: Option<usize>,
    Completed { count, completion }: Completed,
    mut buf: Vec<u8>,
) -> Result<bool> {
    let count = count.min(buf.len());
    if out.is_empty() {
        buf.truncate(count);
        *out = buf;
    } else {
        out.extend_from_slice(&buf[..count]);
    }
    if let Some(limit) = limit {
        if out.len() == limit {
            return Ok(true);
        }
        // A successful zero-byte completion cannot advance an exact read. Fail
        // instead of repeatedly submitting reads that might complete inline.
        if count == 0 {
            return Err(ErrorCode::ErrorIo.into());
        }
        return Ok(false);
    }
    // Only response-oriented reads stop at END or the termination character.
    Ok(completion != CompletionCode::SuccessMaxCnt)
}

/// Future returned by [`AsyncInstrument::async_read`] and
/// [`AsyncInstrument::async_read_exact`].
///
/// Dropping it before completion requests cancellation.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AsyncRead<'a> {
    ss: &'a AsyncInstrument,
    // None: read until the device ends the response. Some(n): read exactly n bytes.
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
            // Request the remaining length up to VISA's count limit; do not split
            // known-length reads into the smaller response-oriented READ_CHUNK.
            Some(n) => async_transfer_count(n.saturating_sub(self.out.len())) as usize,
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
                        me.ss.callback().register(job_id, cx.waker(), buf);
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
                Poll::Ready((Ok(completed), buf)) => {
                    me.id = None;
                    match collect_chunk(&mut me.out, me.limit, completed, buf) {
                        Ok(true) => return me.finish(),
                        Ok(false) => {}
                        Err(e) => {
                            me.done = true;
                            return Poll::Ready(Err(e));
                        }
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

/// Future returned by [`AsyncInstrument::async_write`].
///
/// Dropping it before completion requests cancellation.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AsyncWrite<'a> {
    ss: &'a AsyncInstrument,
    buf: Option<Vec<u8>>,
    id: Option<JobID>,
    len: usize,
    written: usize,
    done: bool,
}

impl<'a> AsyncWrite<'a> {
    pub(super) fn new(ss: &'a AsyncInstrument, buf: Vec<u8>) -> Self {
        Self {
            ss,
            len: buf.len(),
            written: 0,
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
        loop {
            if me.written == me.len {
                me.done = true;
                return Poll::Ready(Ok(me.written));
            }
            if me.id.is_none() {
                let buf = me
                    .buf
                    .take()
                    .expect("buffer held until the transfer starts");
                // Keep the original allocation alive; each job reads the next slice.
                match unsafe { me.ss.instr.visa_write_async(&buf[me.written..]) } {
                    Ok((job_id, completion)) => {
                        me.ss.callback().register(job_id, cx.waker(), buf);
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
            let (result, buf) = std::task::ready!(me.ss.poll_job(job_id, cx));
            me.id = None;
            match result.and_then(|c| advance_write(&mut me.written, me.len, c.count)) {
                Ok(true) => {
                    me.done = true;
                    return Poll::Ready(Ok(me.written));
                }
                Ok(false) => me.buf = Some(buf),
                Err(e) => {
                    me.done = true;
                    return Poll::Ready(Err(e));
                }
            }
        }
    }
}

// Continue only after a full chunk. Preserve the API's short-write byte count.
fn advance_write(written: &mut usize, len: usize, count: usize) -> Result<bool> {
    let requested = async_transfer_count(len - *written) as usize;
    if count > requested {
        return Err(ErrorCode::ErrorIo.into());
    }
    *written += count;
    Ok(count < requested || *written == len)
}

impl Drop for AsyncWrite<'_> {
    fn drop(&mut self) {
        if let Some(job_id) = self.id.take() {
            self.ss.cancel_job(job_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_counts_fit_visa() {
        for len in [0, 1, 4096, u32::MAX as usize] {
            assert_eq!(async_transfer_count(len) as usize, len);
        }
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn oversized_write_advances_without_wrapping() {
        let max = u32::MAX as usize;
        assert_eq!(async_transfer_count(max + 1) as usize, max);
        assert_eq!(async_transfer_count(usize::MAX) as usize, max);
        let len = 2 * max + 17;
        let mut written = 0;
        assert!(!advance_write(&mut written, len, max).unwrap());
        assert_eq!(written, max);
        assert!(!advance_write(&mut written, len, max).unwrap());
        assert_eq!(written, 2 * max);
        assert!(advance_write(&mut written, len, 17).unwrap());
        assert_eq!(written, len);

        // Exactly 4 GiB is MAX + one, not a zero-byte request.
        written = 0;
        assert!(!advance_write(&mut written, max + 1, max).unwrap());
        assert!(advance_write(&mut written, max + 1, 1).unwrap());
        assert_eq!(written, max + 1);

        // A short final chunk reports the cumulative count, not just that chunk.
        written = max;
        assert!(advance_write(&mut written, max + 17, 3).unwrap());
        assert_eq!(written, max + 3);
    }

    #[test]
    fn write_progress_preserves_short_counts_and_rejects_overruns() {
        let mut written = 0;
        assert!(advance_write(&mut written, 8, 3).unwrap());
        assert_eq!(written, 3);
        assert!(advance_write(&mut written, 8, 0).unwrap());
        assert_eq!(written, 3);
        assert_eq!(
            advance_write(&mut written, 8, 6),
            Err(ErrorCode::ErrorIo.into())
        );
        assert_eq!(written, 3);
    }

    #[test]
    fn exact_read_continues_after_short_end() {
        let mut out = Vec::new();
        let first = Completed {
            count: 3,
            completion: CompletionCode::Success,
        };
        assert!(!collect_chunk(&mut out, Some(8), first, b"abc\0\0\0\0\0".to_vec()).unwrap());
        assert_eq!(out, b"abc");
        let last = Completed {
            count: 5,
            completion: CompletionCode::SuccessMaxCnt,
        };
        assert!(collect_chunk(&mut out, Some(8), last, b"defgh".to_vec()).unwrap());
        assert_eq!(out, b"abcdefgh");
    }

    #[test]
    fn exact_read_finishes_at_count_regardless_of_completion_code() {
        for completion in [
            CompletionCode::Success,
            CompletionCode::SuccessMaxCnt,
            CompletionCode::SuccessTermChar,
        ] {
            let mut out = Vec::new();
            assert!(collect_chunk(
                &mut out,
                Some(3),
                Completed {
                    count: 3,
                    completion
                },
                b"abc".to_vec()
            )
            .unwrap());
            assert_eq!(out, b"abc");
        }
    }

    #[test]
    fn exact_read_errors_on_zero_progress() {
        for mut out in [Vec::new(), b"abc".to_vec()] {
            let completed = Completed {
                count: 0,
                completion: CompletionCode::Success,
            };
            assert_eq!(
                collect_chunk(&mut out, Some(8), completed, vec![0; 8]),
                Err(ErrorCode::ErrorIo.into())
            );
        }
    }

    #[test]
    fn response_read_still_finishes_at_end() {
        let mut out = Vec::new();
        let completed = Completed {
            count: 3,
            completion: CompletionCode::Success,
        };
        assert!(collect_chunk(&mut out, None, completed, b"abc\0\0".to_vec()).unwrap());
        assert_eq!(out, b"abc");
    }

    #[test]
    fn response_read_continues_after_full_chunk() {
        let mut out = Vec::new();
        let completed = Completed {
            count: 3,
            completion: CompletionCode::SuccessMaxCnt,
        };
        assert!(!collect_chunk(&mut out, None, completed, b"abc".to_vec()).unwrap());
        assert_eq!(out, b"abc");
    }
}
