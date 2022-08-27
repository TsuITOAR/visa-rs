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
//! ```
//! # fn main() -> visa_rs::Result<()>{
//!     use std::ffi::CString;
//!     use std::io::{BufRead, BufReader, Read, Write};
//!     use visa_rs::{flags::AccessMode, DefaultRM, TIMEOUT_IMMEDIATE};
//!     let rm = DefaultRM::new()?; //open default resource manager
//!     let expr = CString::new("?*KEYSIGH?*INSTR").unwrap().into(); //expr used to match resource name
//!     let rsc = rm.find_res(&expr)?; // find the first resource matched
//!     let mut instr = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?; //open a session to resource
//!     instr.write_all(b"*IDN?\n").unwrap(); //write message
//!     let mut buf_reader = BufReader::new(instr);
//!     let mut buf = String::new();
//!     buf_reader.read_line(&mut buf).unwrap(); //read response
//!     eprintln!("{}", buf);
//!     Ok(())
//! # }
//! ```

use enums::{attribute, event};
use std::ffi::CStr;
use std::{borrow::Cow, ffi::CString, fmt::Display, time::Duration};
use visa_sys as vs;

pub mod enums;
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

/// Default Resource Manager for VISA
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct DefaultRM(session::OwnedSs);

impl DefaultRM {
    /// Returns a session to the Default Resource Manager resource.
    ///
    /// The first call to this function initializes the VISA system, including the Default Resource Manager resource, and also returns a session to that resource. Subsequent calls to this function return unique sessions to the same Default Resource Manager resource.
    ///
    ///When a Resource Manager session is dropped, not only is that session closed, but also all find lists and device sessions (which that Resource Manager session was used to create) are closed.
    ///
    pub fn new() -> Result<Self> {
        let mut new: vs::ViSession = 0;
        wrap_raw_error_in_unsafe!(vs::viOpenDefaultRM(&mut new as _))?;
        Ok(Self(unsafe { OwnedSs::from_raw_ss(new) }))
    }

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
    ///  visa://hostname/?*             |    all resources on the specified remote system. The hostname can be represented as either an IP address (dot-notation) or network machine name. This remote system need not be a conf    igured remote system.
    ///  /?*                            |    all resources on the local machine. Configured remote systems are not queried.
    ///  visa:/ASRL?*INSTR              |    all ASRL resources on the local machine and returns them in URL format (for example, visa:/ASRL1::INSTR).
    ///
    /// see also [official doc](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/vifindrsrc.html)
    ///
    pub fn find_res_list(&self, expr: &ResID) -> Result<ResList> {
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
    pub fn find_res(&self, expr: &ResID) -> Result<ResID> {
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
    pub fn parse_res(
        &self,
        res: &ResID,
    ) -> Result<(attribute::AttrIntfType, attribute::AttrIntfNum)> {
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
    pub fn parse_res_ex(
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

/// Session to a specified resource
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Instrument(OwnedSs);

fn map_to_io_err(err: Error) -> std::io::Error {
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
    ///Manually flushes the specified buffers associated with formatted I/O operations and/or serial communication.
    pub fn visa_flush(&self, mode: flags::FlushMode) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viFlush(self.as_raw_ss(), mode.bits()))?;
        Ok(())
    }
    /// Returns a user-readable description of the status code passed to the operation.
    pub fn status_desc(&self, error: Error) -> Result<VisaString> {
        let mut desc: VisaBuf = new_visa_buf();
        wrap_raw_error_in_unsafe!(vs::viStatusDesc(
            self.as_raw_ss(),
            error.into(),
            desc.as_mut_ptr() as _
        ))?;
        Ok(desc.try_into().unwrap())
    }
    /// Establishes an access mode to the specified resources.
    ///
    /// This operation is used to obtain a lock on the specified resource. The caller can specify the type of lock requested—exclusive or shared lock—and the length of time the operation will suspend while waiting to acquire the lock before timing out. This operation can also be used for sharing and nesting locks.
    ///
    /// The session that gained a shared lock can pass the accessKey to other sessions for the purpose of sharing the lock. The session wanting to join the group of sessions sharing the lock can use the key as an input value to the requestedKey parameter. VISA will add the session to the list of sessions sharing the lock, as long as the requestedKey value matches the accessKey value for the particular resource. The session obtaining a shared lock in this manner will then have the same access privileges as the original session that obtained the lock.
    ///
    ///It is also possible to obtain nested locks through this operation. To acquire nested locks, invoke the viLock() operation with the same lock type as the previous invocation of this operation. For each session, viLock() and viUnlock() share a lock count, which is initialized to 0. Each invocation of viLock() for the same session (and for the same lockType) increases the lock count. In the case of a shared lock, it returns with the same accessKey every time. When a session locks the resource a multiple number of times, it is necessary to invoke the viUnlock() operation an equal number of times in order to unlock the resource. That is, the lock count increments for each invocation of viLock(), and decrements for each invocation of viUnlock(). A resource is actually unlocked only when the lock count is 0.
    ///
    ///The VISA locking mechanism enforces arbitration of accesses to resources on an individual basis. If a session locks a resource, operations invoked by other sessions to the same resource are serviced or returned with a locking error, depending on the operation and the type of lock used. If a session has an exclusive lock, other sessions cannot modify global attributes or invoke operations, but can still get attributes and set local attributes. If the session has a shared lock, other sessions that have shared locks can also modify global attributes and invoke operations. Regardless of which type of lock a session has, if the session is closed without first being unlocked, VISA automatically performs a viUnlock() on that session.
    ///
    ///The locking mechanism works for all processes and resources existing on the same computer. When using remote resources, however, the networking protocol may not provide the ability to pass lock requests to the remote device or resource. In this case, locks will behave as expected from multiple sessions on the same computer, but not necessarily on the remote device. For example, when using the VXI-11 protocol, exclusive lock requests can be sent to a device, but shared locks can only be handled locally.
    ///
    /// see also [`Self::lock_exclusive`], [`Self::lock_shared`] and [`Self::lock_shared_with_key`]
    ///
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

    ///Relinquishes a lock for the specified resource.
    pub fn unlock(&self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viUnlock(self.as_raw_ss()))?;
        Ok(())
    }

    ///Enables notification of a specified event.
    ///
    ///The specified session can be enabled to queue events by specifying VI_QUEUE. Applications can enable the session to invoke a callback function to execute the handler by specifying VI_HNDLR. The applications are required to install at least one handler to be enabled for this mode. Specifying VI_SUSPEND_HNDLR enables the session to receive callbacks, but the invocation of the handler is deferred to a later time. Successive calls to this operation replace the old callback mechanism with the new callback mechanism.
    ///
    ///Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter refers to all events which have previously been enabled on this session, making it easier to switch between the two callback mechanisms for multiple events.
    ///
    /// NI-VISA does not support enabling both the queue and the handler for the same event type on the same session. If you need to use both mechanisms for the same event type, you should open multiple sessions to the resource.
    pub fn enable_event(
        &self,
        event_kind: event::EventKind,
        mechanism: event::Mechanism,
    ) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viEnableEvent(
            self.as_raw_ss(),
            event_kind as _,
            mechanism as _,
            event::EventFilter::Null as _
        ))?;
        Ok(())
    }

    /// Disables notification of the specified event type(s) via the specified mechanism(s).
    ///
    /// The viDisableEvent() operation disables servicing of an event identified by the eventType parameter for the mechanisms specified in the mechanism parameter. This operation prevents new event occurrences from being added to the queue(s). However, event occurrences already existing in the queue(s) are not flushed. Use viDiscardEvents() if you want to discard events remaining in the queue(s).
    ///
    ///Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter allows a session to stop receiving all events. The session can stop receiving queued events by specifying VI_QUEUE. Applications can stop receiving callback events by specifying either VI_HNDLR or VI_SUSPEND_HNDLR. Specifying VI_ALL_MECH disables both the queuing and callback mechanisms.
    ///
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
    /// Discards event occurrences for specified event types and mechanisms in a session.
    ///
    /// The viDiscardEvents() operation discards all pending occurrences of the specified event types and mechanisms from the specified session. Specifying VI_ALL_ENABLED_EVENTS for the eventType parameter discards events of every type that is enabled for the given session.
    ///
    /// The information about all the event occurrences which have not yet been handled is discarded. This operation is useful to remove event occurrences that an application no longer needs. The discarded event occurrences are not available to a session at a later time.
    ///
    ///  This operation does not apply to event contexts that have already been delivered to the application.
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
    /// Waits for an occurrence of the specified event for a given session.
    ///
    /// The viWaitOnEvent() operation suspends the execution of a thread of an application and waits for an event of the type specified by inEventType for a time period specified by timeout. You can wait only for events that have been enabled with the viEnableEvent() operation. Refer to individual event descriptions for context definitions. If the specified inEventType is VI_ALL_ENABLED_EVENTS, the operation waits for any event that is enabled for the given session. If the specified timeout value is VI_TMO_INFINITE, the operation is suspended indefinitely. If the specified timeout value is VI_TMO_IMMEDIATE, the operation is not suspended; therefore, this value can be used to dequeue events from an event queue.
    ///
    ///When the outContext handle returned from a successful invocation of viWaitOnEvent() is no longer needed, it should be passed to viClose().
    ///
    ///If a session's event queue becomes full and a new event arrives, the new event is discarded. The default event queue size (per session) is 50, which is sufficiently large for most  applications. If an application expects more than 50 events to arrive without having been handled, it can modify the value of the attribute VI_ATTR_MAX_QUEUE_LENGTH to the required size.
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
    /// Installs handlers for event callbacks.
    ///
    /// The viInstallHandler() operation allows applications to install handlers on sessions. The handler specified in the handler parameter is installed along with any previously installed handlers for the specified event.
    ///
    /// VISA allows applications to install multiple handlers for an eventType on the same session. You can install multiple handlers through multiple invocations of the viInstallHandler() operation, where each invocation adds to the previous list of handlers. If more than one handler is installed for an eventType, each of the handlers is invoked on every occurrence of the specified event(s). VISA specifies that the handlers are invoked in Last In First Out (LIFO) order.
    ///
    /// *Note*: for some reason pass a closure with signature `|instr, event|{...}`
    /// may get error. Instead, use `|instr: & Instrument, event: & Event|{...}`.
    ///

    pub fn install_handler<F: handler::Callback>(
        &self,
        event_kind: event::EventKind,
        callback: F,
    ) -> Result<handler::Handler<'_, F>> {
        handler::Handler::new(self.as_ss(), event_kind, callback)
    }

    /// Reads a status byte of the service request.
    ///
    /// The IEEE 488.2 standard defines several bit assignments in the status byte. For example, if bit 6 of the status is set, the device is requesting service. In addition to setting bit 6 when requesting service, 488.2 devices also use two other bits to specify their status. Bit 4, the Message Available bit (MAV), is set when the device is ready to send previously queried data. Bit 5, the Event Status bit (ESB), is set if one or more of the enabled 488.2 events occurs. These events include power-on, user request, command error, execution error, device dependent error, query error, request control, and operation complete. The device can assert SRQ when ESB or MAV are set, or when a manufacturer-defined condition occurs. Manufacturers of 488.2 devices use the remaining lower-order bits to communicate the reason for the service request or to summarize the device state.
    ///
    pub fn read_stb(&self) -> Result<u16> {
        let mut stb = 0;
        wrap_raw_error_in_unsafe!(vs::viReadSTB(self.as_raw_ss(), &mut stb as *mut _))?;
        Ok(stb)
    }

    /// The viClear() operation clears the device input and output buffers.
    pub fn clear(&self) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viClear(self.as_raw_ss()))?;
        Ok(())
    }
}

mod async_io;

impl Instrument {
    ///Reads data from device or interface asynchronously.
    ///
    /// The viReadAsync() operation asynchronously transfers data. The data read is to be stored in the buffer represented by buf. This operation normally returns before the transfer terminates.
    ///
    ///Before calling this operation, you should enable the session for receiving I/O completion events. After the transfer has completed, an I/O completion event is posted.
    ///
    ///The operation returns jobId, which you can use with either viTerminate() to abort the operation, or with an I/O completion event to identify which asynchronous read operation completed. VISA will never return VI_NULL for a valid jobID.
    ///
    /// If you have enabled VI_EVENT_IO_COMPLETION for queueing (VI_QUEUE), for each successful call to viReadAsync(), you must call viWaitOnEvent() to retrieve the I/O completion event. This is true even if the I/O is done synchronously (that is, if the operation returns VI_SUCCESS_SYNC).
    /// # Safety
    /// This function is unsafe because the `buf` passed in may be dropped before the transfer terminates

    //todo: return VI_SUCCESS_SYNC, means IO operation has finished, so if there is a waker receiving JobID, would be called before JobID set and can't wake corresponding job
    pub unsafe fn visa_read_async(&self, buf: &mut [u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        #[allow(unused_unsafe)]
        wrap_raw_error_in_unsafe!(vs::viReadAsync(
            self.as_raw_ss(),
            buf.as_mut_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }

    ///The viWriteAsync() operation asynchronously transfers data. The data to be written is in the buffer represented by buf. This operation normally returns before the transfer terminates.
    ///
    ///Before calling this operation, you should enable the session for receiving I/O completion events. After the transfer has completed, an I/O completion event is posted.
    ///
    ///The operation returns a job identifier that you can use with either viTerminate() to abort the operation or with an I/O completion event to identify which asynchronous write operation completed. VISA will never return VI_NULL for a valid jobId.
    ///
    /// If you have enabled VI_EVENT_IO_COMPLETION for queueing (VI_QUEUE), for each successful call to viWriteAsync(), you must call viWaitOnEvent() to retrieve the I/O completion event. This is true even if the I/O is done synchronously (that is, if the operation returns VI_SUCCESS_SYNC).
    ///
    /// # Safety
    /// This function is unsafe because the `buf` passed in may be dropped before the transfer terminates

    //todo: return VI_SUCCESS_SYNC, means IO operation has finished, so if there is a waker receiving JobID, would be called before JobID set and can't wake corresponding job

    pub unsafe fn visa_write_async(&self, buf: &[u8]) -> Result<JobID> {
        let mut id: vs::ViJobId = 0;
        #[allow(unused_unsafe)]
        wrap_raw_error_in_unsafe!(vs::viWriteAsync(
            self.as_raw_ss(),
            buf.as_ptr(),
            buf.len() as _,
            &mut id as _
        ))?;
        Ok(JobID(id))
    }

    /// Requests session to terminate normal execution of an operation.
    ///
    /// This operation is used to request a session to terminate normal execution of an operation, as specified by the jobId parameter. The jobId parameter is a unique value generated from each call to an asynchronous operation.
    ///
    ///If a user passes VI_NULL as the jobId value to viTerminate(), VISA will abort any calls in the current process executing on the specified vi. Any call that is terminated this way should return VI_ERROR_ABORT. Due to the nature of multi-threaded systems, for example where operations in other threads may complete normally before the operation viTerminate() has any effect, the specified return value is not guaranteed.
    ///
    pub fn terminate(&self, job_id: JobID) -> Result<()> {
        wrap_raw_error_in_unsafe!(vs::viTerminate(
            self.as_raw_ss(),
            vs::VI_NULL as _,
            job_id.0
        ))?;
        Ok(())
    }
    /// Safe rust wrapper of [`Self::visa_read_async`]
    ///
    /// *Note*: for now this function returns a future holding reference of `buf` and `Self`,
    /// which means it can't be send to another thread
    pub async fn async_read(&self, buf: &mut [u8]) -> Result<usize> {
        async_io::AsyncRead::new(self, buf).await
    }
    /// Safe rust wrapper of [`Self::visa_write_async`]
    ///
    /// *Note*: for now this function returns a future holding reference of `buf` and `Self`,
    /// which means it can't be send to another thread
    pub async fn async_write(&self, buf: &[u8]) -> Result<usize> {
        async_io::AsyncWrite::new(self, buf).await
    }
}

/// Job ID of a asynchronous operation,
///
/// Returned by [`Instrument::visa_read_async`] or [`Instrument::visa_write_async`], to compare with the attribute [AttrJobId](enums::attribute::AttrJobId) get from [Event](enums::event::Event) to distinguish operations
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub struct JobID(vs::ViJobId);

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
