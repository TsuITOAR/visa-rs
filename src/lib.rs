use std::{borrow::Cow, ffi::CString, fmt::Display, ptr::NonNull, time::Duration};

use visa_sys as vs;

pub mod enums;
pub mod event;
pub mod flags;
pub mod handler;
mod session;
pub const TIMEOUT_IMMEDIATE: Duration = Duration::from_millis(vs::VI_TMO_IMMEDIATE as _);
pub const TIMEOUT_INFINITE: Duration = Duration::from_micros(vs::VI_TMO_INFINITE as _);

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

macro_rules! wrap_raw_error_in_unsafe {
    ($s:expr) => {
        match unsafe { $s } {
            state if state >= SUCCESS => {
                Result::<enums::CompletionCode>::Ok(state.try_into().unwrap())
            }
            e => Result::<enums::CompletionCode>::Err(e.try_into().unwrap()),
        }
    };
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DefaultRM(vs::ViSession);

impl Drop for DefaultRM {
    fn drop(&mut self) {
        unsafe {
            vs::viClose(self.0);
        }
    }
}

impl DefaultRM {
    pub fn new() -> Result<Self> {
        let mut new: vs::ViSession = 0;
        wrap_raw_error_in_unsafe!(vs::viOpenDefaultRM(&mut new as _))?;
        return Ok(Self(new));
    }
    pub fn find_res(&self, expr: &ResID) -> Result<ResList> {
        let mut list: vs::ViFindList = 0;
        let mut cnt: vs::ViUInt32 = 0;
        let mut instr_desc: [vs::ViChar; vs::VI_FIND_BUFLEN as usize] =
            [0; vs::VI_FIND_BUFLEN as _];
        wrap_raw_error_in_unsafe!(vs::viFindRsrc(
            self.0,
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
            self.0,
            res_name.as_vi_const_string(),
            access_mode.bits(),
            open_timeout.as_millis() as _,
            &mut instr as _,
        ))?;
        Ok(Instrument(instr))
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
pub type KeyID = VisaString;
pub type AccessKey = VisaString;

impl From<CString> for VisaString {
    fn from(c: CString) -> Self {
        Self(c)
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

impl Display for ResID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string_lossy().fmt(f)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Instrument(vs::ViSession);

impl Drop for Instrument {
    fn drop(&mut self) {
        unsafe {
            vs::viClose(self.0);
        }
    }
}

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
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viWrite(
            self.0,
            buf.as_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(map_to_io_err)?;

        Ok(ret_cnt as _)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.raw_flush(flags::FlushMode::WRITE_BUF)
            .map_err(map_to_io_err)
        // ? should call flags::FlushMODE::IO_OUT_BUF
    }
}

impl std::io::Read for Instrument {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut ret_cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viRead(
            self.0,
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut ret_cnt as _
        ))
        .map_err(map_to_io_err)?;
        Ok(ret_cnt as _)
    }
}

impl Instrument {
    pub fn raw_flush(&mut self, mode: flags::FlushMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viFlush(self.0, mode.bits()))?;
        Ok(())
    }
    pub fn get_attr(&self, attr_kind: enums::AttrKind) -> enums::Attribute {
        todo!()
    }
    pub fn set_attr(&mut self, attr: enums::Attribute) {
        todo!()
    }
    pub fn status_desc(&mut self, error: Error) -> Result<String> {
        todo!()
    }
    pub fn term(&mut self, job: JobID) -> Result<()> {
        todo!()
    }
    pub fn lock(
        &mut self,
        mode: flags::AccessMode,
        timeout: Duration,
        key: KeyID,
    ) -> Result<AccessKey> {
        todo!()
    }
    pub fn unlock(&mut self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viUnlock(self.0))?;
        Ok(())
    }
    pub fn enable_event(
        &mut self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
        filter: event::EventFilter,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viEnableEvent(
            self.0,
            event_kind as _,
            mechanism as _,
            filter as _
        ))?;
        Ok(())
    }
    pub fn disable_event(
        &mut self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDisableEvent(self.0, event_kind as _, mechanism as _,))?;
        Ok(())
    }
    pub fn discard_events(
        &mut self,
        event: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viDiscardEvents(self.0, event as _, mechanism as _,))?;
        Ok(())
    }
    pub fn wait_on_event(
        &mut self,
        event_kind: event::EventKind,
        timeout: Duration,
    ) -> Result<event::Event> {
        let mut handler: vs::ViEvent = 0;
        let mut out_kind: vs::ViEventType = 0;
        wrap_raw_error_in_unsafe!(vs::viWaitOnEvent(
            self.0,
            event_kind as _,
            timeout.as_millis() as _,
            &mut out_kind as _,
            &mut handler as _
        ))?;
        let kind = event::EventKind::try_from(out_kind).expect("should be valid event type");
        Ok(event::Event { handler, kind })
    }
    pub fn install_handler<F, Out>(
        &mut self,
        event_kind: event::EventKind,
        ops: impl FnMut(&mut Instrument, event::Event) -> Result<Out>,
    ) -> Result<handler::Handler<Out, impl FnMut(&mut Instrument, event::Event) -> vs::ViStatus>>
    {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut ops = ops;
        let closure = move |instr: &mut Instrument, event: event::Event| -> vs::ViStatus {
            let ret = ops(instr, event);
            match ret {
                Err(e) => e.into(),
                Ok(r) => {
                    sender.send(r).expect("receiver side should be valid");
                    SUCCESS
                }
            }
        };
        let (p_f, p_c, call) = split_closure(closure);

        wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            self.0,
            event_kind as _,
            Some(call),
            p_c
        ))?;
        Ok(handler::Handler::new(
            self.0, receiver, event_kind, call, p_f,
        ))
    }
}

fn split_closure<F>(
    closure: F,
) -> (
    std::ptr::NonNull<F>,
    *mut std::ffi::c_void,
    unsafe extern "C" fn(
        vs::ViSession,
        vs::ViEventType,
        vs::ViEvent,
        *mut std::ffi::c_void,
    ) -> vs::ViStatus,
)
where
    F: FnMut(&mut Instrument, event::Event) -> vs::ViStatus,
{
    use std::ffi::c_void;
    let data = Box::into_raw(Box::new(closure));
    unsafe extern "C" fn trampoline<T>(
        instr: vs::ViSession,
        event_type: vs::ViEventType,
        event: vs::ViEvent,
        user_data: *mut c_void,
    ) -> vs::ViStatus
    where
        T: FnMut(&mut Instrument, event::Event) -> vs::ViStatus,
    {
        let closure: &mut T = &mut *(user_data as *mut T);
        let mut instr = Instrument(instr);
        let ret = closure(&mut instr, event::Event::new(event, event_type));
        std::mem::forget(instr); // ? no sure yet, in official example session not closed
        ret
    }

    (
        NonNull::new(data).expect("impossible to pass in a null ptr"),
        data as *mut c_void,
        trampoline::<F>,
    )
}

pub struct JobID(vs::ViJobId);
