//! samples of expanded macros
//! 
//! ```
//! #[repr(u32)]
//! pub enum AttrKind {
//!     AttrRsrcClass = 0xBFFF0001 as _,
//! }
//! 
//! #[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
//! pub enum Attribute {
//!     ///VI_ATTR_4882_COMPLIANT specifies whether the device is 488.2 compliant.
//!     Attr4882Compliant(Attr4882Compliant),
//! }
//! 
//! impl Attribute {
//!     pub(crate) unsafe fn from_kind(kind: AttrKind) -> Self {
//!         match kind {
//!             AttrKind::Attr4882Compliant => Self::from(Attr4882Compliant::zero()),
//!         }
//!     }
//! 
//!     pub(crate) fn inner_c_void(&mut self) -> *mut ::std::ffi::c_void {
//!         match self {
//!             Self::Attr4882Compliant(s) => s.inner_c_void(),
//!         }
//!     }
//! 
//!     pub fn kind(&self) -> AttrKind {
//!         match self {
//!             Self::Attr4882Compliant(s) => super::AttrInner::kind(s),
//!         }
//!     }
//! 
//!     pub(crate) fn as_u64(&self) -> u64 {
//!         match self {
//!             Self::Attr4882Compliant(s) => s.value as _,
//!         }
//!     }
//! }
//! 
//! ///VI_ATTR_4882_COMPLIANT specifies whether the device is 488.2 compliant.
//! pub struct Attr4882Compliant {
//!     value: vs::ViBoolean,
//! }
//! 
//! impl Attr4882Compliant {
//!     pub(crate) fn inner_mut(&mut self) -> &mut vs::ViBoolean {
//!         &mut self.value
//!     }
//!     pub(crate) fn inner_c_void(&mut self) -> *mut ::std::ffi::c_void {
//!         self.inner_mut() as *mut _ as _
//!     }
//! }
//! impl Attr4882Compliant {
//!     pub const VI_TRUE: Self = Self { value: 1 as _ };
//!     pub const VI_FALSE: Self = Self { value: 0 as _ };
//!     pub unsafe fn new_unchecked(value: vs::ViBoolean) -> Self {
//!         Self { value }
//!     }
//!     #[allow(unused_parens)]
//!     pub fn new_checked(value: vs::ViBoolean) -> Option<Self> {
//!         if 1 as vs::ViBoolean == value || 0 as vs::ViBoolean == value {
//!             Some(Self { value })
//!         } else {
//!             None
//!         }
//!     }
//! }
//! impl super::AttrInner for Attr4882Compliant {
//!     fn kind(&self) -> AttrKind {
//!         AttrKind::Attr4882Compliant
//!     }
//! }
//! impl Attr4882Compliant {
//!     unsafe fn zero() -> Self {
//!         Self { value: 0 as _ }
//!     }
//! }
//! ```




use crate::{wrap_raw_error_in_unsafe, Result};

pub use attributes::{AttrKind, Attribute};
use visa_sys as vs;
pub trait HasAttribute: crate::session::AsRawSs {
    fn get_attr(&self, attr_kind: AttrKind) -> Result<Attribute> {
        let mut attr = unsafe { Attribute::from_kind(attr_kind) };
        wrap_raw_error_in_unsafe!(vs::viGetAttribute(
            self.as_raw_ss(),
            attr_kind as _,
            attr.inner_c_void()
        ))?;
        Ok(attr)
    }
    fn set_attr(&self, attr: impl Into<Attribute>) -> Result<()> {
        let attr: Attribute = attr.into();
        wrap_raw_error_in_unsafe!(vs::viSetAttribute(
            self.as_raw_ss(),
            attr.kind() as _,
            attr.as_u64()
        ))?;
        Ok(())
    }
}

impl HasAttribute for crate::event::Event {}
impl HasAttribute for crate::Instrument {}
impl HasAttribute for crate::DefaultRM {}

pub trait AttrInner {
    fn kind(&self) -> AttrKind;
}

impl<T: AttrInner> PartialEq<T> for AttrKind {
    fn eq(&self, other: &T) -> bool {
        self.eq(&other.kind())
    }
}

mod attributes {
    #![allow(overflowing_literals)]
    use visa_sys as vs;
    // todo: add description and range check
    consts_to_enum! {
        pub enum AttrKind: u32 {
            VI_ATTR_RSRC_CLASS	0xBFFF0001
            VI_ATTR_RSRC_NAME	0xBFFF0002
            VI_ATTR_RSRC_IMPL_VERSION	0x3FFF0003
            VI_ATTR_RSRC_LOCK_STATE	0x3FFF0004
            VI_ATTR_MAX_QUEUE_LENGTH	0x3FFF0005
            VI_ATTR_USER_DATA	0x3FFF0007
            VI_ATTR_FDC_CHNL	0x3FFF000D
            VI_ATTR_FDC_MODE	0x3FFF000F
            VI_ATTR_FDC_GEN_SIGNAL_EN	0x3FFF0011
            VI_ATTR_FDC_USE_PAIR	0x3FFF0013
            VI_ATTR_SEND_END_EN	0x3FFF0016
            VI_ATTR_TERMCHAR	0x3FFF0018
            VI_ATTR_TMO_VALUE	0x3FFF001A
            VI_ATTR_GPIB_READDR_EN	0x3FFF001B
            VI_ATTR_IO_PROT	0x3FFF001C
            VI_ATTR_DMA_ALLOW_EN	0x3FFF001E
            VI_ATTR_ASRL_BAUD	0x3FFF0021
            VI_ATTR_ASRL_DATA_BITS	0x3FFF0022
            VI_ATTR_ASRL_PARITY	0x3FFF0023
            VI_ATTR_ASRL_STOP_BITS	0x3FFF0024
            VI_ATTR_ASRL_FLOW_CNTRL	0x3FFF0025
            VI_ATTR_RD_BUF_OPER_MODE	0x3FFF002A
            VI_ATTR_RD_BUF_SIZE	0x3FFF002B
            VI_ATTR_WR_BUF_OPER_MODE	0x3FFF002D
            VI_ATTR_WR_BUF_SIZE	0x3FFF002E
            VI_ATTR_SUPPRESS_END_EN	0x3FFF0036
            VI_ATTR_TERMCHAR_EN	0x3FFF0038
            VI_ATTR_DEST_ACCESS_PRIV	0x3FFF0039
            VI_ATTR_DEST_BYTE_ORDER	0x3FFF003A
            VI_ATTR_SRC_ACCESS_PRIV	0x3FFF003C
            VI_ATTR_SRC_BYTE_ORDER	0x3FFF003D
            VI_ATTR_SRC_INCREMENT	0x3FFF0040
            VI_ATTR_DEST_INCREMENT	0x3FFF0041
            VI_ATTR_WIN_ACCESS_PRIV	0x3FFF0045
            VI_ATTR_WIN_BYTE_ORDER	0x3FFF0047
            VI_ATTR_GPIB_ATN_STATE	0x3FFF0057
            VI_ATTR_GPIB_ADDR_STATE	0x3FFF005C
            VI_ATTR_GPIB_CIC_STATE	0x3FFF005E
            VI_ATTR_GPIB_NDAC_STATE	0x3FFF0062
            VI_ATTR_GPIB_SRQ_STATE	0x3FFF0067
            VI_ATTR_GPIB_SYS_CNTRL_STATE	0x3FFF0068
            VI_ATTR_GPIB_HS488_CBL_LEN	0x3FFF0069
            VI_ATTR_CMDR_LA	0x3FFF006B
            VI_ATTR_VXI_DEV_CLASS	0x3FFF006C
            VI_ATTR_MAINFRAME_LA	0x3FFF0070
            VI_ATTR_MANF_NAME	0xBFFF0072
            VI_ATTR_MODEL_NAME	0xBFFF0077
            VI_ATTR_VXI_VME_INTR_STATUS	0x3FFF008B
            VI_ATTR_VXI_TRIG_STATUS	0x3FFF008D
            VI_ATTR_VXI_VME_SYSFAIL_STATE	0x3FFF0094
            VI_ATTR_WIN_BASE_ADDR	0x3FFF0098
            VI_ATTR_WIN_SIZE	0x3FFF009A
            VI_ATTR_ASRL_AVAIL_NUM	0x3FFF00AC
            VI_ATTR_MEM_BASE	0x3FFF00AD
            VI_ATTR_ASRL_CTS_STATE	0x3FFF00AE
            VI_ATTR_ASRL_DCD_STATE	0x3FFF00AF
            VI_ATTR_ASRL_DSR_STATE	0x3FFF00B1
            VI_ATTR_ASRL_DTR_STATE	0x3FFF00B2
            VI_ATTR_ASRL_END_IN	0x3FFF00B3
            VI_ATTR_ASRL_END_OUT	0x3FFF00B4
            VI_ATTR_ASRL_REPLACE_CHAR	0x3FFF00BE
            VI_ATTR_ASRL_RI_STATE	0x3FFF00BF
            VI_ATTR_ASRL_RTS_STATE	0x3FFF00C0
            VI_ATTR_ASRL_XON_CHAR	0x3FFF00C1
            VI_ATTR_ASRL_XOFF_CHAR	0x3FFF00C2
            VI_ATTR_WIN_ACCESS	0x3FFF00C3
            VI_ATTR_RM_SESSION	0x3FFF00C4
            VI_ATTR_VXI_LA	0x3FFF00D5
            VI_ATTR_MANF_ID	0x3FFF00D9
            VI_ATTR_MEM_SIZE	0x3FFF00DD
            VI_ATTR_MEM_SPACE	0x3FFF00DE
            VI_ATTR_MODEL_CODE	0x3FFF00DF
            VI_ATTR_SLOT	0x3FFF00E8
            VI_ATTR_INTF_INST_NAME	0xBFFF00E9
            VI_ATTR_IMMEDIATE_SERV	0x3FFF0100
            VI_ATTR_INTF_PARENT_NUM	0x3FFF0101
            VI_ATTR_RSRC_SPEC_VERSION	0x3FFF0170
            VI_ATTR_INTF_TYPE	0x3FFF0171
            VI_ATTR_GPIB_PRIMARY_ADDR	0x3FFF0172
            VI_ATTR_GPIB_SECONDARY_ADDR	0x3FFF0173
            VI_ATTR_RSRC_MANF_NAME	0xBFFF0174
            VI_ATTR_RSRC_MANF_ID	0x3FFF0175
            VI_ATTR_INTF_NUM	0x3FFF0176
            VI_ATTR_TRIG_ID	0x3FFF0177
            VI_ATTR_GPIB_REN_STATE	0x3FFF0181
            VI_ATTR_GPIB_UNADDR_EN	0x3FFF0184
            VI_ATTR_DEV_STATUS_BYTE	0x3FFF0189
            VI_ATTR_FILE_APPEND_EN	0x3FFF0192
            VI_ATTR_VXI_TRIG_SUPPORT	0x3FFF0194
            VI_ATTR_TCPIP_ADDR	0xBFFF0195
            VI_ATTR_TCPIP_HOSTNAME	0xBFFF0196
            VI_ATTR_TCPIP_PORT	0x3FFF0197
            VI_ATTR_TCPIP_DEVICE_NAME	0xBFFF0199
            VI_ATTR_TCPIP_NODELAY	0x3FFF019A
            VI_ATTR_TCPIP_KEEPALIVE	0x3FFF019B
            VI_ATTR_4882_COMPLIANT	0x3FFF019F
            VI_ATTR_USB_SERIAL_NUM	0xBFFF01A0
            VI_ATTR_USB_INTFC_NUM	0x3FFF01A1
            VI_ATTR_USB_PROTOCOL	0x3FFF01A7
            VI_ATTR_USB_MAX_INTR_SIZE	0x3FFF01AF
            VI_ATTR_JOB_ID	0x3FFF4006
            VI_ATTR_EVENT_TYPE	0x3FFF4010
            VI_ATTR_SIGP_STATUS_ID	0x3FFF4011
            VI_ATTR_RECV_TRIG_ID	0x3FFF4012
            VI_ATTR_INTR_STATUS_ID	0x3FFF4023
            VI_ATTR_STATUS	0x3FFF4025
            VI_ATTR_RET_COUNT	0x3FFF4026
            VI_ATTR_BUFFER	0x3FFF4027
            VI_ATTR_RECV_INTR_LEVEL	0x3FFF4041
            VI_ATTR_OPER_NAME	0xBFFF4042
            VI_ATTR_GPIB_RECV_CIC_STATE	0x3FFF4193
            VI_ATTR_RECV_TCPIP_ADDR	0xBFFF4198
            VI_ATTR_USB_RECV_INTR_SIZE	0x3FFF41B0
            VI_ATTR_USB_RECV_INTR_DATA	0xBFFF41B1
            VI_ATTR_PXI_DEV_NUM	0x3FFF0201
            VI_ATTR_PXI_FUNC_NUM	0x3FFF0202
            VI_ATTR_PXI_BUS_NUM	0x3FFF0205
            VI_ATTR_PXI_CHASSIS	0x3FFF0206
            VI_ATTR_PXI_SLOTPATH	0xBFFF0207
            VI_ATTR_PXI_SLOT_LBUS_LEFT	0x3FFF0208
            VI_ATTR_PXI_SLOT_LBUS_RIGHT	0x3FFF0209
            VI_ATTR_PXI_TRIG_BUS	0x3FFF020A
            VI_ATTR_PXI_STAR_TRIG_BUS	0x3FFF020B
            VI_ATTR_PXI_STAR_TRIG_LINE	0x3FFF020C
            VI_ATTR_PXI_MEM_TYPE_BAR0	0x3FFF0211
            VI_ATTR_PXI_MEM_TYPE_BAR1	0x3FFF0212
            VI_ATTR_PXI_MEM_TYPE_BAR2	0x3FFF0213
            VI_ATTR_PXI_MEM_TYPE_BAR3	0x3FFF0214
            VI_ATTR_PXI_MEM_TYPE_BAR4	0x3FFF0215
            VI_ATTR_PXI_MEM_TYPE_BAR5	0x3FFF0216
            VI_ATTR_PXI_MEM_BASE_BAR0_32	0x3FFF0221
            VI_ATTR_PXI_MEM_BASE_BAR1_32	0x3FFF0222
            VI_ATTR_PXI_MEM_BASE_BAR2_32	0x3FFF0223
            VI_ATTR_PXI_MEM_BASE_BAR3_32	0x3FFF0224
            VI_ATTR_PXI_MEM_BASE_BAR4_32	0x3FFF0225
            VI_ATTR_PXI_MEM_BASE_BAR5_32	0x3FFF0226
            VI_ATTR_PXI_MEM_SIZE_BAR0_32	0x3FFF0231
            VI_ATTR_PXI_MEM_SIZE_BAR1_32	0x3FFF0232
            VI_ATTR_PXI_MEM_SIZE_BAR2_32	0x3FFF0233
            VI_ATTR_PXI_MEM_SIZE_BAR3_32	0x3FFF0234
            VI_ATTR_PXI_MEM_SIZE_BAR4_32	0x3FFF0235
            VI_ATTR_PXI_MEM_SIZE_BAR5_32	0x3FFF0236
            VI_ATTR_PXI_IS_EXPRESS	0x3FFF0240
            VI_ATTR_PXI_SLOT_LWIDTH	0x3FFF0241
            VI_ATTR_PXI_MAX_LWIDTH	0x3FFF0242
            VI_ATTR_PXI_ACTUAL_LWIDTH	0x3FFF0243
            VI_ATTR_PXI_DSTAR_BUS	0x3FFF0244
            VI_ATTR_PXI_DSTAR_SET	0x3FFF0245
            VI_ATTR_TCPIP_SERVER_CERT_ISSUER_NAME	0xBFFF0270
            VI_ATTR_TCPIP_SERVER_CERT_SUBJECT_NAME	0xBFFF0271
            VI_ATTR_TCPIP_SERVER_CERT_EXPIRATION_DATE	0xBFFF0272
            VI_ATTR_TCPIP_SERVER_CERT_IS_PERPETUAL	0x3FFF0273
            VI_ATTR_TCPIP_SASL_MECHANISM	0xBFFF0274
            VI_ATTR_TCPIP_TLS_CIPHER_SUITE	0xBFFF0275
            VI_ATTR_TCPIP_HISLIP_OVERLAP_EN	0x3FFF0300
            VI_ATTR_TCPIP_HISLIP_VERSION	0x3FFF0301
            VI_ATTR_TCPIP_HISLIP_MAX_MESSAGE_KB	0x3FFF0302
            VI_ATTR_TCPIP_IS_HISLIP	0x3FFF0303
            VI_ATTR_TCPIP_HISLIP_ENCRYPTION_EN	0x3FFF0304
            VI_ATTR_PXI_RECV_INTR_SEQ	0x3FFF4240
            VI_ATTR_PXI_RECV_INTR_DATA	0x3FFF4241
            VI_ATTR_PXI_SRC_TRIG_BUS	0x3FFF020D
            VI_ATTR_PXI_DEST_TRIG_BUS	0x3FFF020E
            VI_ATTR_PXI_MEM_BASE_BAR0_64	0x3FFF0228
            VI_ATTR_PXI_MEM_BASE_BAR1_64	0x3FFF0229
            VI_ATTR_PXI_MEM_BASE_BAR2_64	0x3FFF022A
            VI_ATTR_PXI_MEM_BASE_BAR3_64	0x3FFF022B
            VI_ATTR_PXI_MEM_BASE_BAR4_64	0x3FFF022C
            VI_ATTR_PXI_MEM_BASE_BAR5_64	0x3FFF022D
            VI_ATTR_PXI_MEM_SIZE_BAR0_64	0x3FFF0238
            VI_ATTR_PXI_MEM_SIZE_BAR1_64	0x3FFF0239
            VI_ATTR_PXI_MEM_SIZE_BAR2_64	0x3FFF023A
            VI_ATTR_PXI_MEM_SIZE_BAR3_64	0x3FFF023B
            VI_ATTR_PXI_MEM_SIZE_BAR4_64	0x3FFF023C
            VI_ATTR_PXI_MEM_SIZE_BAR5_64	0x3FFF023D
            VI_ATTR_PXI_ALLOW_WRITE_COMBINE	0x3FFF0246
        }
    }
    visa_rs_proc::visa_attrs! {
        pub struct Attribute{

            const VI_ATTR_4882_COMPLIANT: r#"VI_ATTR_4882_COMPLIANT specifies whether the device is 488.2 compliant."#
            (Read Only Global) ( ViBoolean) [static as N/A in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_ASRL_AVAIL_NUM: r#"VI_ATTR_ASRL_AVAIL_NUM shows the number of bytes available in the low-level I/O receive buffer."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_ASRL_BAUD: r#"VI_ATTR_ASRL_BAUD is the baud rate of the interface. It is represented as an unsigned 32-bit integer so that any baud rate can be used, but it usually requires a commonly used rate such as 300, 1200, 2400, or 9600 baud."#
            (Read/Write Global) ( ViUInt32) [static as 9600 in 0 to FFFFFFFFh]

            const VI_ATTR_ASRL_CTS_STATE: r#"VI_ATTR_ASRL_CTS_STATE shows the current state of the Clear To Send (CTS) input signal."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_DATA_BITS: r#"VI_ATTR_ASRL_DATA_BITS is the number of data bits contained in each frame (from 5 to 8). The data bits for each frame are located in the low-order bits of every byte stored in memory."#
            (Read/Write Global) ( ViUInt16) [static as 8 in 5 to 8]

            const VI_ATTR_ASRL_DCD_STATE: r#"VI_ATTR_ASRL_DCD_STATE represents the current state of the Data Carrier Detect (DCD) input signal. The DCD signal is often used by modems to indicate the detection of a carrier (remote modem) on the telephone line. The DCD signal is also known as Receive Line Signal Detect (RLSD). This attribute is Read Only except when the VI_ATTR_ASRL_WIRE_MODE attribute is set to VI_ASRL_WIRE_232_DCE , or VI_ASRL_WIRE_232_AUTO with the hardware currently in the DCE state."#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_DSR_STATE: r#"VI_ATTR_ASRL_DSR_STATE shows the current state of the Data Set Ready (DSR) input signal."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_DTR_STATE: r#"VI_ATTR_ASRL_DTR_STATE shows the current state of the Data Terminal Ready (DTR) input signal. When the VI_ATTR_ASRL_FLOW_CNTRL attribute is set to VI_ASRL_FLOW_DTR_DSR , this attribute is Read Only. Querying the value will return VI_STATE_UNKNOWN ."#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_END_IN: r#"VI_ATTR_ASRL_END_IN indicates the method used to terminate read operations. Because the default value of VI_ATTR_TERMCHAR VI_ATTR_ASRL_END_IN or VI_ATTR_TERMCHAR ."#
            (Read/Write Local) ( ViUInt16) [static as VI_ASRL_END_TERMCHAR in VI_ASRL_END_NONE (0) VI_ASRL_END_LAST_BIT (1) VI_ASRL_END_TERMCHAR (2)]

            const VI_ATTR_ASRL_END_OUT: r#"VI_ATTR_ASRL_END_OUT indicates the method used to terminate write operations."#
            (Read/Write Local) ( ViUInt16) [static as VI_ASRL_END_NONE in VI_ASRL_END_NONE (0) VI_ASRL_END_LAST_BIT (1) VI_ASRL_END_TERMCHAR (2) VI_ASRL_END_BREAK (3)]

            const VI_ATTR_ASRL_FLOW_CNTRL: r#"VI_ATTR_ASRL_FLOW_CNTRL indicates the type of flow control used by the transfer mechanism. This attribute can specify multiple flow control mechanisms by bit-ORing multiple values together. However, certain combinations may not be supported by all serial ports and/or operating systems."#
            (Read/Write Global) ( ViUInt16) [static as VI_ASRL_FLOW_NONE in VI_ASRL_FLOW_NONE (0) VI_ASRL_FLOW_XON_XOFF (1) VI_ASRL_FLOW_RTS_CTS (2) VI_ASRL_FLOW_DTR_DSR (4)]

            const VI_ATTR_ASRL_PARITY: r#"VI_ATTR_ASRL_PARITY is the parity used with every frame transmitted and received."#
            (Read/Write Global) ( ViUInt16) [static as VI_ASRL_PAR_NONE in VI_ASRL_PAR_NONE (0) VI_ASRL_PAR_ODD (1) VI_ASRL_PAR_EVEN (2) VI_ASRL_PAR_MARK (3) VI_ASRL_PAR_SPACE (4)]

            const VI_ATTR_ASRL_REPLACE_CHAR: r#"VI_ATTR_ASRL_REPLACE_CHAR specifies the character to be used to replace incoming characters that arrive with errors (such as parity error)."#
            (Read/Write Local) ( ViUInt8) [static as 0 in 0 to FFh]

            const VI_ATTR_ASRL_RI_STATE: r#"VI_ATTR_ASRL_RI_STATE represents the current state of the Ring Indicator (RI) input signal. The RI signal is often used by modems to indicate that the telephone line is ringing. This attribute is Read Only except when the VI_ATTR_ASRL_WIRE_MODE attribute is set to VI_ASRL_WIRE_232_DCE , or VI_ASRL_WIRE_232_AUTO with the hardware currently in the DCE state."#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_RTS_STATE: r#"VI_ATTR_ASRL_RTS_STATE is used to manually assert or unassert the Request To Send (RTS) output signal. When the VI_ATTR_ASRL_FLOW_CNTRL attribute is set to VI_ASRL_FLOW_RTS_CTS , this attribute is Read Only. Querying the value will return VI_STATE_UNKNOWN ."#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN (-1)]

            const VI_ATTR_ASRL_STOP_BITS: r#"VI_ATTR_ASRL_STOP_BITS is the number of stop bits used to indicate the end of a frame. The value VI_ASRL_STOP_ONE5 indicates one-and-one-half (1.5) stop bits."#
            (Read/Write Global) ( ViUInt16) [static as VI_ASRL_STOP_ONE in VI_ASRL_STOP_ONE (10) VI_ASRL_STOP_ONE5 (15) VI_ASRL_STOP_TWO (20)]

            const VI_ATTR_ASRL_XOFF_CHAR: r#"VI_ATTR_ASRL_XOFF_CHAR specifies the value of the XOFF character used for XON/XOFF flow control (both directions). If XON/XOFF flow control (software handshaking) is not being used, the value of this attribute is ignored."#
            (Read/Write Local) ( ViUInt8) [static as <Control-S> (13h) in 0 to FFh]

            const VI_ATTR_ASRL_XON_CHAR: r#"VI_ATTR_ASRL_XON_CHAR specifies the value of the XON character used for XON/XOFF flow control (both directions). If XON/XOFF flow control (software handshaking) is not being used, the value of this attribute is ignored."#
            (Read/Write Local) ( ViUInt8) [static as <Control-Q> (11h) in 0 to FFh]

            const VI_ATTR_BUFFER: r#"VI_ATTR_BUFFER contains the address of a buffer that was used in an asynchronous operation."#
            (Read Only) ( ViBuf) [static as N/A in N/A]

            const VI_ATTR_CMDR_LA: r#"VI_ATTR_CMDR_LA is the unique logical address of the commander of the VXI device used by the given session."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to 255 VI_UNKNOWN_LA (-1)]

            const VI_ATTR_DEST_ACCESS_PRIV: r#"VI_ATTR_DEST_ACCESS_PRIV specifies the address modifier to be used in high-level access operations, such as viOut XX () and viMoveOut XX () , when writing to the destination."#
            (Read/Write Local) ( ViUInt16) [static as VI_DATA_PRIV in VI_DATA_PRIV (0) VI_DATA_NPRIV (1) VI_PROG_PRIV (2) VI_PROG_NPRIV (3) VI_BLCK_PRIV (4) VI_BLCK_NPRIV (5) VI_D64_PRIV (6) VI_D64_NPRIV (7)]

            const VI_ATTR_DEST_BYTE_ORDER: r#"VI_ATTR_DEST_BYTE_ORDER specifies the byte order to be used in high-level access operations, such as viOut XX () and viMoveOut XX () , when writing to the destination."#
            (Read/Write Local) ( ViUInt16) [static as VI_BIG_ENDIAN in VI_BIG_ENDIAN (0) VI_LITTLE_ENDIAN (1)]

            const VI_ATTR_DEST_INCREMENT: r#"VI_ATTR_DEST_INCREMENT is used in the viMoveOut XX () operations to specify by how many elements the destination offset is to be incremented after every transfer. The default value of this attribute is 1 (that is, the destination address will be incremented by 1 after each transfer), and the viMoveOut XX () operations move into consecutive elements. If this attribute is set to 0, the viMoveOut XX () operations will always write to the same element, essentially treating the destination as a FIFO register."#
            (Read/Write Local) ( ViInt32) [static as 1 in 0 to 1]

            const VI_ATTR_DEV_STATUS_BYTE: r#"This attribute specifies the 488-style status byte of the local controller or device associated with this session. If this attribute is written and bit 6 (40h) is set, this device or controller will assert a service request (SRQ) if it is defined for this interface."#
            (Read/Write Global) ( ViUInt8) [static as N/A in 0 to FFh]

            const VI_ATTR_DMA_ALLOW_EN: r#"This attribute specifies whether I/O accesses should use DMA ( VI_TRUE ) or Programmed I/O ( VI_FALSE ). In some implementations, this attribute may have global effects even though it is documented to be a local attribute. Since this affects performance and not functionality, that behavior is acceptable."#
            (Read/Write Local) ( ViBoolean) [static as N/A in VI_TRUE(1) VI_FALSE(0)]

            const VI_ATTR_EVENT_TYPE: r#"VI_ATTR_EVENT_TYPE is the unique logical identifier for the event type of the specified event."#
            (Read Only) ( ViEventType) [static as N/A in 0h to FFFFFFFFh]

            const VI_ATTR_FDC_CHNL: r#"VI_ATTR_FDC_CHNL determines which Fast Data Channel (FDC) will be used to transfer the buffer."#
            (Read/Write Local) ( ViUInt16) [static as N/A in 0 to 7]

            const VI_ATTR_FDC_MODE: r#"VI_ATTR_FDC_MODE specifies which Fast Data Channel (FDC) mode to use (either normal or stream mode)."#
            (Read/Write Local) ( ViUInt16) [static as VI_FDC_NORMAL in VI_FDC_NORMAL (1) VI_FDC_STREAM (2)]

            const VI_ATTR_FDC_USE_PAIR: r#"Setting VI_ATTR_FDC_USE_PAIR to VI_TRUE specifies to use a channel pair for transferring data. Otherwise, only one channel will be used."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_FILE_APPEND_EN: r#"This attribute specifies whether viReadToFile() will overwrite (truncate) or append when opening a file."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_GPIB_ADDR_STATE: r#"This attribute shows whether the specified GPIB interface is currently addressed to talk or listen, or is not addressed."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_GPIB_UNADDRESSED(0) VI_GPIB_TALKER(1) VI_GPIB_LISTENER(2)]

            const VI_ATTR_GPIB_ATN_STATE: r#"This attribute shows the current state of the GPIB ATN (ATtentioN) interface line."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED(1) VI_STATE_UNASSERTED(0) VI_STATE_UNKNOWN(-1)]

            const VI_ATTR_GPIB_CIC_STATE: r#"This attribute shows whether the specified GPIB interface is currently CIC (Controller In Charge)."#
            (Read Only Global) ( ViBoolean) [static as N/A in VI_TRUE(1) VI_FALSE(0)]

            const VI_ATTR_GPIB_HS488_CBL_LEN: r#"This attribute specifies the total number of meters of GPIB cable used in the specified GPIB interface."#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_GPIB_HS488_NIMPL(-1) VI_GPIB_HS488_DISABLED(0) 1-15]

            const VI_ATTR_GPIB_NDAC_STATE: r#"This attribute shows the current state of the GPIB NDAC (Not Data ACcepted) interface line."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED(1) VI_STATE_UNASSERTED(0) VI_STATE_UNKNOWN(-1)]

            const VI_ATTR_GPIB_PRIMARY_ADDR: r#"VI_ATTR_GPIB_PRIMARY_ADDR specifies the primary address of the GPIB device used by the given session. For the GPIB INTFC Resource, this attribute is Read-Write."#
            (INSTR, MEMACC, BACKPLANE: Read Only Global) ( ViUInt16) [static as N/A in 0 to 30]

            const VI_ATTR_GPIB_READDR_EN: r#"VI_ATTR_GPIB_READDR_EN specifies whether to use repeat addressing before each read or write operation."#
            (Read/Write Local) ( ViBoolean) [static as VI_TRUE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_GPIB_RECV_CIC_STATE: r#"This attribute specifies whether the local controller has gained or lost CIC status."#
            (Read-Only) ( ViBoolean) [static as N/A in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_GPIB_REN_STATE: r#"VI_ATTR_GPIB_REN_STATE returns the current state of the GPIB REN (Remote ENable) interface line."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED(1) VI_STATE_UNASSERTED(0) VI_STATE_UNKNOWN(-1)]

            const VI_ATTR_GPIB_SECONDARY_ADDR: r#"VI_ATTR_GPIB_SECONDARY_ADDR specifies the secondary address of the GPIB device used by the given session. For the GPIB INTFC Resource, this attribute is Read-Write."#
            (INSTR, MEMACC, BACKPLANE: Read Only Global) ( ViUInt16) [static as N/A in 0 to 30, VI_NO_SEC_ADDR (FFFFh)]

            const VI_ATTR_GPIB_SRQ_STATE: r#"This attribute shows the current state of the GPIB SRQ (Service ReQuest) interface line."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED(1) VI_STATE_UNASSERTED(0) VI_STATE_UNKNOWN(-1)]

            const VI_ATTR_GPIB_SYS_CNTRL_STATE: r#"This attribute shows whether the specified GPIB interface is currently the system controller. In some implementations, this attribute may be modified only through a configuration utility. On these systems this attribute is read-only (RO)."#
            (Read/Write Global) ( ViBoolean) [static as N/A in VI_TRUE(1) VI_FALSE(0)]

            const VI_ATTR_GPIB_UNADDR_EN: r#"VI_ATTR_GPIB_UNADDR_EN specifies whether to unaddress the device (UNT and UNL) after each read or write operation."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_IMMEDIATE_SERV: r#"VI_ATTR_IMMEDIATE_SERV specifies whether the device associated with this session is an immediate servant of the controller running VISA."#
            (Read Only Global) ( ViBoolean) [static as N/A in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_INTF_INST_NAME: r#"VI_ATTR_INTF_INST_NAME specifies human-readable text that describes the given interface."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_INTF_NUM: r#"VI_ATTR_INTF_NUM specifies the board number for the given interface."#
            (Read Only Global) ( ViUInt16) [static as 0 in 0h to FFFFh]

            const VI_ATTR_INTF_TYPE: r#"VI_ATTR_INTF_TYPE specifies the interface type of the given session."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_INTF_GPIB (1) VI_INTF_VXI (2) VI_INTF_GPIB_VXI (3) VI_INTF_ASRL (4) VI_INTF_PXI (5) VI_INTF_TCPIP (6) VI_INTF_USB (7)]

            const VI_ATTR_INTR_STATUS_ID: r#"VI_ATTR_INTR_STATUS_ID specifies the 32-bit status/ID retrieved during the IACK cycle."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_IO_PROT: r#"VI_ATTR_IO_PROT specifies which protocol to use. In VXI, you can choose normal word serial or fast data channel (FDC). In GPIB, you can choose normal or high-speed (HS-488) transfers. In serial, TCPIP, or USB RAW, you can choose normal transfers or 488.2-defined strings. In USB INSTR, you can choose normal or vendor-specific transfers. In previous versions of VISA, VI_PROT_NORMAL was known as VI_NORMAL , VI_PROT_FDC was known as VI_FDC , VI_PROT_HS488 was known as VI_HS488 , and VI_PROT_4882_STRS was known as VI_ASRL488 ."#
            (Read/Write Local) ( ViUInt16)
            [
                while USB RAW {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_4882_STRS (4)}
                while VXI {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_FDC (2)}
                while GPIB {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_HS488 (3)}
                while Serial {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_4882_STRS (4)}
                while TCPIP {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_4882_STRS (4)}
                while USB INSTR {static as VI_PROT_NORMAL in VI_PROT_NORMAL (1) VI_PROT_USBTMC_VENDOR (5)}
            ]

            const VI_ATTR_JOB_ID: r#"VI_ATTR_JOB_ID contains the job ID of the asynchronous operation that has completed."#
            (Read Only) ( ViJobId) [static as N/A in N/A]

            const VI_ATTR_MAINfRAME_LA: r#"VI_ATTR_MA.infRAME_LA specifies the lowest logical address in the mainframe. If the logical address is not known, VI_UNKNOWN_LA is returned."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to 255 VI_UNKNOWN_LA (-1)]

            const VI_ATTR_MANF_ID: r#"VI_ATTR_MANF_ID is the manufacturer identification number of the device. For VXI resources, this refers to the VXI Manufacturer ID. For PXI INSTR resources, if the subsystem PCI Vendor ID is nonzero, this refers to the subsystem Vendor ID. Otherwise, this refers to the Vendor ID. For USB resources, this refers to the Vendor ID (VID)."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0h to FFFFh]

            const VI_ATTR_MANF_NAME: r#"This string attribute is the manufacturer name."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_MAX_QUEUE_LENGTH: r#"VI_ATTR_MAX_QUEUE_LENGTH specifies the maximum number of events that can be queued at any time on the given session. Events that occur after the queue has become full will be discarded. VI_ATTR_MAX_QUEUE_LENGTH is a Read/Write attribute until the first time viEnableEvent() is called on a session. Thereafter, this attribute is Read Only."#
            (Read/Write Local) ( ViUInt32) [static as 50 in 1h to FFFFFFFFh]

            const VI_ATTR_MEM_BASE: r#"VI_ATTR_MEM_BASE , VI_ATTR_MEM_BASE_32 , and VI_ATTR_MEM_BASE_64 specify the base address of the device in VXIbus memory address space. This base address is applicable to A24 or A32 address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_BASE: ViBusAddress) [static as  N/A in VI_ATTR_MEM_BASE: 0h to FFFFFFFFh for 32-bit applications 0h to FFFFFFFFFFFFFFFFh for 64-bit applications]

            const VI_ATTR_MEM_BASE_32: r#"VI_ATTR_MEM_BASE , VI_ATTR_MEM_BASE_32 , and VI_ATTR_MEM_BASE_64 specify the base address of the device in VXIbus memory address space. This base address is applicable to A24 or A32 address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_BASE_32: ViUInt32) [static as  N/A in VI_ATTR_MEM_BASE_32: 0h to FFFFFFFFh]

            const VI_ATTR_MEM_BASE_64: r#"VI_ATTR_MEM_BASE , VI_ATTR_MEM_BASE_32 , and VI_ATTR_MEM_BASE_64 specify the base address of the device in VXIbus memory address space. This base address is applicable to A24 or A32 address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_BASE_64: ViUInt64) [static as  N/A in VI_ATTR_MEM_BASE_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_MEM_SIZE: r#"VI_ATTR_MEM_SIZE , VI_ATTR_MEM_SIZE_32 , and VI_ATTR_MEM_SIZE_64 specify the size of memory requested by the device in VXIbus address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_SIZE: ViBusSize) [static as  N/A in VI_ATTR_MEM_SIZE: 0h to FFFFFFFFh for 32-bit applications 0h to FFFFFFFFFFFFFFFFh for 64-bit applications]

            const VI_ATTR_MEM_SIZE_32: r#"VI_ATTR_MEM_SIZE , VI_ATTR_MEM_SIZE_32 , and VI_ATTR_MEM_SIZE_64 specify the size of memory requested by the device in VXIbus address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_SIZE_32: ViUInt32) [static as  N/A in VI_ATTR_MEM_SIZE_32: 0h to FFFFFFFFh]

            const VI_ATTR_MEM_SIZE_64: r#"VI_ATTR_MEM_SIZE , VI_ATTR_MEM_SIZE_32 , and VI_ATTR_MEM_SIZE_64 specify the size of memory requested by the device in VXIbus address space. If the value of VI_ATTR_MEM_SPACE is VI_A16_SPACE , the value of this attribute is meaningless for the given VXI device."#
            (Read Only Global) (VI_ATTR_MEM_SIZE_64: ViUInt64) [static as  N/A in VI_ATTR_MEM_SIZE_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_MEM_SPACE: r#"VI_ATTR_MEM_SPACE specifies the VXIbus address space used by the device. The three types are A16, A24, or A32 memory address space. A VXI device with memory in A24 or A32 space also has registers accessible in the configuration section of A16 space. A VME device with memory in multiple address spaces requires one VISA resource for each address space used."#
            (Read Only Global) ( ViUInt16) [static as VI_A16_SPACE in VI_A16_SPACE (1) VI_A24_SPACE (2) VI_A32_SPACE (3)]

            const VI_ATTR_MODEL_CODE: r#"VI_ATTR_MODEL_CODE specifies the model code for the device. For VXI resources, this refers to the VXI Model Code. For PXI INSTR resources, if the subsystem PCI Vendor ID is nonzero, this refers to the subsystem Device ID. Otherwise, this refers to the Device ID. For USB resources, this refers to the Product ID (PID)."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0h to FFFFh]

            const VI_ATTR_MODEL_NAME: r#"This string attribute is the model name of the device."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_OPER_NAME: r#"VI_ATTR_OPER_NAME contains the name of the operation generating this event."#
            (Read Only) ( ViString) [static as N/A in N/A]

            const VI_ATTR_PXI_ACTUAL_LWIDTH: r#"VI_ATTR_PXI_ACTUAL_LWIDTH specifies the PCI Express link width negotiated between the PCI Express host controller and the device. A value of -1 indicates that the device is not a PXI/PCI Express device."#
            (Read Only Global) ( ViInt16) [static as N/A in -1, 1, 2, 4, 8, 16]

            const VI_ATTR_PXI_BUS_NUM: r#"VI_AT TR_PXI_BUS_NUM specifies the PCI bus number of this device."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0 to 255]

            const VI_ATTR_PXI_CHASSIS: r#"VI_ATTR_PXI_CHASSIS specifies the PXI chassis number of this device. A value of -1 means the chassis number is unknown."#
            (Read Only Global) ( ViInt16) [static as N/A in -1, 0 to 255]

            const VI_ATTR_PXI_DEST_TRIG_BUS: r#"VI_ATTR_PXI_DEST_TRIG_BUS specifies the segment to use to qualify trigDest in viMapTrigger . * You can determine the number of segments from MAX (in the trigger reservation panel), from the chassis documentation, and by looking at the dividing lines on the physical front panel of the chassis itself. Range: Single-Segment Chassis (8 Slots or Less): N/A, Multisegment Chassis (More than 8 Slots): 1...number of chassis segments"#
            (Read/Write Local) ( ViInt16) [static as -1 in N/A]

            const VI_ATTR_PXI_DEV_NUM: r#"This is the PXI device number."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0 to 31]

            const VI_ATTR_PXI_DSTAR_BUS: r#"VI_ATTR_PXI_DSTAR_BUS specifies the differential star bus number of this device. A value of -1 means the chassis is unidentified or does not have a timing slot."#
            (Read Only Global) ( ViInt16) [static as N/A in N/A]

            const VI_ATTR_PXI_DSTAR_SET: r#"VI_ATTR_PXI_DSTAR_SET specifies the  set of PXIe DStar lines connected to the slot this device is in. Each slot can be connected  to a set of DStar lines, and each set has a number. For example, one slot could be connected to the DStar set 2, while the next one could be connected to the DStar set 4. The VI_ATTR_PXI_DSTAR_SET value does  not represent individual line numbers; instead, it represents the number of the set itself. A PXIe DStar set consists of the numbered differential pairs PXIe-DSTARA, PXIe-DSTARB, and PXIe-DSTARC routed from the PXIe system timing slot. For example, if VI_ATTR_PXI_DSTAR_SET is 4, the slot the device is in is connected to PXIe-DStarA_4, PXIe-DStarB_4, and PXIe-DStarC_4.  A value of -1 means the chassis is unidentified or the slot the device is in does not have a DStar set connected to it. Also, although a PXIe slot has a DStar connection, the device in that slot may not. In that case, the value of VI_ATTR_PXI_DSTAR_SET still will be the set connected to the slot the device is in."#
            (Read Only Global) ( ViInt16) [static as N/A in -1, 0 to 16]

            const VI_ATTR_PXI_FUNC_NUM: r#"This is the PCI function number of the PXI/PCI resource. For most devices, the function number is 0, but a multifunction device may have a function number up to 7. The meaning of a function number other than 0 is device specific."#
            (Read Only Global) ( ViUInt16) [static as 0 in 0 to 7]

            const VI_ATTR_PXI_IS_EXPRESS: r#"VI_ATTR_PXI_IS_EXPRESS specifies whether the device is PXI/PCI or PXI/PCI Express."#
            (Read Only Global) ( ViBoolean) [static as N/A in VI_TRUE, VI_FALSE]

            const VI_ATTR_PXI_MAX_LWIDTH: r#"VI_ATTR_PXI_MAX_LWIDTH specifies the maximum PCI Express link width of the device. A value of -1 indicates that the device is not a PXI/PCI Express device."#
            (Read Only Global) ( ViInt16) [static as N/A in -1, 1, 2, 4, 8, 16]

            const VI_ATTR_PXI_MEM_BASE_BAR0: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR0: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR0: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            /* duplicate of _BARx */

            const VI_ATTR_PXI_MEM_BASE_BAR1: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR1: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR1: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            const VI_ATTR_PXI_MEM_BASE_BAR2: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR2: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR2: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            const VI_ATTR_PXI_MEM_BASE_BAR3: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR3: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR3: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            const VI_ATTR_PXI_MEM_BASE_BAR4: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR4: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR4: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            const VI_ATTR_PXI_MEM_BASE_BAR5: r#"PXI memory base address assigned to the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_SIZE_BAR5: r#"Memory size used by the device in the specified BAR. If the value of the corresponding VI_ATTR_PXI_MEM_TYPE_BAR x is VI_PXI_ADDR_NONE , the value of this attribute is undefined for the given PXI device."#
            (Read Only Global) ( ViUInt32) [static as N/A in 0 to FFFFFFFFh]

            const VI_ATTR_PXI_MEM_TYPE_BAR5: r#"Memory type used by the device in the specified BAR (if applicable)."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_PXI_ADDR_NONE(0) VI_PXI_ADDR_MEM(1) VI_PXI_ADDR_IO(2)]

            /* end duplicate of _BARx */

            const VI_ATTR_PXI_RECV_INTR_DATA: r#"VI_ATTR_PXI_RECV_INTR_DATA shows the first PXI/PCI register that was read in the successful interrupt detection sequence."#
            (Read Only) ( ViUInt32) [static as N/A in N/A]

            const VI_ATTR_PXI_RECV_INTR_SEQ: r#"VI_ATTR_PXI_RECV_INTR_SEQ shows the index of the interrupt sequence that detected the interrupt condition."#
            (Read Only) ( ViInt16) [static as N/A in N/A]

            const VI_ATTR_PXI_SLOT_LBUS_LEFT: r#"VI_ATTR_PXI_SLOT_LBUS_LEFT specifies the slot number or special feature connected to the local bus left lines of this device."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_PXI_LBUS_UNKNOWN (-1) ; VI_PXI_LBUS_NONE (0) ; NormalSlots (1 to 18); VI_PXI_LBUS_STAR_TRIG_BUS_0 (1000) to VI_PXI_LBUS_STAR_TRIG_BUS_9 (1009) ; VI_PXI_STAR_TRIG_CONTROLLER (1413) ; VI_PXI_LBUS_SCXI (2000)]

            const VI_ATTR_PXI_SLOT_LBUS_RIGHT: r#"VI_ATTR_PXI_SLOT_LBUS_RIGHT specifies the slot number or special feature connected to the local bus right lines of this device."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_PXI_LBUS_UNKNOWN (-1) ; VI_PXI_LBUS_NONE (0) ; NormalSlots (1 to 18); VI_PXI_LBUS_STAR_TRIG_BUS_0 (1000) to VI_PXI_LBUS_STAR_TRIG_BUS_9 (1009) ; VI_PXI_STAR_TRIG_CONTROLLER (1413) ; VI_PXI_LBUS_SCXI (2000)]

            const VI_ATTR_PXI_SLOT_LWIDTH: r#"VI_ATTR_PXI_SLOT_LWIDTH specifies the PCI Express link width of the PXI Express peripheral slot in which the device resides. A value of -1 indicates that the device is not a PXI Express device."#
            (Read Only Global) ( ViInt16) [static as N/A in -1, 1, 4, 8]

            const VI_ATTR_PXI_SLOTPATH: r#"VI_ATTR_PXI_SLOTPATH specifies the slot path of this device. The purpose of a PXI slot path is to describe the PCI bus hierarchy in a manner independent of the PCI bus number. PXI slot paths are a sequence of values representing the PCI device number and function number of a PCI module and each parent PCI bridge that routes the module to the host PCI bridge (bus 0). Each value is represented as " dev[.func] ", where the function number is listed only if it is non-zero. When a PXI slot path includes multiple values, the values are comma-separated. The string format of the attribute value looks like this: device1[.function1][,device2[.function2]][,...] An example string is " 5.1,12,8 ". In this case, there is a PCI-to-PCI bridge on device 8 on the root bus. On its secondary bus, there is another PCI-to-PCI bridge on device 12. On its secondary bus, there is an instrument on device 5, function 1. The example string value describes this instrument's slot path."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_PXI_SRC_TRIG_BUS: r#"VI_ATTR_PXI_SRC_TRIG_BUS specifies the segment to use to qualify trigSrc in viMapTrigger . * You can determine the number of segments from MAX (in the trigger reservation panel), from the chassis documentation, and by looking at the dividing lines on the physical front panel of the chassis itself. Range: Single-Segment Chassis (8 Slots or Less): N/A Multisegment Chassis (More than 8 Slots): 1...number of chassis segments"#
            (Read/Write Local) ( ViInt16) [static as -1 in N/A]

            const VI_ATTR_PXI_STAR_TRIG_BUS: r#"VI_ATTR_PXI_STAR_TRIG_BUS specifies the star trigger bus number of this device."#
            (Read Only Global) ( ViInt16) [static as N/A in N/A]

            const VI_ATTR_PXI_STAR_TRIG_LINE: r#"V I_ATTR_PXI_STAR_TRIG_LINE specifies the PXI_STAR line connected to this device."#
            (Read Only Global) ( ViInt16) [static as N/A in N/A]

            const VI_ATTR_PXI_TRIG_BUS: r#"VI_ATTR_PXI_TRIG_BUS specifies the trigger bus number of this device."#
            (INSTR: Read Only Global BACKPLANE: Read/Write Local) ( ViInt16) [static as N/A in N/A]

            const VI_ATTR_RD_BUF_OPER_MODE: r#"VI_ATTR_RD_BUF_OPER_MODE specifies the operational mode of the formatted I/O read buffer. When the operational mode is set to VI_FLUSH_DISABLE (default), the buffer is flushed only on explicit calls to viFlush() . If the operational mode is set to VI_FLUSH_ON_ACCESS , the read buffer is flushed every time a viScanf() (or related) operation completes."#
            (Read/Write Local) ( ViUInt16) [static as VI_FLUSH_DISABLE in VI_FLUSH_ON_ACCESS (1) VI_FLUSH_DISABLE (3)]

            const VI_ATTR_RD_BUF_SIZE: r#"This is the current size of the formatted I/O input buffer for this session. The user can modify this value by calling viSetBuf() ."#
            (Read Only Local) ( ViUInt32) [static as N/A in N/A]

            const VI_ATTR_RECV_INTR_LEVEL: r#"VI_ATTR_RECV_INTR_LEVEL is the VXI interrupt level on which the interrupt was received."#
            (Read Only) ( ViInt16) [static as N/A in 1 to 7; VI_UNKNOWN_LEVEL (-1)]

            const VI_ATTR_RECV_TRIG_ID: r#"VI_ATTR_RECV_TRIG_ID identifies the triggering mechanism on which the specified trigger event was received."#
            (Read Only) ( ViInt16) [static as N/A in VI_TRIG_SW(-1) VI_TRIG_TTL0 (0) to VI_TRIG_TTL7 (7); VI_TRIG_ECL0 (8) to VI_TRIG_ECL1 (9)]

            const VI_ATTR_RET_COUNT: r#"VI_ATTR_RET_COUNT , VI_ATTR_RET_COUNT_32 , and VI_ATTR_RET_COUNT_64 contain the actual number of elements that were asynchronously transferred. VI_ATTR_RET_COUNT_32 is always a 32-bit value. VI_ATTR_RET_COUNT_64 is always a 64-bit value. VI_ATTR_RET_COUNT_64 is not supported with 32-bit applications. VI_ATTR_RET_COUNT is a 32-bit value for 32-bit applications and a 64-bit value for 64-bit applications."#
            (Read Only) (VI_ATTR_RET_COUNT: ViUInt32 for 32-bit applications ViUInt64 for 64-bit applications) [static as  N/A in VI_ATTR_RET_COUNT: 0h to FFFFFFFFh for 32-bit applications 0h to FFFFFFFFFFFFFFFFh for 64-bit applications]

            const VI_ATTR_RET_COUNT_32: r#"VI_ATTR_RET_COUNT , VI_ATTR_RET_COUNT_32 , and VI_ATTR_RET_COUNT_64 contain the actual number of elements that were asynchronously transferred. VI_ATTR_RET_COUNT_32 is always a 32-bit value. VI_ATTR_RET_COUNT_64 is always a 64-bit value. VI_ATTR_RET_COUNT_64 is not supported with 32-bit applications. VI_ATTR_RET_COUNT is a 32-bit value for 32-bit applications and a 64-bit value for 64-bit applications."#
            (Read Only) (VI_ATTR_RET_COUNT_32: ViUInt32) [static as  N/A in VI_ATTR_RET_COUNT_32: 0h to FFFFFFFFh]

            const VI_ATTR_RET_COUNT_64: r#"VI_ATTR_RET_COUNT , VI_ATTR_RET_COUNT_32 , and VI_ATTR_RET_COUNT_64 contain the actual number of elements that were asynchronously transferred. VI_ATTR_RET_COUNT_32 is always a 32-bit value. VI_ATTR_RET_COUNT_64 is always a 64-bit value. VI_ATTR_RET_COUNT_64 is not supported with 32-bit applications. VI_ATTR_RET_COUNT is a 32-bit value for 32-bit applications and a 64-bit value for 64-bit applications."#
            (Read Only) (VI_ATTR_RET_COUNT_64: ViUInt64) [static as  N/A in VI_ATTR_RET_COUNT_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_RM_SESSION: r#"VI_ATTR_RM_SESSION specifies the session of the Resource Manager that was used to open this session."#
            (Read Only Local) ( ViSession) [static as N/A in N/A]

            const VI_ATTR_RSRC_CLASS: r#"VI_ATTR_RSRC_CLASS specifies the resource class (for example, "INSTR") as defined by the canonical resource name."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_RSRC_IMPL_VERSION: r#"VI_ATTR_RSRC_IMPL_VERSION is the resource version that uniquely identifies each of the different revisions or implementations of a resource. This attribute value is defined by the individual manufacturer and increments with each new revision. The format of the value has the upper 12 bits as the major number of the version, the next lower 12 bits as the minor number of the version, and the lowest 8 bits as the sub-minor number of the version."#
            (Read Only Global) ( ViVersion) [static as N/A in 0h to FFFFFFFFh]

            const VI_ATTR_RSRC_LOCK_STATE: r#"VI_ATTR_RSRC_LOCK_STATE indicates the current locking state of the resource. The resource can be unlocked, locked with an exclusive lock, or locked with a shared lock."#
            (Read Only Global) ( ViAccessMode) [static as VI_NO_LOCK in VI_NO_LOCK (0) VI_EXCLUSIVE_LOCK (1) VI_SHARED_LOCK (2)]

            const VI_ATTR_RSRC_MANF_ID: r#"VI_ATTR_RSRC_MANF_ID is a value that corresponds to the VXI manufacturer ID of the vendor that implemented the VISA library. This attribute is not related to the device manufacturer attributes."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0h to 3FFFh]

            const VI_ATTR_RSRC_MANF_NAME: r#"VI_ATTR_RSRC_MANF_NAME is a string that corresponds to the manufacturer name of the vendor that implemented the VISA library. This attribute is not related to the device manufacturer attributes."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_RSRC_NAME: r#"VI_ATTR_RSRC_NAME is the unique identifier for a resource. Refer to VISA Resource Syntax and Examples for the syntax of resource strings and examples."#
            (Read Only Global) ( ViRsrc) [static as N/A in N/A]

            const VI_ATTR_RSRC_SPEC_VERSION: r#"VI_ATTR_RSRC_SPEC_VERSION is the resource version that uniquely identifies the version of the VISA specification to which the implementation is compliant. The format of the value has the upper 12 bits as the major number of the version, the next lower 12 bits as the minor number of the version, and the lowest 8 bits as the sub-minor number of the version. The current VISA specification defines the value to be 00300000h."#
            (Read Only Global) ( ViVersion) [static as 00300000h in 0h to FFFFFFFFh]

            const VI_ATTR_SEND_END_EN: r#"VI_ATTR_SEND_END_EN specifies whether to assert END during the transfer of the last byte of the buffer. VI_ATTR_SEND_END_EN is relevant only in viWrite and related operations. On Serial INSTR sessions, if this attribute is set to VI_FALSE, the write will transmit the exact contents of the user buffer, without modifying it and without appending anything to the data being written. If this attribute is set to VI_TRUE, VISA will perform the behavior described in VI_ATTR_ASRL_END_OUT . On GPIB, VXI, TCP/IP INSTR, and USB INSTR sessions, if this attribute is set to VI_TRUE, VISA will include the 488.2 defined "end of message" terminator."#
            (Read/Write Local) ( ViBoolean) [static as VI_TRUE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_SIGP_STATUS_ID: r#"VI_ATTR_SIGP_STATUS_ID is the 16-bit Status/ID value retrieved during the IACK cycle or from the Signal register."#
            (Read Only) ( ViUInt16) [static as N/A in 0h to FFFFh]

            const VI_ATTR_SLOT: r#"VI_ATTR_SLOT specifies the physical slot location of the device. If the slot number is not known, VI_UNKNOWN_SLOT is returned."#
            (Read Only Global) ( ViInt16)
            [
                while VXI {static as N/A in 0 to 12 VI_UNKNOWN_SLOT (-1)}
                while PXI {static as N/A in 1 to 18 VI_UNKNOWN_SLOT (-1)}
            ]

            const VI_ATTR_SRC_ACCESS_PRIV: r#"VI_ATTR_SRC_ACCESS_PRIV specifies the address modifier to be used in high-level access operations, such as viIn XX () and viMoveIn XX () , when reading from the source."#
            (Read/Write Local) ( ViUInt16) [static as VI_DATA_PRIV in VI_DATA_PRIV (0) VI_DATA_NPRIV (1) VI_PROG_PRIV (2) VI_PROG_NPRIV (3) VI_BLCK_PRIV (4) VI_BLCK_NPRIV (5) VI_D64_PRIV (6) VI_D64_NPRIV (7)]

            const VI_ATTR_SRC_BYTE_ORDER: r#"VI_ATTR_SRC_BYTE_ORDER specifies the byte order to be used in high-level access operations, such as viIn XX () and viMoveIn XX () , when reading from the source."#
            (Read/Write Local) ( ViUInt16) [static as VI_BIG_ENDIAN in VI_BIG_ENDIAN (0) VI_LITTLE_ENDIAN (1)]

            const VI_ATTR_SRC_INCREMENT: r#"VI_ATTR_SRC_INCREMENT is used in the viMoveIn XX () operations to specify by how many elements the source offset is to be incremented after every transfer. The default value of this attribute is 1 (that is, the source address will be incremented by 1 after each transfer), and the viMoveIn XX () operations move from consecutive elements. If this attribute is set to 0, the viMoveIn XX () operations will always read from the same element, essentially treating the source as a FIFO register."#
            (Read/Write Local) ( ViInt32) [static as 1 in 0 to 1]

            const VI_ATTR_STATUS: r#"VI_ATTR_STATUS contains the return code of the operation generating this event."#
            (Read Only) ( ViStatus) [static as N/A in N/A]

            const VI_ATTR_SUPPRESS_END_EN: r#"VI_ATTR_SUPPRESS_END_EN is relevant only in viRead and related operations. For all session types on which this attribute is supported, if this attribute is set to VI_TRUE, read will not terminate due to an END condition. However, a read may still terminate successfully if VI_ATTR_TERMCHAR_EN is set to VI_TRUE. Otherwise, read will not terminate until all requested data is received (or an error occurs). On Serial INSTR sessions, if this attribute is set to VI_FALSE, VISA will perform the behavior described in VI_ATTR_ASRL_END_IN . On USB RAW sessions, if this attribute is set to VI_FALSE, VISA will perform the behavior described in VI_ATTR_USB_END_IN . On TCP/IP SOCKET sessions, if this attribute is set to VI_FALSE, if NI-VISA reads some data and then detects a pause in the arrival of data packets, it will terminate the read operation. On TCP/IP SOCKET sessions, this attribute defaults to VI_TRUE in NI-VISA. On VXI INSTR sessions, if this attribute is set to VI_FALSE, the END bit terminates read operations."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_TCPIP_ADDR: r#"This is the TCPIP address of the device to which the session is connected. This string is formatted in dot notation."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_TCPIP_DEVICE_NAME: r#"This specifies the LAN device name used by the VXI-11 or LXI protocol during connection."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_TCPIP_HOSTNAME: r#"This specifies the host name of the device. If no host name is available, this attribute returns an empty string."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_TCPIP_KEEPALIVE: r#"Setting this attribute to TRUE requests that a TCP/IP provider enable the use of keep-alive packets on TCP connections. After the system detects that a connection was dropped, VISA returns a lost connection error code on subsequent I/O calls on the session. The time required for the system to detect that the connection was dropped is dependent on the system and is not settable."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE(1) VI_FALSE(0)]

            const VI_ATTR_TCPIP_NODELAY: r#"The Nagle algorithm is disabled when this attribute is enabled (and vice versa). The Nagle algorithm improves network performance by buffering "send" data until a full-size packet can be sent. This attribute is enabled by default in VISA to verify that synchronous writes get flushed immediately."#
            (Read/Write Local) ( ViBoolean) [static as VI_TRUE in VI_TRUE(1) VI_FALSE(0)]

            const VI_ATTR_TCPIP_PORT: r#"This specifies the port number for a given TCPIP address. For a TCPIP SOCKET Resource, this is a required part of the address string."#
            (Read Only Global) ( ViUInt16) [static as N/A in 0 to FFFFh]

            const VI_ATTR_TERMCHAR: r#"VI_ATTR_TERMCHAR is the termination character. When the termination character is read and VI_ATTR_TERMCHAR_EN is enabled during a read operation, the read operation terminates. For a Serial INSTR session, VI_ATTR_TERMCHAR is Read/Write when the corresponding session is not enabled to receive VI_EVENT_ASRL_TERMCHAR events. When the session is enabled to receive VI_EVENT_ASRL_TERMCHAR events, the attribute VI_ATTR_TERMCHAR is Read Only. For all other session types, the attribute VI_ATTR_TERMCHAR is always Read/Write."#
            (Read/Write Local) ( ViUInt8) [static as 0Ah (linefeed) in 0 to FFh]

            const VI_ATTR_TERMCHAR_EN: r#"VI_ATTR_TERMCHAR_EN is a flag that determines whether the read operation should terminate when a termination character is received. This attribute is ignored if VI_ATTR_ASRL_END_IN is set to VI_ASRL_END_TERMCHAR. This attribute is valid for both raw I/O (viRead) and formatted I/O (viScanf)."#
            (Read/Write Local) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_TMO_VALUE: r#"VI_ATTR_TMO_VALUE specifies the minimum timeout value to use (in milliseconds) when accessing the device associated with the given session. A timeout value of VI_TMO_IMMEDIATE means that operations should never wait for the device to respond. A timeout value of VI_TMO_INFINITE disables the timeout mechanism. Notice that the actual timeout value used by the driver may be higher than the requested one. The actual timeout value is returned when this attribute is retrieved via viGetAttribute() ."#
            (Read/Write Local) ( ViUInt32) [static as 2000 in VI_TMO_IMMEDIATE (0) ; 1 to FFFFFFFEh; VI_TMO_INFINITE (FFFFFFFFh)]

            const VI_ATTR_TRIG_ID: r#"VI_ATTR_TRIG_ID is the identifier for the current triggering mechanism. VI_ATTR_TRIG_ID is Read/Write when the corresponding session is not enabled to receive trigger events. When the session is enabled to receive trigger events, the attribute VI_ATTR_TRIG_ID is Read Only."#
            (Read/Write Local) ( ViInt16)
            [
                while PXI {static as VI_TRIG_SW in VI_TRIG_SW (-1) VI_TRIG_TTL0 (0) to VI_TRIG_TTL7 (7)}
                while Serial {static as VI_TRIG_SW in VI_TRIG_SW (-1)}
                while GPIB {static as VI_TRIG_SW in VI_TRIG_SW (-1)}
                while VXI {static as VI_TRIG_SW in VI_TRIG_SW (-1); VI_TRIG_TTL0 (0) to VI_TRIG_TTL7 (7); VI_TRIG_ECL0 (8) to VI_TRIG_ECL1 (9)}
                while TCPIP {static as VI_TRIG_SW in VI_TRIG_SW (-1)}
            ]

            const VI_ATTR_USB_INTFC_NUM: r#"VI_ATTR_USB_INTFC_NUM specifies the USB interface number used by the given session."#
            (Read Only Global) ( ViInt16) [static as 0 in 0 to FEh]

            const VI_ATTR_USB_MAX_INTR_SIZE: r#"VI_ATTR_USB_MAX_INTR_SIZE specifies the maximum size of data that will be stored by any given USB interrupt. If a USB interrupt contains more data than this size, the data in excess of this size will be lost. VI_ATTR_USB_MAX_INTR_SIZE is Read/Write when the corresponding session is not enabled to receive USB interrupt events. When the session is enabled to receive USB interrupt events, the attribute VI_ATTR_USB_MAX_INTR_SIZE is Read Only."#
            (Read/Write Local) ( ViUInt16) [static as N/A in 0 to FFFFh]

            const VI_ATTR_USB_PROTOCOL: r#"VI_ATTR_USB_PROTOCOL specifies the USB protocol used by this USB interface."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to FFh]

            const VI_ATTR_USB_RECV_INTR_DATA: r#"VI_ATTR_USB_RECV_INTR_DATA contains the actual received data from the USB Interrupt. The passed in data buffer must be of size at least equal to the value of VI_ATTR_USB_RE CV_INTR_SIZE ."#
            (Read Only) ( ViAUInt8) [static as N/A in N/A]

            const VI_ATTR_USB_RECV_INTR_SIZE: r#"VI_ATTR_USB_RECV_INTR_SIZE contains the number of bytes of USB interrupt data that is stored."#
            (Read Only) ( ViUInt16) [static as N/A in N/A]

            const VI_ATTR_USB_SERIAL_NUM: r#"VI_ATTR_USB_SERIAL_NUM specifies the USB serial number of this device."#
            (Read Only Global) ( ViString) [static as N/A in N/A]

            const VI_ATTR_USER_DATA: r#"VI_ATTR_USER_DATA , VI_ATTR_USER_DATA_32 , and VI_ATTR_USER_DATA_64 store data to be used privately by the application for a particular session. VISA does not use this data for any purpose. It is provided to the application for its own use. VI_ATTR_USER_DATA_64 is not supported with 32-bit applications."#
            (Read/Write Local) (VI_ATTR_USER_DATA: ViAddr) [static as  N/A in VI_ATTR_USER_DATA: Not specified]

            const VI_ATTR_USER_DATA_32: r#"VI_ATTR_USER_DATA , VI_ATTR_USER_DATA_32 , and VI_ATTR_USER_DATA_64 store data to be used privately by the application for a particular session. VISA does not use this data for any purpose. It is provided to the application for its own use. VI_ATTR_USER_DATA_64 is not supported with 32-bit applications."#
            (Read/Write Local) (VI_ATTR_USER_DATA_32: ViUInt32) [static as  N/A in VI_ATTR_USER_DATA_32: 0h to FFFFFFFFh]

            const VI_ATTR_USER_DATA_64: r#"VI_ATTR_USER_DATA , VI_ATTR_USER_DATA_32 , and VI_ATTR_USER_DATA_64 store data to be used privately by the application for a particular session. VISA does not use this data for any purpose. It is provided to the application for its own use. VI_ATTR_USER_DATA_64 is not supported with 32-bit applications."#
            (Read/Write Local) (VI_ATTR_USER_DATA_64: ViUInt64) [static as  N/A in VI_ATTR_USER_DATA_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_VXI_DEV_CLASS: r#"This attribute represents the VXI-defined device class to which the resource belongs, either message based ( VI_VXI_CLASS_MESSAGE ), register based ( VI_VXI_CLASS_REGISTER ), extended ( VI_VXI_CLASS_EXTENDED ), or memory ( VI_VXI_CLASS_MEMORY ). VME devices are usually either register based or belong to a miscellaneous class ( VI_VXI_CLASS_OTHER )."#
            (Read Only Global) ( ViUInt16) [static as N/A in VI_VXI_CLASS_MEMORY(0) VI_VXI_CLASS_EXTENDED(1) VI_VXI_CLASS_MESSAGE(2) VI_VXI_CLASS_REGISTER(3) VI_VXI_CLASS_OTHER(4)]

            const VI_ATTR_VXI_LA: r#"For an INSTR session, VI_ATTR_VXI_LA specifies the logical address of the VXI or VME device used by the given session. For a MEMACC or SERVANT session, this attribute specifies the logical address of the local controller."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to 511]

            const VI_ATTR_VXI_TRIG_STATUS: r#"This attribute shows the current state of the VXI trigger lines. This is a bit vector with bits 0-9 corresponding to VI_TRIG_TTL0 through VI_TRIG_ECL1 ."#
            (Read Only Global) ( ViUInt32) [static as N/A in N/A]

            const VI_ATTR_VXI_TRIG_SUPPORT: r#"This attribute shows which VXI trigger lines this implementation supports. This is a bit vector with bits 0-9 corresponding to VI_TRIG_TTL0 through VI_TRIG_ECL1 ."#
            (Read Only Global) ( ViUInt32) [static as N/A in N/A]

            const VI_ATTR_VXI_VME_INTR_STATUS: r#"This attribute shows the current state of the VXI/VME interrupt lines. This is a bit vector with bits 0-6 corresponding to interrupt lines 1-7."#
            (Read Only Global) ( ViUInt16) [static as N/A in N/A]

            const VI_ATTR_VXI_VME_SYSFAIL_STATE: r#"This attribute shows the current state of the VXI/VME SYSFAIL (SYStem FAILure) backplane line."#
            (Read Only Global) ( ViInt16) [static as N/A in VI_STATE_ASSERTED(1) VI_STATE_DEASSERTED(0) VI_STATE_UNKNOWN(-1)]

            const VI_ATTR_WIN_ACCESS: r#"VI_ATTR_WIN_ACCESS specifies the modes in which the current window may be accessed."#
            (Read Only Local) ( ViUInt16) [static as VI_NMAPPED in VI_NMAPPED (1) VI_USE_OPERS (2) VI_DEREF_ADDR (3)]

            const VI_ATTR_WIN_ACCESS_PRIV: r#"VI_ATTR_WIN_ACCESS_PRIV specifies the address modifier to be used in low-level access operations, such as viMapAddress() , viPeek XX () , and viPoke XX () , when accessing the mapped window. This attribute is Read/Write when the corresponding session is not mapped (that is, when VI_ATTR_WIN_ACCESS is VI_NMAPPED . When the session is mapped, this attribute is Read Only."#
            (Read/Write Local) ( ViUInt16) [static as VI_DATA_PRIV in VI_DATA_PRIV (0) VI_DATA_NPRIV (1) VI_PROG_PRIV (2) VI_PROG_NPRIV (3) VI_BLCK_PRIV (4) VI_BLCK_NPRIV (5)]

            const VI_ATTR_WIN_BASE_ADDR: r#"VI_ATTR_WIN_BASE_ADDR , VI_ATTR_WIN_BASE_ADDR_32 , and VI_ATTR_WIN_BASE_ADDR_64 specify the base address of the interface bus to which this window is mapped. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_BASE_ADDR: ViBusAddress) [static as  N/A in VI_ATTR_WIN_BASE_ADDR: 0h to FFFFFFFFh for 32-bit applications 0h to FFFFFFFFFFFFFFFFh for 64-bit applications]

            const VI_ATTR_WIN_BASE_ADDR_32: r#"VI_ATTR_WIN_BASE_ADDR , VI_ATTR_WIN_BASE_ADDR_32 , and VI_ATTR_WIN_BASE_ADDR_64 specify the base address of the interface bus to which this window is mapped. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_BASE_ADDR_32: ViUInt32) [static as  N/A in VI_ATTR_WIN_BASE_ADDR_32: 0h to FFFFFFFFh]

            const VI_ATTR_WIN_BASE_ADDR_64: r#"VI_ATTR_WIN_BASE_ADDR , VI_ATTR_WIN_BASE_ADDR_32 , and VI_ATTR_WIN_BASE_ADDR_64 specify the base address of the interface bus to which this window is mapped. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_BASE_ADDR_64: ViUInt64) [static as  N/A in VI_ATTR_WIN_BASE_ADDR_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_WIN_BYTE_ORDER: r#"VI_ATTR_WIN_BYTE_ORDER specifies the byte order to be used in low-level access operations, such as viMapAddress() , viPeek XX () , and viPoke XX () , when accessing the mapped window. This attribute is Read/Write when the corresponding session is not mapped (that is, when VI_ATTR_WIN_ACCESS is VI_NMAPPED . When the session is mapped, this attribute is Read Only."#
            (Read/Write Local) ( ViUInt16) [static as VI_BIG_ENDIAN in VI_BIG_ENDIAN (0) VI_LITTLE_ENDIAN (1)]

            const VI_ATTR_WIN_SIZE: r#"VI_ATTR_WIN_SIZE , VI_ATTR_WIN_SIZE_32 , and VI_ATTR_WIN_SIZE_64 specify the size of the region mapped to this window. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_SIZE: ViBusSize) [static as  N/A in VI_ATTR_WIN_SIZE: 0h to FFFFFFFFh for 32-bit applications 0h to FFFFFFFFFFFFFFFFh for 64-bit applications]

            const VI_ATTR_WIN_SIZE_32: r#"VI_ATTR_WIN_SIZE , VI_ATTR_WIN_SIZE_32 , and VI_ATTR_WIN_SIZE_64 specify the size of the region mapped to this window. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_SIZE_32: ViUInt32) [static as  N/A in VI_ATTR_WIN_SIZE_32: 0h to FFFFFFFFh]

            const VI_ATTR_WIN_SIZE_64: r#"VI_ATTR_WIN_SIZE , VI_ATTR_WIN_SIZE_32 , and VI_ATTR_WIN_SIZE_64 specify the size of the region mapped to this window. If the value of VI_ATTR_WIN_ACCESS is VI_NMAPPED , the value of this attribute is undefined."#
            (Read Only Local) (VI_ATTR_WIN_SIZE_64: ViUInt64) [static as  N/A in VI_ATTR_WIN_SIZE_64: 0h to FFFFFFFFFFFFFFFFh]

            const VI_ATTR_WR_BUF_OPER_MODE: r#"VI_ATTR_WR_BUF_OPER_MODE specifies the operational mode of the formatted I/O write buffer. When the operational mode is set to VI_FLUSH_WHEN_FULL (default), the buffer is flushed when an END indicator is written to the buffer, or when the buffer fills up. If the operational mode is set to VI_FLUSH_ON_ACCESS , the write buffer is flushed under the same conditions, and also every time a viPrintf() (or related) operation completes."#
            (Read/Write Local) ( ViUInt16) [static as VI_FLUSH_WHEN_FULL in VI_FLUSH_ON_ACCESS (1) VI_FLUSH_WHEN_FULL (2)]

            const VI_ATTR_WR_BUF_SIZE: r#"This is the current size of the formatted I/O output buffer for this session. The user can modify this value by calling viSetBuf() ."#
            (Read Only Local) ( ViUInt32) [static as N/A in N/A]

            /*- National Instruments ---------------------------------------------------*/

            /*
            const VI_ATTR_USB_ALT_SETTING: r#"VI_ATTR_USB_ALT_SETTING specifies the USB alternate setting used by this USB interface. VI_ATTR_USB_ALT_SETTING is Read/Write when the corresponding session is not enabled to receive USB interrupt events. If the session is enabled to receive USB interrupt events or if there are any other sessions to this resource, the attribute VI_ATTR_USB_ALT_SETTING is Read Only."#
            (Read/Write Global) ( ViInt16) [static as 0 in 0 to FFh]

            const VI_ATTR_USB_BULK_IN_PIPE: r#"VI_ATTR_USB_BULK_IN_PIPE specifies the endpoint address of the USB bulk-in pipe used by the given session. An initial value of -1 signifies that this resource does not have any bulk-in pipes. This endpoint is used in viRead and related operations."#
            (Read/Write Local) ( ViInt16) [static as N/A in -1, 81h to 8Fh]

            const VI_ATTR_USB_BULK_IN_STATUS: r#"VI_ATTR_USB_BULK_IN_STATUS specifies whether the USB bulk-in pipe used by the given session is stalled or ready. This attribute can be set to only VI_USB_PIPE_READY ."#
            (Read/Write Local) ( ViInt16) [static as N/A in VI_USB_PIPE_STATE_UNKNOWN (-1) VI_USB_PIPE_READY (0) VI_USB_PIPE_STALLED (1)]

            const VI_ATTR_USB_BULK_OUT_PIPE: r#"VI_ATTR_USB_BULK_OUT_PIPE specifies the endpoint address of the USB bulk-out or interrupt-out pipe used by the given session. An initial value of -1 signifies that this resource does not have any bulk-out or interrupt-out pipes. This endpoint is used in viWrite and related operations."#
            (Read/Write Local) ( ViInt16) [static as N/A in -1, 01h to 0Fh]

            const VI_ATTR_USB_BULK_OUT_STATUS: r#"VI_ATTR_USB_BULK_OUT_STATUS specifies whether the USB bulk-out or interrupt-out pipe used by the given session is stalled or ready. This attribute can be set to only VI_USB_PIPE_READY ."#
            (Read/Write Local) ( ViInt16) [static as N/A in VI_USB_PIPE_STATE_UNKNOWN (-1) VI_USB_PIPE_READY (0) VI_USB_PIPE_STALLED (1)]

            const VI_ATTR_USB_CLASS: r#"VI_ATTR_USB_CLASS specifies the USB class used by this USB interface."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to FFh]

            const VI_ATTR_USB_CTRL_PIPE: r#"VI_ATTR_USB_CTRL_PIPE specifies the endpoint address of the USB control pipe used by the given session. A value of 0 signifies that the default control pipe will be used. This endpoint is used in viUsbControlIn and viUsbControlOut operations. Nonzero values may not be supported on all platforms."#
            (Read/Write Local) ( ViInt16) [static as 00h in 00h to 0Fh]

            const VI_ATTR_USB_END_IN: r#"VI_ATTR_USB_END_IN indicates the method used to terminate read operations. If it is set to VI_USB_END_NONE , short packets are ignored for read operations, so reads will not terminate until all of the requested data is received (or an error occurs). If it is set to VI_USB_END_SHORT , the read operation will terminate on a short packet; use this if the device will terminate all read transfers with a short packet, including sending a zero (short) packet when the last data packet is full. If it is set to VI_USB_END_SHORT_OR_COUNT , the read operation will terminate on a short packet or when it receives the requested count of data bytes; use this if the device does not send zero packets."#
            (Read/Write Local) ( ViUInt16) [static as VI_USB_END_SHORT_OR_COUNT in VI_USB_END_NONE (0) VI_USB_END_SHORT (4)]

            const VI_ATTR_USB_INTR_IN_PIPE: r#"VI_ATTR_USB_INTR_IN_PIPE specifies the endpoint address of the USB interrupt-in pipe used by the given session. An initial value of -1 signifies that this resource does not have any interrupt-in pipes. This endpoint is used in viEnableEvent for VI_EVENT_USB_INTR ."#
            (Read/Write Local) ( ViInt16) [static as N/A in -1, 81h to 8Fh]

            const VI_ATTR_USB_INTR_IN_STATUS: r#"VI_ATTR_USB_INTR_IN_STATUS specifies whether the USB interrupt-in pipe used by the given session is stalled or ready. This attribute can be set to only VI_USB_PIPE_READY ."#
            (Read/Write Local) ( ViInt16) [static as N/A in VI_USB_PIPE_STATE_UNKNOWN (-1) VI_USB_PIPE_READY (0) VI_USB_PIPE_STALLED (1)]

            const VI_ATTR_USB_NUM_INTFCS: r#"VI_ATTR_USB_NUM_INTFCS specifies the number of interfaces supported by this USB device."#
            (Read Only Global) ( ViInt16) [static as N/A in 1 to FFh]

            const VI_ATTR_USB_NUM_PIPES: r#"VI_ATTR_USB_NUM_PIPES specifies the number of pipes supported by this USB interface. This does not include the default control pipe."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to 30]

            const VI_ATTR_USB_SUBCLASS: r#"VI_ATTR_USB_SUBCLASS specifies the USB subclass used by this USB interface."#
            (Read Only Global) ( ViInt16) [static as N/A in 0 to FFh]

            const VI_ATTR_VXI_TRIG_DIR: r#"VI_ATTR_TRIG_DIR is a bit map of the directions of the mapped TTL trigger lines. Bits 0-7 represent TTL triggers 0-7 respectively. A bit's value of 0 means the line is routed out of the frame, and a value of 1 means into the frame. In order for a direction to be set, the line must also be enabled using VI_ATTR_VXI_TRIG_LINES_EN . INSTR Resource VI_ATTR_VXI_TRIG_LINES_EN"#
            (Read/Write Global) ( ViUInt16) [static as 0 in N/A]

            const VI_ATTR_VXI_TRIG_LINES_EN: r#"VI_ATTR_VXI_TRIG_LINES_EN is a bit map of what VXI TLL triggers have mappings. Bits 0-7 represent TTL triggers 0-7 respectively. A bit's value of 0 means the trigger line is unmapped, and 1 means a mapping exists. Use VI_ATTR_VXI_TRIG_DIR to set an enabled line's direction. INSTR Resource VI_ATTR_VXI_TRIG_DIR"#
            (Read/Write Global) ( ViUInt16) [static as 0 in N/A]

            const VI_ATTR_ASRL_ALLOW_TRANSMIT: r#"If set to VI_FALSE, it suspends transmission as if an XOFF character has been received. If set to VI_TRUE , it resumes transmission as if an XON character has been received. If XON/XOFF flow control (software handshaking) is not being used, it is invalid to set this attribute to VI_FALSE ."#
            (Read/Write Global) ( ViBoolean) [static as VI_TRUE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_ASRL_BREAK_LEN: r#"This controls the duration (in milliseconds) of the break signal asserted when VI_ATTR_ASRL_END_OUT is set to VI_ASRL_END_BREAK . If you want to control the assertion state and length of a break signal manually, use the VI_ATTR_ASRL_BREAK_STATE attribute instead."#
            (Read/Write Local) ( ViInt16) [static as 250 in 1-500]

            const VI_ATTR_ASRL_BREAK_STATE: r#"If set to VI_STATE_ASSERTED , it suspends character transmission and places the transmission line in a break state until this attribute is reset to VI_STATE_UNASSERTED . This attribute lets you manually control the assertion state and length of a break signal. If you want VISA to send a break signal after each write operation automatically, use the VI_ATTR_ASRL_BREAK_LEN and VI_ATTR_ASRL_END_OUT attributes instead."#
            (Read/Write Global) ( ViInt16) [static as VI_STATE_UNASSERTED in VI_STATE_ASSERTED (1) VI_STATE_UNASSERTED (0) VI_STATE_UNKNOWN ( - 1)]

            const VI_ATTR_ASRL_CONNECTED: r#"VI_ATTR_ASRL_CONNECTED indicates whether the port is properly connected to another port or device. This attribute is valid only with serial drivers developed by National Instruments and documented to support this feature with the corresponding National Instruments hardware."#
            (Read Only Global) ( ViBoolean) [static as N/A in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_ASRL_DISCARD_NULL: r#"If set to VI_TRUE , NUL characters are discarded. Otherwise, they are treated as normal data characters. For binary transfers, set this attribute to VI_FALSE ."#
            (Read/Write Global) ( ViBoolean) [static as VI_FALSE in VI_TRUE (1) VI_FALSE (0)]

            const VI_ATTR_ASRL_WIRE_MODE: r#"VI_ATTR_ASRL_WIRE_MODE represents the current wire/transceiver mode. For RS-485 hardware, this attribute is valid only with the RS-485 serial driver developed by National Instruments. For RS-232 hardware, the values RS232/DCE and RS232/AUTO are valid only with RS-232 serial drivers developed by National Instruments and documented to support this feature with the corresponding National Instruments hardware. When this feature is not supported, RS232/DTE is the only valid value. RS-232 settings: (Windows) RS-485 settings: (Linux) RS-485 settings:"#
            (Read/Write Global) ( ViInt16) [static as N/A in VI_ASRL_WIRE_485_4 (0) VI_ASRL_WIRE_485_2_DTR_ECHO (1) VI_ASRL_WIRE_485_2_DTR_CTRL (2) VI_ASRL_WIRE_485_2_AUTO (3) VI_ASRL_WIRE_232_DTE (128) VI_ASRL_WIRE_232_DCE (129) VI_ASRL_WIRE_232_AUTO (130) VI_STATE_UNKNOWN (-1)]
            */
        }
    }
}
