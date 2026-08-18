use crate::{async_io::AsyncInstrument, Error, Instrument, JobID};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Adapts a VISA session to tokio's IO traits.
///
/// Visa transfers own their buffer until the completion event arrives, so this holds its
/// own buffers rather than lending out the caller's. A read that returns more bytes than
/// the caller's `ReadBuf` can take keeps the remainder for the next call.
pub struct InstrumentTokioAdapter {
    instr: AsyncInstrument,
    read_current: Option<JobID>,
    write_current: Option<JobID>,
    /// Bytes read from the device but not yet handed to the caller, and how far into
    /// them we have got. When drained, the allocation is recycled for the next read.
    read_buf: Vec<u8>,
    read_pos: usize,
    /// Recycled allocation for the next write.
    write_buf: Vec<u8>,
}

impl TryFrom<Instrument> for InstrumentTokioAdapter {
    type Error = Error;
    fn try_from(value: Instrument) -> Result<Self, Self::Error> {
        Ok(Self::new(AsyncInstrument::new(value)?))
    }
}

impl From<AsyncInstrument> for InstrumentTokioAdapter {
    fn from(value: AsyncInstrument) -> Self {
        Self::new(value)
    }
}

impl From<InstrumentTokioAdapter> for AsyncInstrument {
    fn from(mut value: InstrumentTokioAdapter) -> Self {
        value.cancel_in_flight();
        let value = std::mem::ManuallyDrop::new(value);
        // SAFETY: `value` is never dropped, so reading each field out of it moves it
        // exactly once. Both jobs are already cancelled, which is all `Drop` does.
        let instr = unsafe { std::ptr::read(&value.instr) };
        drop(unsafe { std::ptr::read(&value.read_buf) });
        drop(unsafe { std::ptr::read(&value.write_buf) });
        instr
    }
}

impl From<InstrumentTokioAdapter> for Instrument {
    fn from(value: InstrumentTokioAdapter) -> Self {
        let async_instr: AsyncInstrument = value.into();
        async_instr.into()
    }
}

impl InstrumentTokioAdapter {
    pub fn new(instr: AsyncInstrument) -> Self {
        Self {
            instr,
            read_current: None,
            write_current: None,
            read_buf: Vec::new(),
            read_pos: 0,
            write_buf: Vec::new(),
        }
    }

    fn map_vs_err(err: Error) -> io::Error {
        io::Error::other(err)
    }

    fn cancel_in_flight(&mut self) {
        if let Some(job_id) = self.read_current.take() {
            self.instr.cancel_job(job_id);
        }
        if let Some(job_id) = self.write_current.take() {
            self.instr.cancel_job(job_id);
        }
    }

    /// Moves as much of `read_buf` into `buf` as fits, keeping any remainder for the
    /// next `poll_read`. Never writes past `buf.remaining()`.
    fn deliver(&mut self, buf: &mut ReadBuf<'_>) {
        let available = &self.read_buf[self.read_pos..];
        let n = available.len().min(buf.remaining());
        buf.put_slice(&available[..n]);
        self.read_pos += n;
        if self.read_pos == self.read_buf.len() {
            self.read_buf.clear();
            self.read_pos = 0;
        }
    }

    fn poll_current_write(&mut self, cx: &Context<'_>) -> Poll<io::Result<usize>> {
        let Some(job_id) = self.write_current else {
            return Poll::Ready(Ok(0));
        };
        match self.instr.poll_job(job_id, cx) {
            Poll::Ready((result, mut buf)) => {
                self.write_current = None;
                buf.clear();
                self.write_buf = buf;
                Poll::Ready(result.map(|c| c.count).map_err(|e| {
                    log::error!("tokio async write completion error: {}", e);
                    Self::map_vs_err(e)
                }))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for InstrumentTokioAdapter {
    fn drop(&mut self) {
        self.cancel_in_flight();
    }
}

impl AsyncRead for InstrumentTokioAdapter {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if me.read_current.is_none() {
            // Leftovers from a previous transfer come first.
            if me.read_pos < me.read_buf.len() {
                me.deliver(buf);
                return Poll::Ready(Ok(()));
            }
            let mut owned = std::mem::take(&mut me.read_buf);
            owned.clear();
            owned.resize(buf.remaining(), 0);
            me.read_pos = 0;
            match me.instr.start_read_id(owned, cx.waker()) {
                Ok(job_id) => me.read_current = Some(job_id),
                Err(e) => return Poll::Ready(Err(Self::map_vs_err(e))),
            }
        }
        let job_id = me.read_current.expect("set immediately above");
        match me.instr.poll_job(job_id, cx) {
            Poll::Ready((Ok(c), mut owned)) => {
                me.read_current = None;
                owned.truncate(c.count.min(owned.len()));
                me.read_buf = owned;
                me.read_pos = 0;
                me.deliver(buf);
                Poll::Ready(Ok(()))
            }
            Poll::Ready((Err(e), mut owned)) => {
                me.read_current = None;
                owned.clear();
                me.read_buf = owned;
                me.read_pos = 0;
                log::error!("tokio async read completion error: {}", e);
                Poll::Ready(Err(Self::map_vs_err(e)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl AsyncWrite for InstrumentTokioAdapter {
    /// Note that while a write is in flight this reports on *that* transfer; the
    /// contract allows `buf` to differ between polls, so poll to completion before
    /// changing what you are writing.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();
        if me.write_current.is_none() {
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            let mut owned = std::mem::take(&mut me.write_buf);
            owned.clear();
            owned.extend_from_slice(buf);
            match me.instr.start_write_id(owned, cx.waker()) {
                Ok(job_id) => me.write_current = Some(job_id),
                Err(e) => return Poll::Ready(Err(Self::map_vs_err(e))),
            }
        }
        me.poll_current_write(cx)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use crate::flags::FlushMode;
        let me = self.get_mut();
        // A queued write has to reach the device before flushing means anything.
        if me.write_current.is_some() {
            match me.poll_current_write(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(_)) => {}
            }
        }
        me.instr
            .instrument()
            .visa_flush(FlushMode::WRITE_BUF | FlushMode::IO_OUT_BUF)
            .map_err(Self::map_vs_err)?;
        Poll::Ready(Ok(()))
    }

    /// Visa has no half-close, so this is a flush: pending output reaches the device,
    /// and the session itself closes when the adapter is dropped.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
