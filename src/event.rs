use num_enum::TryFromPrimitive;
use visa_sys as vs;

#[repr(u32)]
#[derive(TryFromPrimitive, Debug, PartialEq, PartialOrd, Eq, Ord, Hash, Clone, Copy)]
pub enum EventKind {
    IoCompletion = vs::VI_EVENT_IO_COMPLETION,
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
