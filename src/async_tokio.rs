use crate::{
    async_io::{AsyncId, AsyncInstrument},
    enums::status::ErrorCode,
    Error, Instrument,
};
use bytes::BytesMut;
use std::{
    io,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub struct InstrumentTokioAdapter {
    instr: AsyncInstrument,
    read_current: Option<AsyncId>,
    write_current: Option<AsyncId>,
    read_buf: BytesMut,
    write_buf: BytesMut,
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
        if let Some(id) = value.read_current.take() {
            value.instr.cancel_job(id.job_id);
        }
        if let Some(id) = value.write_current.take() {
            value.instr.cancel_job(id.job_id);
        }
        let value = std::mem::ManuallyDrop::new(value);
        // SAFETY: We intentionally prevent drop of `value` and take ownership of `instr`.
        // `instr` is not used afterward, and `value` is never dropped.
        unsafe { std::ptr::read(&value.instr) }
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
            read_buf: BytesMut::new(),
            write_buf: BytesMut::new(),
        }
    }

    fn update_waker(waker: &std::sync::Arc<std::sync::Mutex<Waker>>, cx: &Context<'_>) {
        let mut old_waker = waker.lock().unwrap();
        if !old_waker.will_wake(cx.waker()) {
            old_waker.clone_from(cx.waker());
        }
    }

    fn map_vs_err(err: Error) -> io::Error {
        io::Error::other(err)
    }

    fn poll_current_read(
        &mut self,
        cx: &Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let id = match self.read_current.as_mut() {
            Some(id) => id,
            None => return Poll::Ready(Ok(())),
        };
        match id.rec.try_recv() {
            Ok(ret) => {
                self.read_current = None;
                match ret {
                    Ok(n) => {
                        let n = n.min(self.read_buf.len());
                        buf.put_slice(&self.read_buf[..n]);
                        Poll::Ready(Ok(()))
                    }
                    Err(e) => {
                        log::error!("tokio async read completion error: {}", e);
                        Poll::Ready(Err(Self::map_vs_err(e)))
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                Self::update_waker(&id.waker, cx);
                Poll::Pending
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.read_current = None;
                Poll::Ready(Err(Self::map_vs_err(Error(ErrorCode::ErrorConnLost))))
            }
        }
    }

    fn poll_current_write(&mut self, cx: &Context<'_>) -> Poll<io::Result<usize>> {
        let id = match self.write_current.as_mut() {
            Some(id) => id,
            None => return Poll::Ready(Ok(0)),
        };
        match id.rec.try_recv() {
            Ok(ret) => {
                self.write_current = None;
                match ret {
                    Ok(n) => Poll::Ready(Ok(n)),
                    Err(e) => {
                        log::error!("tokio async write completion error: {}", e);
                        Poll::Ready(Err(Self::map_vs_err(e)))
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                Self::update_waker(&id.waker, cx);
                Poll::Pending
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.write_current = None;
                Poll::Ready(Err(Self::map_vs_err(Error(ErrorCode::ErrorConnLost))))
            }
        }
    }
}

impl Drop for InstrumentTokioAdapter {
    fn drop(&mut self) {
        if let Some(id) = self.read_current.take() {
            self.instr.cancel_job(id.job_id);
        }
        if let Some(id) = self.write_current.take() {
            self.instr.cancel_job(id.job_id);
        }
    }
}

impl AsyncRead for InstrumentTokioAdapter {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let Self {
            instr,
            read_current,
            write_current: _,
            read_buf,
            write_buf: _,
        } = &mut *self;
        if read_current.is_none() {
            let remaining = buf.remaining();
            if remaining == 0 {
                return Poll::Ready(Ok(()));
            }
            read_buf.resize(remaining, 0);
            match instr.start_read_id(read_buf.as_mut(), cx.waker()) {
                Ok(id) => {
                    *read_current = Some(id);
                }
                Err(e) => return Poll::Ready(Err(Self::map_vs_err(e))),
            }
        }
        self.poll_current_read(cx, buf)
    }
}

impl AsyncWrite for InstrumentTokioAdapter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let Self {
            instr,
            read_current: _,
            write_current,
            read_buf: _,
            write_buf,
        } = &mut *self;
        if write_current.is_none() {
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            write_buf.clear();
            write_buf.extend_from_slice(buf);
            match instr.start_write_id(write_buf, cx.waker()) {
                Ok(id) => {
                    *write_current = Some(id);
                }
                Err(e) => return Poll::Ready(Err(Self::map_vs_err(e))),
            }
        }
        self.poll_current_write(cx)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        use crate::flags::FlushMode;
        self.instr
            .instr
            .visa_flush(FlushMode::WRITE_BUF | FlushMode::IO_OUT_BUF)
            .map_err(Self::map_vs_err)?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
