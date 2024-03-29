//! Visa status code and corresponding meaning,
//! comes from [official NI-visa document](https://www.ni.com/docs/en-US/bundle/ni-visa/page/ni-visa/completion_codes.html),
//!
pub use completion::CompletionCode;
pub use error::ErrorCode;
mod error {
    #![allow(overflowing_literals)]
    #![allow(non_upper_case_globals)]
    consts_to_enum! {
        #[format=doc]
        #[repr(ViStatus)]
        pub enum ErrorCode{
            //Completion Codes          Values	    Meaning
            VI_ERROR_SYSTEM_ERROR	    0xBFFF0000	"Unknown system error (miscellaneous error)."
            VI_ERROR_INV_OBJECT	        0xBFFF000E	"The given session or object reference is invalid."
            VI_ERROR_RSRC_LOCKED	    0xBFFF000F	"Specified type of lock cannot be obtained or specified operation cannot be performed, because the resource is locked."
            VI_ERROR_INV_EXPR	        0xBFFF0010	"Invalid expression specified for search."
            VI_ERROR_RSRC_NFOUND	    0xBFFF0011	"Insufficient location information or the device or resource is not present in the system."
            VI_ERROR_INV_RSRC_NAME	    0xBFFF0012	"Invalid resource reference specified. Parsing error."
            VI_ERROR_INV_ACC_MODE	    0xBFFF0013	"Invalid access mode."
            VI_ERROR_TMO	            0xBFFF0015	"Timeout expired before operation completed."
            VI_ERROR_CLOSING_FAILED	    0xBFFF0016	"Unable to deallocate the previously allocated data structures corresponding to this session or object reference."
            VI_ERROR_INV_DEGREE	        0xBFFF001B	"Specified degree is invalid."
            VI_ERROR_INV_JOB_ID	        0xBFFF001C	"Specified job identifier is invalid."
            VI_ERROR_NSUP_ATTR	        0xBFFF001D	"The specified attribute is not defined or supported by the referenced session, event, or find list."
            VI_ERROR_NSUP_ATTR_STATE    0xBFFF001E	"The specified state of the attribute is not valid, or is not supported as defined by the session, event, or find list."
            VI_ERROR_ATTR_READONLY	    0xBFFF001F	"The specified attribute is Read Only."
            VI_ERROR_INV_LOCK_TYPE	    0xBFFF0020	"The specified type of lock is not supported by this resource."
            VI_ERROR_INV_ACCESS_KEY	    0xBFFF0021	"The access key to the resource associated with this session is invalid."
            VI_ERROR_INV_EVENT	        0xBFFF0026	"Specified event type is not supported by the resource."
            VI_ERROR_INV_MECH	        0xBFFF0027	"Invalid mechanism specified."
            VI_ERROR_HNDLR_NINSTALLED	0xBFFF0028	"A handler is not currently installed for the specified event."
            VI_ERROR_INV_HNDLR_REF	    0xBFFF0029	"The given handler reference is invalid."
            VI_ERROR_INV_CONTEXT	    0xBFFF002A	"Specified event context is invalid."
            VI_ERROR_QUEUE_OVERFLOW	    0xBFFF002D	"The event queue for the specified type has overflowed (usually due to previous events not having been closed)."
            VI_ERROR_NENABLED	        0xBFFF002F	"The session must be enabled for events of the specified type in order to receive them."
            VI_ERROR_ABORT	            0xBFFF0030	"The operation was aborted."
            VI_ERROR_RAW_WR_PROT_VIOL	0xBFFF0034	"Violation of raw write protocol occurred during transfer."
            VI_ERROR_RAW_RD_PROT_VIOL	0xBFFF0035	"Violation of raw read protocol occurred during transfer."
            VI_ERROR_OUTP_PROT_VIOL	    0xBFFF0036	"Device reported an output protocol error during transfer."
            VI_ERROR_INP_PROT_VIOL	    0xBFFF0037	"Device reported an input protocol error during transfer."
            VI_ERROR_BERR	            0xBFFF0038	"Bus error occurred during transfer."
            VI_ERROR_IN_PROGRESS	    0xBFFF0039	"Unable to queue the asynchronous operation because there is already an operation in progress."
            VI_ERROR_INV_SETUP	        0xBFFF003A	"Unable to start operation because setup is invalid (due to attributes being set to an inconsistent state)."
            VI_ERROR_QUEUE_ERROR	    0xBFFF003B	"Unable to queue asynchronous operation (usually due to the I/O completion event not being enabled or insufficient space in the session's queue)."
            VI_ERROR_ALLOC	            0xBFFF003C	"Insufficient system resources to perform necessary memory allocation."
            VI_ERROR_INV_MASK	        0xBFFF003D	"Invalid buffer mask specified."
            VI_ERROR_IO	                0xBFFF003E	"Could not perform operation because of I/O error."
            VI_ERROR_INV_FMT	        0xBFFF003F	"A format specifier in the format string is invalid."
            VI_ERROR_NSUP_FMT	        0xBFFF0041	"A format specifier in the format string is not supported."
            VI_ERROR_LINE_IN_USE	    0xBFFF0042	"The specified trigger line is currently in use."
            VI_ERROR_NSUP_MODE	        0xBFFF0046	"The specified mode is not supported by this VISA implementation."
            VI_ERROR_SRQ_NOCCURRED	    0xBFFF004A	"Service request has not been received for the session."
            VI_ERROR_INV_SPACE	        0xBFFF004E	"Invalid address space specified."
            VI_ERROR_INV_OFFSET	        0xBFFF0051	"Invalid offset specified."
            VI_ERROR_INV_WIDTH	        0xBFFF0052	"Invalid source or destination width specified."
            VI_ERROR_NSUP_OFFSET	    0xBFFF0054	"Specified offset is not accessible from this hardware."
            VI_ERROR_NSUP_VAR_WIDTH	    0xBFFF0055	"Cannot support source and destination widths that are different."
            VI_ERROR_WINDOW_NMAPPED	    0xBFFF0057	"The specified session is not currently mapped."
            VI_ERROR_RESP_PENDING	    0xBFFF0059	"A previous response is still pending, causing a multiple query error."
            VI_ERROR_NLISTENERS	        0xBFFF005F	"No Listeners condition is detected (both NRFD and NDAC are deasserted)."
            VI_ERROR_NCIC	            0xBFFF0060	"The interface associated with this session is not currently the controller in charge."
            VI_ERROR_NSYS_CNTLR	        0xBFFF0061	"The interface associated with this session is not the system controller."
            VI_ERROR_NSUP_OPER	        0xBFFF0067	"The given session or object reference does not support this operation."
            VI_ERROR_INTR_PENDING	    0xBFFF0068	"An interrupt is still pending from a previous call."
            VI_ERROR_ASRL_PARITY	    0xBFFF006A	"A parity error occurred during transfer."
            VI_ERROR_ASRL_FRAMING	    0xBFFF006B	"A framing error occurred during transfer."
            VI_ERROR_ASRL_OVERRUN	    0xBFFF006C	"An overrun error occurred during transfer. A character was not read from the hardware before the next character arrived."
            VI_ERROR_TRIG_NMAPPED	    0xBFFF006E	"The path from trigSrc to trigDest is not currently mapped."
            VI_ERROR_NSUP_ALIGN_OFFSET	0xBFFF0070	"The specified offset is not properly aligned for the access width of the operation."
            VI_ERROR_USER_BUF	        0xBFFF0071	"A specified user buffer is not valid or cannot be accessed for the required size."
            VI_ERROR_RSRC_BUSY	        0xBFFF0072	"The resource is valid, but VISA cannot currently access it."
            VI_ERROR_NSUP_WIDTH	        0xBFFF0076	"Specified width is not supported by this hardware."
            VI_ERROR_INV_PARAMETER	    0xBFFF0078	"The value of some parameter—which parameter is not known—is invalid."
            VI_ERROR_INV_PROT	        0xBFFF0079	"The protocol specified is invalid."
            VI_ERROR_INV_SIZE	        0xBFFF007B	"Invalid size of window specified."
            VI_ERROR_WINDOW_MAPPED	    0xBFFF0080	"The specified session currently contains a mapped window."
            VI_ERROR_NIMPL_OPER	        0xBFFF0081	"The given operation is not implemented."
            VI_ERROR_INV_LENGTH	        0xBFFF0083	"Invalid length specified."
            VI_ERROR_INV_MODE	        0xBFFF0091	"The specified mode is invalid."
            VI_ERROR_SESN_NLOCKED	    0xBFFF009C	"The current session did not have any lock on the resource."
            VI_ERROR_MEM_NSHARED	    0xBFFF009D	"The device does not export any memory."
            VI_ERROR_LIBRARY_NFOUND	    0xBFFF009E	"A code library required by VISA could not be located or loaded."
            VI_ERROR_NSUP_INTR	        0xBFFF009F	"The interface cannot generate an interrupt on the requested level or with the requested statusID value."
            VI_ERROR_INV_LINE	        0xBFFF00A0	"The value specified by the line parameter is invalid."
            VI_ERROR_FILE_ACCESS	    0xBFFF00A1	"An error occurred while trying to open the specified file. Possible reasons include an invalid path or lack of access rights."
            VI_ERROR_FILE_IO	        0xBFFF00A2	"An error occurred while performing I/O on the specified file."
            VI_ERROR_NSUP_LINE	        0xBFFF00A3	"One of the specified lines (trigSrc or trigDest) is not supported by this VISA implementation, or the combination of lines is not a valid mapping."
            VI_ERROR_NSUP_MECH	        0xBFFF00A4	"The specified mechanism is not supported for the given event type."
            VI_ERROR_INTF_NUM_NCONFIG	0xBFFF00A5	"The interface type is valid but the specified interface number is not configured."
            VI_ERROR_CONN_LOST	        0xBFFF00A6	"The connection for the given session has been lost."
            VI_ERROR_MACHINE_NAVAIL	    0xBFFF00A7	"The remote machine does not exist or is not accepting any connections."
            VI_ERROR_NPERMISSION	    0xBFFF00A8	"Access to the resource or remote machine is denied. This is due to lack of sufficient privileges for the current user or machine."
        }
    }
}
mod completion {
    #![allow(non_upper_case_globals)]
    consts_to_enum! {
        #[format=doc]
        #[repr(ViStatus)]
        pub enum CompletionCode{
            VI_SUCCESS	                0x00000000  "Operation completed successfully."
            VI_SUCCESS_EVENT_EN	        0x3FFF0002	"Specified event is already enabled for at least one of the specified mechanisms."
            VI_SUCCESS_EVENT_DIS	    0x3FFF0003	"Specified event is already disabled for at least one of the specified mechanisms."
            VI_SUCCESS_QUEUE_EMPTY	    0x3FFF0004	"Operation completed successfully, but queue was already empty."
            VI_SUCCESS_TERM_CHAR	    0x3FFF0005	"The specified termination character was read."
            VI_SUCCESS_MAX_CNT	        0x3FFF0006	"The number of bytes read is equal to the input count."
            VI_WARN_QUEUE_OVERFLOW	    0x3FFF000C	"The event returned is valid. One or more events that occurred have not been raised because there was no room available on the queue at the time of their occurrence. This could happen because VI_ATTR_MAX_QUEUE_LENGTH is not set to a large enough value for your application and/or events are coming in faster than you are servicing them."
            VI_WARN_CONFIG_NLOADED	    0x3FFF0077	"The specified configuration either does not exist or could not be loaded; using VISA-specified defaults."
            VI_SUCCESS_DEV_NPRESENT	    0x3FFF007D	"Session opened successfully, but the device at the specified address is not responding."
            VI_SUCCESS_TRIG_MAPPED	    0x3FFF007E	"The path from trigSrc to trigDest is already mapped."
            VI_SUCCESS_QUEUE_NEMPTY	    0x3FFF0080	"Wait terminated successfully on receipt of an event notification. There is still at least one more event occurrence of the requested type(s) available for this session."
            VI_WARN_NULL_OBJECT	        0x3FFF0082	"The specified object reference is uninitialized."
            VI_WARN_NSUP_ATTR_STATE	    0x3FFF0084	"Although the specified state of the attribute is valid, it is not supported by this resource implementation."
            VI_WARN_UNKNOWN_STATUS	    0x3FFF0085	"The status code passed to the operation could not be interpreted."
            VI_WARN_NSUP_BUF	        0x3FFF0088	"The specified buffer is not supported."
            VI_SUCCESS_NCHAIN	        0x3FFF0098	"Event handled successfully. Do not invoke any other handlers on this session for this event."
            VI_SUCCESS_NESTED_SHARED	0x3FFF0099	"Operation completed successfully, and this session has nested shared locks."
            VI_SUCCESS_NESTED_EXCLUSIVE	0x3FFF009A	"Operation completed successfully, and this session has nested exclusive locks."
            VI_SUCCESS_SYNC	            0x3FFF009B	"Asynchronous operation request was actually performed synchronously."
            VI_WARN_EXT_FUNC_NIMPL	    0x3FFF00A9	"The operation succeeded, but a lower level driver did not implement the extended functionality."
        }
    }
}

impl TryFrom<super::attribute::AttrStatus> for CompletionCode {
    type Error = ErrorCode;
    fn try_from(value: super::attribute::AttrStatus) -> Result<Self, Self::Error> {
        let status = value.into_inner();
        if let Ok(o) = Self::try_from(status) {
            Ok(o)
        } else {
            Err(ErrorCode::try_from(status).unwrap())
        }
    }
}
