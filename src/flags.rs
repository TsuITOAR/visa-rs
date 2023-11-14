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

    /// Used in [`Instrument::visa_flush`](crate::Instrument::visa_flush), specifies the action to be taken with flushing the buffer.
    ///
    ///It is possible to combine any of these read flags and write flags for different buffers by ORing the flags. However, combining two flags for the same buffer in the same call to viFlush() is illegal.
    ///
    ///Notice that when using formatted I/O operations with a session to a Serial device or Ethernet socket, a flush of the formatted I/O buffers also causes the corresponding I/O communication buffers to be flushed. For example, calling viFlush() with VI_WRITE_BUF also flushes the VI_IO_OUT_BUF.
    ///
    /// Although you can explicitly flush the buffers by making a call to viFlush(), the buffers are flushed implicitly under some conditions. These conditions vary for the viPrintf() and viScanf() operations.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FlushMode: vs::ViUInt16  {
        /// Discard the read buffer contents. If data was present in the read buffer and no END-indicator was present, read from the device until encountering an END indicator (which causes the loss of data). This action resynchronizes the next viScanf() call to read a \<TERMINATED RESPONSE MESSAGE\>. (Refer to the IEEE 488.2 standard.)
        const READ_BUF = vs::VI_READ_BUF as _ ;
        /// Discard the read buffer contents (does not perform any I/O to the device).
        const READ_BUF_DISCARD = vs::VI_READ_BUF_DISCARD as _ ;
        /// Flush the write buffer by writing all buffered data to the device.
        const WRITE_BUF = vs::VI_WRITE_BUF as _;
        /// Discard the write buffer contents (does not perform any I/O to the device).
        const WRITE_BUF_DISCARD  = vs::VI_WRITE_BUF_DISCARD as _;
        /// Discard the low-level I/O receive buffer contents (same as VI_IO_IN_BUF_DISCARD).
        const IO_IN_BUF = vs::VI_IO_IN_BUF as _;
        /// Discard the low-level I/O receive buffer contents (does not perform any I/O to the device).
        const IO_IN_BUF_DISCARD = vs::VI_IO_IN_BUF_DISCARD as _;
        /// Flush the low-level I/O transmit buffer by writing all buffered data to the device.
        const IO_OUT_BUF = vs::VI_IO_OUT_BUF as _;
        /// Discard the low-level I/O transmit buffer contents (does not perform any I/O to the device).
        const IO_OUT_BUF_DISCARD = vs::VI_IO_OUT_BUF_DISCARD as _;
    }
}

bitflags! {
    /// Used in [`Instrument::set_buf`](crate::Instrument::set_buf), specifies the type of buffer.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BufMask: vs::ViUInt16  {
        /// Formatted I/O read buffer.
        const READ_BUF = vs::VI_READ_BUF as _ ;
        /// Formatted I/O write buffer.
        const WRITE_BUF = vs::VI_WRITE_BUF as _  ;
        /// Low-level I/O receive buffer.
        const IO_IN_BUF = vs::VI_IO_IN_BUF as _;
        /// Low-level I/O transmit buffer.
        const IO_OUT_BUF = vs::VI_IO_OUT_BUF as _;
    }
}
