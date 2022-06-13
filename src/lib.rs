#![feature(cstr_from_bytes_until_nul)]
use std::ffi::CStr;
use std::{borrow::Cow, ffi::CString, fmt::Display, time::Duration};

use visa_sys as vs;

pub mod enums;
pub mod event;
pub mod flags;
pub mod handler;
pub mod session;

use session::{AsRawSs, AsSs, FromRawSs, IntoRawSs, OwnedSs};

pub const TIMEOUT_IMMEDIATE: Duration = Duration::from_millis(vs::VI_TMO_IMMEDIATE as _);
pub const TIMEOUT_INFINITE: Duration = Duration::from_micros(vs::VI_TMO_INFINITE as _);
macro_rules! impl_session_traits {
    ($($id:ident),* $(,)?) => {
        $(
            impl IntoRawSs for $id {
                fn into_raw_ss(self) -> session::RawSs {
                    self.0.into_raw_ss()
                }
            }

            impl AsRawSs for $id {
                fn as_raw_ss(&self) -> session::RawSs {
                    self.0.as_raw_ss()
                }
            }

            impl AsSs for $id {
                fn as_ss(&self) -> session::BorrowedSs<'_> {
                    self.0.as_ss()
                }
            }

            impl FromRawSs for $id {
                unsafe fn from_raw_ss(s: session::RawSs) -> Self {
                    Self(FromRawSs::from_raw_ss(s))
                }
            }
        )*
    };
}

impl_session_traits! { DefaultRM, Instrument}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Error(enums::ErrorCode);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<enums::ErrorCode> for Error {
    fn from(s: enums::ErrorCode) -> Self {
        Self(s)
    }
}

impl Into<enums::ErrorCode> for Error {
    fn into(self) -> enums::ErrorCode {
        self.0
    }
}

impl Into<vs::ViStatus> for Error {
    fn into(self) -> vs::ViStatus {
        self.0.into()
    }
}

impl TryFrom<vs::ViStatus> for Error {
    type Error = <enums::ErrorCode as TryFrom<vs::ViStatus>>::Error;
    fn try_from(value: vs::ViStatus) -> std::result::Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}
pub type Result<T> = std::result::Result<T, Error>;
const SUCCESS: vs::ViStatus = vs::VI_SUCCESS as _;

#[macro_export]
macro_rules! wrap_raw_error_in_unsafe {
    ($s:expr) => {
        match unsafe { $s } {
            state if state >= $crate::SUCCESS => {
                Result::<$crate::enums::CompletionCode>::Ok(state.try_into().unwrap())
            }
            e => Result::<$crate::enums::CompletionCode>::Err(e.try_into().unwrap()),
        }
    };
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DefaultRM(session::OwnedSs);

impl DefaultRM {
    pub fn new() -> Result<Self> {
        let mut new: vs::ViSession = 0;
        wrap_raw_error_in_unsafe!(vs::viOpenDefaultRM(&mut new as _))?;
        return Ok(Self(unsafe { OwnedSs::from_raw_ss(new) }));
    }
    pub fn find_res(&self, expr: &ResID) -> Result<ResList> {
        let mut list: vs::ViFindList = 0;
        let mut cnt: vs::ViUInt32 = 0;
        let mut instr_desc: [vs::ViChar; vs::VI_FIND_BUFLEN as usize] =
            [0; vs::VI_FIND_BUFLEN as _];
        wrap_raw_error_in_unsafe!(vs::viFindRsrc(
            self.as_raw_ss(),
            expr.as_vi_const_string(),
            &mut list,
            &mut cnt,
            instr_desc.as_mut_ptr(),
        ))?;
        Ok(ResList {
            list,
            cnt: cnt as _,
            instr_desc,
        })
    }
    pub fn open(
        &self,
        res_name: &ResID,
        access_mode: flags::AccessMode,
        open_timeout: Duration,
    ) -> Result<Instrument> {
        let mut instr: vs::ViSession = 0;
        wrap_raw_error_in_unsafe!(vs::viOpen(
            self.as_raw_ss(),
            res_name.as_vi_const_string(),
            access_mode.bits(),
            open_timeout.as_millis() as _,
            &mut instr as _,
        ))?;
        Ok(unsafe { Instrument::from_raw_ss(instr) })
    }
}

#[derive(Debug)]
pub struct ResList {
    list: vs::ViFindList,
    cnt: i32,
    instr_desc: [vs::ViChar; vs::VI_FIND_BUFLEN as _],
}

impl ResList {
    pub fn find_next(&mut self) -> Result<Option<ResID>> {
        if self.cnt < 1 {
            return Ok(None);
        }
        let next = ResID::from(
            CString::new(
                self.instr_desc
                    .into_iter()
                    .map(|x| x as u8)
                    .take_while(|x| *x != b'\0')
                    .collect::<Vec<_>>(),
            )
            .expect("can not be null"),
        );
        if self.cnt > 1 {
            wrap_raw_error_in_unsafe!(vs::viFindNext(self.list, self.instr_desc.as_mut_ptr()))?;
        }
        self.cnt -= 1;
        Ok(next.into())
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct VisaString(CString);

pub type ResID = VisaString;
pub type AccessKey = VisaString;

impl From<CString> for VisaString {
    fn from(c: CString) -> Self {
        Self(c)
    }
}

type VisaBuf = [u8; vs::VI_FIND_BUFLEN as _];

fn new_visa_buf() -> VisaBuf {
    [0; vs::VI_FIND_BUFLEN as _]
}

impl TryFrom<[u8; vs::VI_FIND_BUFLEN as _]> for VisaString {
    type Error = core::ffi::FromBytesUntilNulError;
    fn try_from(f: [u8; vs::VI_FIND_BUFLEN as _]) -> std::result::Result<Self, Self::Error> {
        Ok(Self(CStr::from_bytes_until_nul(f.as_slice())?.to_owned()))
    }
}

impl VisaString {
    fn as_vi_const_string(&self) -> vs::ViConstString {
        self.0.as_ptr()
    }
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.0.to_string_lossy()
    }
}

impl Display for VisaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string_lossy().fmt(f)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Instrument(OwnedSs);

fn map_to_io_err(err: Error) -> std::io::Error {
    use enums::ErrorCode::*;
    use std::io::ErrorKind::*;
    std::io::Error::new(
        match err.0 {
            ErrorInvObject => AddrNotAvailable,
            ErrorNsupOper => Unsupported,
            ErrorRsrcLocked => ConnectionRefused,
            ErrorTmo => TimedOut,
            ErrorRawWrProtViol | ErrorRawRdProtViol => InvalidData,
            ErrorInpProtViol | ErrorOutpProtViol => BrokenPipe,
            ErrorBerr => BrokenPipe,
            ErrorInvSetup => InvalidInput,
            ErrorNcic => PermissionDenied,
            ErrorNlisteners => Other,
            ErrorAsrlParity | ErrorAsrlFraming => Other,
            ErrorAsrlOverrun => Other,
            ErrorConnLost => BrokenPipe,
            ErrorInvMask => InvalidInput,
            ErrorIo => return std::io::Error::last_os_error(),
            _ => unreachable!(),
        },
        err,
    )
}

impl std::io::Write for Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        <&Instrument>::write(&mut &*self, buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        <&Instrument>::flush(&mut &*self)
    }
}

impl std::io::Read for Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        <&Instrument>::read(&mut &*self, buf)
    }
}

impl std::io::Write for &Instrument {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viWrite(
            self.as_raw_ss(),
            buf.as_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(map_to_io_err)?;

        Ok(ret_cnt as _)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.visa_flush(flags::FlushMode::WRITE_BUF)
            .map_err(map_to_io_err)
        // ? should call flags::FlushMODE::IO_OUT_BUF
    }
}

impl std::io::Read for &Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viRead(
            self.as_raw_ss(),
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(map_to_io_err)?;
        Ok(ret_cnt as _)
    }
}

impl Instrument {
    pub fn visa_flush(&self, mode: flags::FlushMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viFlush(self.as_raw_ss(), mode.bits()))?;
        Ok(())
    }

    pub fn status_desc(&self, error: Error) -> Result<VisaString> {
        let mut desc: VisaBuf = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viStatusDesc(
            self.as_raw_ss(),
            error.into(),
            desc.as_mut_ptr() as _
        ))?;
        Ok(desc.try_into().unwrap())
    }

    pub fn lock(
        &self,
        mode: flags::AccessMode,
        timeout: Duration,
        key: Option<AccessKey>,
    ) -> Result<Option<AccessKey>> {
        if (mode & flags::AccessMode::SHARED_LOCK).is_empty() {
            wrap_raw_error_in_unsafe!(vs::viLock(
                self.as_raw_ss(),
                mode.bits(),
                timeout.as_millis() as _,
                vs::VI_NULL as _,
                vs::VI_NULL as _
            ))?;
            Ok(None)
        } else {
            let mut ak = new_visa_buf();
            wrap_raw_error_in_unsafe!(vs::viLock(
                self.as_raw_ss(),
                mode.bits(),
                timeout.as_millis() as _,
                key.map(|x| x.as_vi_const_string())
                    .unwrap_or(vs::VI_NULL as _),
                ak.as_mut_ptr() as _
            ))?;
            Ok(Some(ak.try_into().unwrap()))
        }
    }

    pub fn lock_exclusive(&self, timeout: Duration) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            vs::VI_NULL as _,
            vs::VI_NULL as _
        ))?;
        Ok(())
    }

    pub fn lock_shared(&self, timeout: Duration) -> Result<AccessKey> {
        let mut ak = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            vs::VI_NULL as _,
            ak.as_mut_ptr() as _
        ))?;
        Ok(ak.try_into().unwrap())
    }

    pub fn lock_shared_with_key(&self, timeout: Duration, key: AccessKey) -> Result<AccessKey> {
        let mut ak = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viLock(
            self.as_raw_ss(),
            flags::AccessMode::EXCLUSIVE_LOCK.bits(),
            timeout.as_millis() as _,
            key.as_vi_const_string() as _,
            ak.as_mut_ptr() as _
        ))?;
        Ok(ak.try_into().unwrap())
    }

    pub fn unlock(&mut self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viUnlock(self.as_raw_ss()))?;
        Ok(())
    }
    pub fn enable_event(
        &self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
        filter: event::EventFilter,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viEnableEvent(
            self.as_raw_ss(),
            event_kind as _,
            mechanism as _,
            filter as _
        ))?;
        Ok(())
    }
    pub fn disable_event(
        &self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDisableEvent(
            self.as_raw_ss(),
            event_kind as _,
            mechanism as _,
        ))?;
        Ok(())
    }
    pub fn discard_events(
        &self,
        event: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDiscardEvents(
            self.as_raw_ss(),
            event as _,
            mechanism as _,
        ))?;
        Ok(())
    }
    pub fn wait_on_event(
        &self,
        event_kind: event::EventKind,
        timeout: Duration,
    ) -> Result<event::Event> {
        let mut handler: vs::ViEvent = 0;
        let mut out_kind: vs::ViEventType = 0;
        wrap_raw_error_in_unsafe!(vs::viWaitOnEvent(
            self.as_raw_ss(),
            event_kind as _,
            timeout.as_millis() as _,
            &mut out_kind as _,
            &mut handler as _
        ))?;
        let kind = event::EventKind::try_from(out_kind).expect("should be valid event type");
        Ok(event::Event { handler, kind })
    }

    ///
    /// Note: for some reason pass a closure with signature `|instr, event|{...}`
    /// may get error. Instead, use `|instr: & Instrument, event: & Event|{...}`.
    ///

    pub fn install_handler<F: handler::Callback>(
        &self,
        event_kind: event::EventKind,
        callback: F,
    ) -> Result<handler::Handler<'_, F>> {
        handler::Handler::new(self.as_ss(), event_kind, callback)
    }
}

mod async_io;

impl Instrument {
    pub fn visa_read_async(&self, buf: &mut [u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        wrap_raw_error_in_unsafe!(vs::viReadAsync(
            self.as_raw_ss(),
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }
    pub fn visa_write_async(&self, buf: &[u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        wrap_raw_error_in_unsafe!(vs::viWriteAsync(
            self.as_raw_ss(),
            buf.as_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }

    pub async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        async_io::AsyncRead::new(self, buf).await
    }

    pub async fn async_write(&self, buf: &mut [u8]) -> Result<usize> {
        async_io::AsyncWrite::new(self, buf).await
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub struct JobID(vs::ViJobId);
