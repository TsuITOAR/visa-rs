use crate::{async_io::AsyncInstrument, Error, Instrument, JobID};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// Adapts a VISA instrument to Tokio's [`AsyncRead`] and [`AsyncWrite`] traits.
///
/// `write().await` and `write_all().await` start VISA transfers as bytes are accepted;
/// they do not wait for completion or automatically flush. `flush().await` waits for
/// completion and reports write errors; it is not needed to start the transfer.
/// Flush before reading a response or dropping/converting the adapter.
///
/// Dropping a Tokio read, write, or flush future does not cancel an already started
/// transfer. Dropping or converting the adapter requests cancellation of pending I/O.
pub struct InstrumentTokioAdapter {
    // VISA retains buffer pointers until completion; never lend it the caller's buffer.
    instr: AsyncInstrument,
    read_current: Option<JobID>,
    write_current: Option<JobID>,
    // Undelivered bytes and cursor. Recycle the allocation once drained.
    read_buf: Vec<u8>,
    read_pos: usize,
    // Recycled allocation for the next write.
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

    // Deliver what fits, retaining the remainder for the next poll_read.
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

    fn poll_current_write(&mut self, cx: &Context<'_>) -> Poll<io::Result<()>> {
        let Some(job_id) = self.write_current else {
            return Poll::Ready(Ok(()));
        };
        match self.instr.poll_job(job_id, cx) {
            Poll::Ready((result, mut buf)) => {
                self.write_current = None;
                let expected = buf.len();
                buf.clear();
                self.write_buf = buf;
                let completion = result.map_err(|e| {
                    log::error!("tokio async write completion error: {}", e);
                    Self::map_vs_err(e)
                })?;
                // These bytes have already been acknowledged to the caller. A short
                // completion must not silently discard the untransferred suffix.
                if completion.count != expected {
                    let kind = if completion.count == 0 {
                        io::ErrorKind::WriteZero
                    } else if completion.count > expected {
                        io::ErrorKind::InvalidData
                    } else {
                        io::ErrorKind::Other
                    };
                    return Poll::Ready(Err(io::Error::new(
                        kind,
                        format!(
                            "VISA wrote {} of {expected} accepted bytes",
                            completion.count
                        ),
                    )));
                }
                Poll::Ready(Ok(()))
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
            owned.resize(
                crate::instrument::async_transfer_count(buf.remaining()) as usize,
                0,
            );
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
    /// Starts writing accepted bytes. While the previous transfer is pending,
    /// no bytes from this call are accepted. Completion errors are deferred until
    /// the next write or flush.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let me = self.get_mut();
        match me.poll_current_write(cx) {
            Poll::Pending => {
                if !buf.is_empty() {
                    log::warn!(
                        "tokio write waiting for previous VISA job; {} input bytes not accepted yet",
                        buf.len()
                    );
                }
                return Poll::Pending;
            }
            Poll::Ready(result) => result?,
        }
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        // VISA's count is 32-bit. A larger slice is accepted in multiple writes.
        let n = crate::instrument::async_transfer_count(buf.len()) as usize;
        let mut owned = std::mem::take(&mut me.write_buf);
        owned.clear();
        owned.extend_from_slice(&buf[..n]);
        match me.instr.start_write_id(owned, cx.waker()) {
            Ok(job_id) => me.write_current = Some(job_id),
            Err(e) => return Poll::Ready(Err(Self::map_vs_err(e))),
        }
        Poll::Ready(Ok(n))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use crate::flags::FlushMode;
        let me = self.get_mut();
        // A queued write has to reach the device before flushing means anything.
        std::task::ready!(me.poll_current_write(cx))?;
        me.instr
            .instrument()
            .visa_flush(FlushMode::WRITE_BUF | FlushMode::IO_OUT_BUF)
            .map_err(Self::map_vs_err)?;
        Poll::Ready(Ok(()))
    }

    /// Flushes pending output without closing the session.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
