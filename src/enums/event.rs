//!
//! [EventKind] defined in VISA Library 7.1 specification,
//! corresponding attributes defined in [attribute](super::attribute) mod,
//! doc from [NI-VISA Product Documentation](https://www.ni.com/docs/en-US/bundle/ni-visa/page/ni-visa/events.html), 
//! with some difference ignored
//! 
//!


use visa_sys as vs;

pub use event_kind::*;

mod event_kind {
    #![allow(overflowing_literals)]
    consts_to_enum! {
        pub enum EventKind:u32 {
            VI_EVENT_IO_COMPLETION      0x3FFF2009 r#"
This event notifies the application that an asynchronous operation has completed.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | Unique logical identifier of the event. This attribute always has the value of VI_EVENT_IO_COMPLETION for this event type.
VI_ATTR_STATUS      | Contains the return code of the asynchronous I/O operation that has completed.
VI_ATTR_JOB_ID      | Contains the job ID of the asynchronous operation that has completed.
VI_ATTR_BUFFER      | Contains the address of the buffer that was used in the asynchronous operation.
VI_ATTR_RET_COUNT   | Contains the actual number of elements that were asynchronously transferred.
VI_ATTR_OPER_NAME   | Contains the name of the operation generating the event.
"#
            VI_EVENT_TRIG               0xBFFF200A r#"
This event notifies the application that a trigger interrupt was received from the device. This may be either a hardware or software trigger, depending on the interface and the current session settings.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | Unique logical identifier of the event. This attribute always has the value of VI_EVENT_TRIG for this event type.
VI_ATTR_RECV_TRIG_ID| The identifier of the triggering mechanism on which the specified trigger event was received.
"#
            VI_EVENT_SERVICE_REQ        0x3FFF200B r#"
This event notifies the application that a service request was received from the device or interface associated with the given session.
*Note*: When you receive a VI_EVENT_SERVICE_REQ on an instrument session, you must call viReadSTB() to guarantee delivery of future service request events on the given session.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | Unique logical identifier of the event. This attribute always has the value of VI_EVENT_SERVICE_REQ for this event type.
"#
            VI_EVENT_CLEAR              0x3FFF200D r#"
Notification that the local controller has been sent a device clear message.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event.
"#
            VI_EVENT_EXCEPTION          0xBFFF200E r#"
This event notifies the application that an error condition has occurred during an operation invocation. In VISA, exceptions are defined as events. The exception-handling model follows the event-handling model for callbacks, and is like any other event in VISA, except that the queueing and suspended handler mechanisms are not allowed.  

A VISA operation generating an exception blocks until the exception handler execution is completed. However, an exception handler sometimes may prefer to terminate the program prematurely without returning the control to the operation generating the exception. VISA does not preclude an application from using a platform-specific or language-specific exception handling mechanism from within the VISA exception handler. For example, the C++ try/catch block can be used in an application in conjunction with the C++ throw mechanism from within the VISA exception handler.  

One situation in which an exception event will not be generated is in the case of asynchronous operations. If the error is detected after the operation is posted—once the asynchronous portion has begun—the status is returned normally via the I/O completion event. However, if an error occurs before the asynchronous portion begins—the error is returned from the asynchronous operation itself—then the exception event will still be raised. This deviation is due to the fact that asynchronous operations already raise an event when they complete, and this I/O completion event may occur in the context of a separate thread previously unknown to the application. In summary, a single application event handler can easily handle error conditions arising from both exception events and failed asynchronous operations.  
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event. This attribute always has the value of VI_EVENT_EXCEPTION for this event type.
VI_ATTR_STATUS      |   Contains the status code returned by the operation generating the error.
VI_ATTR_OPER_NAME   |   Contains the name of the operation generating the event.
"#
            VI_EVENT_GPIB_CIC           0x3FFF2012 r#"
Notification that the GPIB controller has gained or lost CIC (controller in charge) status.
Attribute Name              |	Description
-------------               | ------------------------------------------
VI_ATTR_EVENT_TYPE          | 	Unique logical identifier of the event.
VI_ATTR_GPIB_RECV_CIC_STATE |   Specifies whether the CIC status was gained or lost.
"#
            VI_EVENT_GPIB_TALK          0x3FFF2013 r#"
Notification that the GPIB controller has been addressed to talk.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event.
"#
            VI_EVENT_GPIB_LISTEN        0x3FFF2014 r#"
Notification that the GPIB controller has been addressed to listen.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event.
"#
            VI_EVENT_VXI_VME_SYSFAIL    0x3FFF201D r#"
Notification that the VXI/VME SYSFAIL* line has been asserted.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event.
"#
            VI_EVENT_VXI_VME_SYSRESET   0x3FFF201E r#"
Notification that the VXI/VME SYSRESET* line has been asserted.
Attribute Name      |	Description
-------------       | ------------------------------------------
VI_ATTR_EVENT_TYPE  | 	Unique logical identifier of the event.
"#
            VI_EVENT_VXI_SIGP           0x3FFF2020 r#"
This event notifies the application that a VXIbus signal or VXIbus interrupt was received from the device associated with the given session.
Attribute Name          |	Description
-------------           | ------------------------------------------
VI_ATTR_EVENT_TYPE      | 	Unique logical identifier of the event. This attribute always has the value of VI_EVENT_VXI_SIGP for this event type.
VI_ATTR_SIGP_STATUS_ID  |   The 16-bit Status/ID value retrieved during the IACK cycle or from the Signal register.
"#
            VI_EVENT_VXI_VME_INTR       0xBFFF2021 r#"
This event notifies the application that a VXIbus interrupt was received from the device associated with the given session.
Attribute Name          |	Description
-------------           | ------------------------------------------
VI_ATTR_EVENT_TYPE      | 	Unique logical identifier of the event. This attribute always has the value of VI_EVENT_VXI_VME_INTR for this event type.
VI_ATTR_INTR_STATUS_ID  |   The 32-bit Status/ID value retrieved during the IACK cycle.
VI_ATTR_RECV_INTR_LEVEL |   The VXI interrupt level on which the interrupt was received.
"#
            VI_EVENT_PXI_INTR           0x3FFF2022 r#"
This event notifies that a PXI interrupt has occurred.
Attribute Name              |	Description
-------------               | ------------------------------------------
VI_ATTR_EVENT_TYPE          | 	Unique logical identifier of the event.
VI_ATTR_PXI_RECV_INTR_SEQ   |   The index of the interrupt sequence that detected the interrupt condition.
VI_ATTR_PXI_RECV_INTR_DATA  |   The first PXI/PCI register that was read in the successful interrupt detection sequence.
"#
            VI_EVENT_TCPIP_CONNECT      0x3FFF2036 r#"

"#
            VI_EVENT_USB_INTR           0x3FFF2037 r#"
This event notifies that a USB interrupt has occurred.
Attribute Name              |	Description
-------------               | ------------------------------------------
VI_ATTR_EVENT_TYPE          | Unique logical identifier of the event.
VI_ATTR STATUS              | Contains the status code returned by this event.
VI_ATTR_USB_RECV_INTR_SIZE  | The number of bytes of USB interrupt data that is stored.
VI_ATTR_USB_RECV_INTR_DATA  | The actual received data from the USB Interrupt.
"#
            VI_ALL_ENABLED_EVENTS       0x3FFF7FFF r#"
Specifying VI_ALL_ENABLED_EVENTS in viEnableEvent for the eventType parameter refers to all events which have previously been enabled on this session, making it easier to switch between the two callback mechanisms for multiple events.
"#
        }
    }
}

#[repr(i16)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub enum Mechanism {
    Queue = vs::VI_QUEUE as _,
    Handler = vs::VI_HNDLR as _,
    SuspendHandler = vs::VI_SUSPEND_HNDLR as _,
    AllMech = vs::VI_ALL_MECH as _,
}

#[repr(i16)]
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub enum EventFilter {
    Null = vs::VI_NULL as _,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Event {
    pub(crate) handler: vs::ViEvent,
    pub(crate) kind: EventKind,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        self.kind
    }
    pub(crate) fn new(handler: vs::ViEvent, kind: vs::ViEventType) -> Self {
        Self {
            handler,
            kind: EventKind::try_from(kind).expect("should be valid event kind"),
        }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            vs::viClose(self.handler);
        }
    }
}

impl PartialEq<EventKind> for Event {
    fn eq(&self, other: &EventKind) -> bool {
        self.kind.eq(other)
    }
}

impl crate::session::AsRawSs for Event {
    fn as_raw_ss(&self) -> crate::session::RawSs {
        self.handler
    }
}
