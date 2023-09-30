//!
//! Safe rust bindings for VISA(Virtual Instrument Software Architecture) library
//!
//! Most documentation comes from [NI-VISA Product Documentation](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/help_file_title.html)
//!
//! # Requirements
//! This crate needs to link to an installed visa library, for example, [NI-VISA](https://www.ni.com/en-us/support/downloads/drivers/download.ni-visa.html).
//!
//! You can specify path of `visa64.lib` file (or `visa32.lib` on 32-bit systems) by setting environment variable `LIB_VISA_PATH`.
//!
//! On Windows, the default installation path will be added if no path is specified.
//!
//! # Example
//!
//! Codes below will find the first Keysight instrument in your environment and print out its `*IDN?` response.
//!
//! ```
//! fn main() -> visa_rs::Result<()>{
//!     use std::ffi::CString;
//!     use std::io::{BufRead, BufReader, Read, Write};
//!     use visa_rs::prelude::*;
//!
//!     // open default resource manager
//!     let rm: DefaultRM = DefaultRM::new()?;
//!
//!     // expression to match resource name
//!     let expr = CString::new("?*KEYSIGH?*INSTR").unwrap().into();
//!
//!     // find the first resource matched
//!     let rsc = rm.find_res(&expr)?;
//!
//!     // open a session to the resource, the session will be closed when rm is dropped
//!     let instr: Instrument = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
//!
//!     // write message
//!     (&instr).write_all(b"*IDN?\n").map_err(io_to_vs_err)?;
//!
//!     // read response
//!     let mut buf_reader = BufReader::new(&instr);
//!     let mut buf = String::new();
//!     buf_reader.read_line(&mut buf).map_err(io_to_vs_err)?;
//!
//!     eprintln!("{}", buf);
//!     Ok(())
//! }
//! ```

use enums::{attribute, event};
use std::ffi::CStr;
use std::{borrow::Cow, ffi::CString, fmt::Display, time::Duration};
pub use visa_sys as vs;

mod async_io;
pub mod enums;
pub mod flags;
pub mod handler;
mod instrument;
pub mod prelude;
pub mod session;

pub use instrument::Instrument;

use session::{AsRawSs, AsSs, FromRawSs, IntoRawSs, OwnedSs};

pub const TIMEOUT_IMMEDIATE: Duration = Duration::from_millis(vs::VI_TMO_IMMEDIATE as _);
pub const TIMEOUT_INFINITE: Duration = Duration::from_millis(vs::VI_TMO_INFINITE as _);
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

macro_rules! impl_session_traits_for_borrowed {
    ($($id:ident),* $(,)?) => {
        $(
            impl AsRawSs for $id<'_> {
                fn as_raw_ss(&self) -> session::RawSs {
                    self.0.as_raw_ss()
                }
            }

            impl AsSs for $id<'_> {
                fn as_ss(&self) -> session::BorrowedSs<'_> {
                    self.0.as_ss()
                }
            }
        )*
    };
}

impl_session_traits! { DefaultRM, Instrument}
impl_session_traits_for_borrowed! {WeakRM}
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Error(pub enums::status::ErrorCode);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<enums::status::ErrorCode> for Error {
    fn from(s: enums::status::ErrorCode) -> Self {
        Self(s)
    }
}

impl From<Error> for enums::status::ErrorCode {
    fn from(s: Error) -> Self {
        s.0
    }
}

impl From<Error> for vs::ViStatus {
    fn from(s: Error) -> Self {
        s.0.into()
    }
}

impl TryFrom<vs::ViStatus> for Error {
    type Error = <enums::status::ErrorCode as TryFrom<vs::ViStatus>>::Error;
    fn try_from(value: vs::ViStatus) -> std::result::Result<Self, Self::Error> {
        Ok(Self(value.try_into()?))
    }
}

impl TryFrom<std::io::Error> for Error {
    type Error = std::io::Error;
    fn try_from(value: std::io::Error) -> std::result::Result<Self, Self::Error> {
        if let Some(e) = value.get_ref() {
            if let Some(e) = e.downcast_ref::<Error>() {
                return Ok(*e);
            }
        }
        Err(value)
    }
}

/// Quickly convert [std::io::Error].
///
///  # Panics
///
/// Panic if the input Error is not converted from [visa_rs::Error](Error), use [TryInto] to perform conversion,
/// the io error must be generated from [Instrument] IO ops
pub fn io_to_vs_err(e: std::io::Error) -> Error {
    e.try_into().unwrap()
}

fn vs_to_io_err(err: Error) -> std::io::Error {
    use enums::status::ErrorCode::*;
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
            ErrorIo => std::io::Error::last_os_error().kind(),
            _ => unreachable!(),
        },
        err,
    )
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<enums::attribute::AttrStatus> for Result<enums::status::CompletionCode> {
    fn from(a: enums::attribute::AttrStatus) -> Self {
        match a.into_inner() {
            state if state >= SUCCESS => Ok(state.try_into().unwrap()),
            e => Err(e.try_into().unwrap()),
        }
    }
}

const SUCCESS: vs::ViStatus = vs::VI_SUCCESS as _;

#[doc(hidden)]
#[macro_export]
macro_rules! wrap_raw_error_in_unsafe {
    ($s:expr) => {
        match unsafe { $s } {
            state if state >= $crate::SUCCESS => $crate::Result::<
                $crate::enums::status::CompletionCode,
            >::Ok(state.try_into().unwrap()),
            e => {
                $crate::Result::<$crate::enums::status::CompletionCode>::Err(e.try_into().unwrap())
            }
        }
    };
}

/// Ability as the Default Resource Manager for VISA
pub trait AsResourceManager: AsRawSs {
    ///
    /// Queries a VISA system to locate the resources associated with a specified interface.
    ///
    /// The viFindRsrc() operation matches the value specified in the expr parameter with the resources available for a particular interface. A regular expression is a string consisting of ordinary characters as well as special characters. You use a regular expression to specify patterns to match in a given string; in other words, it is a search criterion. The viFindRsrc() operation uses a case-insensitive compare feature when matching resource names against the regular expression specified in expr. For example, calling viFindRsrc() with "VXI?*INSTR" would return the same resources as invoking it with "vxi?*instr".
    ///
    /// All resource strings returned by viFindRsrc() will always be recognized by viOpen(). However, viFindRsrc() will not necessarily return all strings that you can pass to viParseRsrc() or viOpen(). This is especially true for network and TCPIP resources.
    ///
    /// The search criteria specified in the expr parameter has two parts: a regular expression over a resource string, and an optional logical expression over attribute values. The regular expression is matched against the resource strings of resources known to the VISA Resource Manager. If the resource string matches the regular expression, the attribute values of the resource are then matched against the expression over attribute values. If the match is successful, the resource has met the search criteria and gets added to the list of resources found.
    ///
    /// Special Characters and Operators|       Meaning
    /// :-----------------------------: |       :----------------------------------
    /// \?                              |       Matches any one character.
    /// \\                              |       Makes the character that follows it an ordinary character instead of special character. For example, when a question mark follows a backslash (\?), it matches the ? character instead of  any one character.
    /// \[list\]                        |       Matches any one character from the enclosed list. You can use a hyphen to match a range of characters.
    /// \[^list\]                       |       Matches any character not in the enclosed list. You can use a hyphen to match a range of characters.
    /// \*                              |       Matches 0 or more occurrences of the preceding character or expression.
    /// \+                              |       Matches 1 or more occurrences of the preceding character or expression.
    /// Exp\|exp                        |       Matches either the preceding or following expression. The or operator | matches the entire expression that precedes or follows it and not just the character that precedes or    follows it. For example, VXI|GPIB means (VXI)|(GPIB), not VX(I|G)PIB.
    /// (exp)                           |       Grouping characters or expressions.
    ///
    ///  Regular Expression             |   Sample Matches
    /// :-----------------------------: |   :----------------------------------
    ///  GPIB?*INSTR                    |    GPIB0::2::INSTR, and GPIB1::1::1::INSTR.
    ///  GPIB\[0-9\]\*::?*INSTR         |    GPIB0::2::INSTR and GPIB1::1::1::INSTR.
    ///  GPIB\[^0\]::?*INSTR            |    GPIB1::1::1::INSTR but not GPIB0::2::INSTR or GPIB12::8::INSTR.
    ///  VXI?*INSTR                     |    VXI0::1::INSTR.
    ///  ?*VXI\[0-9\]\*::?*INSTR        |    VXI0::1::INSTR.
    ///  ASRL\[0-9\]\*::?*INSTR         |    ASRL1::INSTR but not VXI0::5::INSTR.
    ///  ASRL1\+::INSTR                 |    ASRL1::INSTR and ASRL11::INSTR but not ASRL2::INSTR.
    ///  (GPIB\|VXI)?*INSTR             |    GPIB1::5::INSTR and VXI0::3::INSTR but not ASRL2::INSTR.
    ///  (GPIB0\|VXI0)::1::INSTR        |    GPIB0::1::INSTR and VXI0::1::INSTR.
    ///  ?*INSTR                        |    all INSTR (device) resources.
    ///  ?*VXI\[0-9\]\*::?*MEMACC       |    VXI0::MEMACC.
    ///  VXI0::?*                       |    VXI0::1::INSTR, VXI0::2::INSTR, and VXI0::MEMACC.
    ///  ?*                             |    all resources.
    ///  visa://hostname/?*             |    all resources on the specified remote system. The hostname can be represented as either an IP address (dot-notation) or network machine name. This remote system need not be a configured remote system.
    ///  /?*                            |    all resources on the local machine. Configured remote systems are not queried.
    ///  visa:/ASRL?*INSTR              |    all ASRL resources on the local machine and returns them in URL format (for example, visa:/ASRL1::INSTR).
    ///
    /// see also [official doc](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/vifindrsrc.html)
    ///
    fn find_res_list(&self, expr: &ResID) -> Result<ResList> {
        let mut list: vs::ViFindList = 0;
        let mut cnt: vs::ViUInt32 = 0;
        let mut instr_desc = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viFindRsrc(
            self.as_raw_ss(),
            expr.as_vi_const_string(),
            &mut list,
            &mut cnt,
            instr_desc.as_mut_ptr() as _,
        ))?;
        Ok(ResList {
            list,
            cnt: cnt as _,
            instr_desc,
        })
    }

    ///
    /// Queries a VISA system to locate the resources associated with a specified interface, return the first resource matched
    ///
    fn find_res(&self, expr: &ResID) -> Result<ResID> {
        /*
        !keysight impl visa will try to write at address vs::VI_NULL, cause exit code: 0xc0000005, STATUS_ACCESS_VIOLATION
        let mut instr_desc = new_visa_buf();
        let mut cnt: vs::ViUInt32 = 0;
        wrap_raw_error_in_unsafe!(vs::viFindRsrc(
            self.as_raw_ss(),
            expr.as_vi_const_string(),
            vs::VI_NULL as _,
            &mut cnt as *mut _,
            instr_desc.as_mut_ptr() as _,
        ))?;
        Ok(instr_desc.try_into().unwrap())
        */

        Ok(self.find_res_list(expr)?.instr_desc.try_into().unwrap())
    }

    /// Parse a resource string to get the interface information.
    fn parse_res(&self, res: &ResID) -> Result<(attribute::AttrIntfType, attribute::AttrIntfNum)> {
        let mut ty = 0;
        let mut num = 0;
        wrap_raw_error_in_unsafe!(vs::viParseRsrc(
            self.as_raw_ss(),
            res.as_vi_const_string(),
            &mut ty as *mut _,
            &mut num as *mut _
        ))?;
        unsafe {
            Ok((
                attribute::AttrIntfType::new_unchecked(ty),
                attribute::AttrIntfNum::new_unchecked(num),
            ))
        }
    }

    /// Parse a resource string to get extended interface information.
    ///
    /// the returned three VisaStrings are:
    ///
    /// + Specifies the resource class (for example, "INSTR") of the given resource string.
    ///
    /// + This is the expanded version of the given resource string. The format should be similar to the VISA-defined canonical resource name.
    ///
    /// + Specifies the user-defined alias for the given resource string.
    fn parse_res_ex(
        &self,
        res: &ResID,
    ) -> Result<(
        attribute::AttrIntfType,
        attribute::AttrIntfNum,
        VisaString,
        VisaString,
        VisaString,
    )> {
        let mut ty = 0;
        let mut num = 0;
        let mut str1 = new_visa_buf();
        let mut str2 = new_visa_buf();
        let mut str3 = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viParseRsrcEx(
            self.as_raw_ss(),
            res.as_vi_const_string(),
            &mut ty as *mut _,
            &mut num as *mut _,
            str1.as_mut_ptr() as _,
            str2.as_mut_ptr() as _,
            str3.as_mut_ptr() as _,
        ))?;
        unsafe {
            Ok((
                attribute::AttrIntfType::new_unchecked(ty),
                attribute::AttrIntfNum::new_unchecked(num),
                str1.try_into().unwrap(),
                str2.try_into().unwrap(),
                str3.try_into().unwrap(),
            ))
        }
    }

    ///
    /// Opens a session to the specified resource.
    ///
    /// For the parameter accessMode, either VI_EXCLUSIVE_LOCK (1) or VI_SHARED_LOCK (2).
    ///
    /// VI_EXCLUSIVE_LOCK (1) is used to acquire an exclusive lock immediately upon opening a session; if a lock cannot be acquired, the session is closed and an error is returned.
    ///
    /// VI_LOAD_CONFIG (4) is used to configure attributes to values specified by some external configuration utility. Multiple access modes can be used simultaneously by specifying a bit-wise OR of the values other than VI_NULL.
    ///
    ///  NI-VISA currently supports VI_LOAD_CONFIG only on Serial INSTR sessions.
    ///
    fn open(
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

    /// Close this session and all find lists and device sessions.
    fn close_all(&self) {
        std::mem::drop(unsafe { DefaultRM::from_raw_ss(self.as_raw_ss()) })
    }
}

impl<'a> AsResourceManager for WeakRM<'a> {}
impl AsResourceManager for DefaultRM {}

/// A [`ResourceManager`](AsResourceManager) which is [`Clone`] and doesn't close everything on drop
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct WeakRM<'a>(session::BorrowedSs<'a>);

/// A [`ResourceManager`](AsResourceManager) which close everything on drop
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DefaultRM(session::OwnedSs);

impl DefaultRM {
    /// [`DefaultRM`] will close everything opened by it on drop.
    /// By converting to a [`WeakRM`], such behavior can be avoided.
    ///
    /// *Note*: Sessions opened by another resource manager (get from another call to [`Self::new`]) won't be influenced.
    pub fn leak(self) -> WeakRM<'static> {
        unsafe { WeakRM(session::BorrowedSs::borrow_raw(self.into_raw_ss())) }
    }

    /// [`DefaultRM`] will close everything opened by it on drop.
    /// By converting to a [`WeakRM`], such behavior can be avoided.
    ///
    /// *Note*: Sessions opened by another resource manager (get from another call to [`Self::new`]) won't be influenced.
    pub fn borrow(&'_ self) -> WeakRM<'_> {
        WeakRM(self.as_ss())
    }

    /// Returns a session to the Default Resource Manager resource.
    ///
    /// The first call to this function initializes the VISA system, including the Default Resource Manager resource, and also returns a session to that resource. Subsequent calls to this function return unique sessions to the same Default Resource Manager resource.
    ///
    /// When a Resource Manager session is dropped, not only is that session closed, but also all find lists and device sessions (which that Resource Manager session was used to create) are closed.
    ///
    pub fn new() -> Result<Self> {
        let mut new: vs::ViSession = 0;
        wrap_raw_error_in_unsafe!(vs::viOpenDefaultRM(&mut new as _))?;
        Ok(Self(unsafe { OwnedSs::from_raw_ss(new) }))
    }
}

/// Returned by [`DefaultRM::find_res_list`], handler to iterator over matched resources
#[derive(Debug)]
pub struct ResList {
    list: vs::ViFindList,
    cnt: i32,
    instr_desc: VisaBuf,
}

impl ResList {
    /// Returns the next resource from the list of resources found during a previous call to viFindRsrc().
    pub fn find_next(&mut self) -> Result<Option<ResID>> {
        if self.cnt < 1 {
            return Ok(None);
        }
        let next: ResID = self.instr_desc.try_into().unwrap();
        if self.cnt > 1 {
            wrap_raw_error_in_unsafe!(vs::viFindNext(
                self.list,
                self.instr_desc.as_mut_ptr() as _
            ))?;
        }
        self.cnt -= 1;
        Ok(Some(next))
    }
}

/// Simple wrapper of [std::ffi::CString]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
pub struct VisaString(CString);

/// resource ID
pub type ResID = VisaString;

/// Access key used in [`Instrument::lock`]
pub type AccessKey = VisaString;

impl From<CString> for VisaString {
    fn from(c: CString) -> Self {
        Self(c)
    }
}

impl From<VisaString> for CString {
    fn from(value: VisaString) -> Self {
        value.0
    }
}

type VisaBuf = [u8; vs::VI_FIND_BUFLEN as _];

const fn new_visa_buf() -> VisaBuf {
    [0; vs::VI_FIND_BUFLEN as _]
}

#[derive(Debug, Clone, Copy)]
pub struct FromBytesWithNulError;

impl Display for FromBytesWithNulError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bytes include none null(\\0) character")
    }
}

impl std::error::Error for FromBytesWithNulError {}

impl TryFrom<[u8; vs::VI_FIND_BUFLEN as _]> for VisaString {
    type Error = FromBytesWithNulError;
    fn try_from(f: [u8; vs::VI_FIND_BUFLEN as _]) -> std::result::Result<Self, Self::Error> {
        let mut index = f.split_inclusive(|t| *t == b'\0');
        let cstr = index.next().ok_or(FromBytesWithNulError)?;
        Ok(Self(
            CStr::from_bytes_with_nul(cstr)
                .map_err(|_| FromBytesWithNulError)?
                .to_owned(),
        ))
    }
}

impl VisaString {
    fn as_vi_const_string(&self) -> vs::ViConstString {
        self.0.as_ptr()
    }
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        self.0.to_string_lossy()
    }
    pub fn from_string(s: String) -> Option<Self> {
        CString::new(s).ok().map(|x| x.into())
    }
}

impl Display for VisaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_string_lossy().fmt(f)
    }
}

/// Job ID of an asynchronous operation.
///
/// Returned by [`Instrument::visa_read_async`] or [`Instrument::visa_write_async`], used to be compared with the attribute [AttrJobId](enums::attribute::AttrJobId) got from [Event](enums::event::Event) to distinguish operations.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub struct JobID(pub(crate) vs::ViJobId);

impl JobID {
    ///create JobId with value null, used in [`Instrument::terminate`] to abort all calls
    pub fn null() -> Self {
        Self(vs::VI_NULL as _)
    }
}

impl From<enums::attribute::AttrJobId> for JobID {
    fn from(s: enums::attribute::AttrJobId) -> Self {
        Self(s.into_inner())
    }
}

impl PartialEq<enums::attribute::AttrJobId> for JobID {
    fn eq(&self, other: &enums::attribute::AttrJobId) -> bool {
        self.eq(&JobID::from(other.clone()))
    }
}

#[cfg(test)]
mod test {
    use std::ffi::CString;

    use crate::{
        session::{AsRawSs, FromRawSs},
        AsResourceManager, DefaultRM, *,
    };
    use anyhow::{bail, Result};
    #[test]
    fn rm_behavior() -> Result<()> {
        let rm1 = DefaultRM::new()?;
        let rm2 = DefaultRM::new()?;
        let r1 = rm1.as_raw_ss();
        assert_ne!(rm1, rm2);
        std::mem::drop(rm1);
        let expr = CString::new("?*").unwrap().into();
        match unsafe { DefaultRM::from_raw_ss(r1) }.find_res(&expr) {
            Err(crate::Error(crate::enums::status::ErrorCode::ErrorInvObject)) => {
                Ok::<_, crate::Error>(())
            }
            _ => bail!("unexpected behavior using a resource manager after it is dropped"),
        }?;
        match rm2.find_res(&expr) {
            Ok(_) | Err(crate::Error(crate::enums::status::ErrorCode::ErrorRsrcNfound)) => Ok(()),
            _ => bail!("unexpected behavior using a resource manager after dropping another resource manager"),
        }
    }

    #[test]
    fn convert_io_error() {
        let vs_error = Error(enums::status::ErrorCode::ErrorTmo);
        let io_error = vs_to_io_err(vs_error);
        assert_eq!(Error::try_from(io_error).unwrap(), vs_error);
        let no_vs_io_error = std::io::Error::new(std::io::ErrorKind::Other, FromBytesWithNulError);
        assert!(Error::try_from(no_vs_io_error).is_err());
    }
}
