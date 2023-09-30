//!
//! Defines [`AccessMode`] and [`FlushMode`]
//!
//!

use bitflags::bitflags;
use visa_sys as vs;

bitflags! {
    /// Used in [`DefaultRM::open`](crate::DefaultRM::open) and [`Instrument::lock`](crate::Instrument::lock), specifies the type of lock requested
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct AccessMode: vs::ViAccessMode  {
        const NO_LOCK = vs::VI_NO_LOCK;
        const EXCLUSIVE_LOCK = vs::VI_EXCLUSIVE_LOCK;
        const SHARED_LOCK = vs::VI_SHARED_LOCK;
        const LOAD_CONFIG = vs::VI_LOAD_CONFIG;
    }
}

impl Default for AccessMode {
    fn default() -> Self {
        Self::NO_LOCK
    }
}

bitflags! {

    /// Used in [`Instrument::visa_flush`](crate::eInstrument::visa_flush), specifies the action to be taken with flushing the buffer.
    ///
    ///It is possible to combine any of these read flags and write flags for different buffers by ORing the flags. However, combining two flags for the same buffer in the same call to viFlush() is illegal.
    ///
    ///Notice that when using formatted I/O operations with a session to a Serial device or Ethernet socket, a flush of the formatted I/O buffers also causes the corresponding I/O communication buffers to be flushed. For example, calling viFlush() with VI_WRITE_BUF also flushes the VI_IO_OUT_BUF.
    ///
    /// Although you can explicitly flush the buffers by making a call to viFlush(), the buffers are flushed implicitly under some conditions. These conditions vary for the viPrintf() and viScanf() operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FlushMode: vs::ViUInt16  {
        const READ_BUF = vs::VI_READ_BUF as _ ;
        const READ_BUF_DISCARD = vs::VI_READ_BUF_DISCARD as _ ;
        const WRITE_BUF = vs::VI_WRITE_BUF as _;
        const WRITE_BUF_DISCARD  = vs::VI_WRITE_BUF_DISCARD as _;
        const IO_IN_BUF = vs::VI_IO_IN_BUF as _;
        const IO_IN_BUF_DISCARD = vs::VI_IO_IN_BUF_DISCARD as _;
        const IO_OUT_BUF = vs::VI_IO_OUT_BUF as _;
        const IO_OUT_BUF_DISCARD = vs::VI_IO_OUT_BUF_DISCARD as _;
    }
}
