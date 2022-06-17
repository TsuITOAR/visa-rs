//!
//! Defines [`Callback`] trait used in [`Instrument::install_handler`](crate::Instrument::install_handler),
//! which returns a [`Handler`] to manage lifetime of data passed
//!
//!
//!

use std::{
    ptr::NonNull,
    sync::mpsc::{Receiver, Sender},
};
use visa_sys as vs;

use crate::{
    enums::event,
    session::{AsRawSs, BorrowedSs, FromRawSs},
    Instrument, Result, SUCCESS,
};

/// Defines a ability to be passed to [`Instrument::install_handler`](crate::Instrument::install_handler)
pub trait Callback {
    type Output;
    fn call(&mut self, instr: &Instrument, event: &event::Event) -> Self::Output;
}

impl<F, Out> Callback for F
where
    F: FnMut(&Instrument, &event::Event) -> Out,
{
    type Output = Out;
    fn call(&mut self, instr: &Instrument, event: &event::Event) -> Self::Output {
        self(instr, event)
    }
}

struct CallbackPack<F: Callback> {
    sender: Sender<F::Output>,
    core: F,
}

impl<F: Callback> CallbackPack<F> {
    fn from_callback(f: F) -> (Self, Receiver<F::Output>) {
        let (sender, receiver) = std::sync::mpsc::channel();
        (Self { sender, core: f }, receiver)
    }
    fn call(&mut self, instr: &Instrument, event: &event::Event) -> vs::ViStatus {
        //Normally, an application should always return VI_SUCCESS from all callback handlers. If a specific handler does not want other handlers to be invoked for the given event for the given session, it should return VI_SUCCESS_NCHAIN. No return value from a handler on one session will affect callbacks on other sessions. Future versions of VISA (or specific implementations of VISA) may take actions based on other return values, so a user should return VI_SUCCESS from handlers unless there is a specific reason to do otherwise.
        self.sender
            .send(self.core.call(instr, event))
            .expect("receiver side should be valid");
        SUCCESS
    }
}

struct CallbackWrapper<F: Callback> {
    f: NonNull<CallbackPack<F>>,
    //? not sure if reproduce from F would get the same fn pointer, so better hold it
    hold: unsafe extern "C" fn(
        vs::ViSession,
        vs::ViEventType,
        vs::ViEvent,
        *mut std::ffi::c_void,
    ) -> vs::ViStatus,
}
fn split_pack<C: Callback>(
    pack: CallbackPack<C>,
) -> (
    std::ptr::NonNull<CallbackPack<C>>,
    unsafe extern "C" fn(
        vs::ViSession,
        vs::ViEventType,
        vs::ViEvent,
        *mut std::ffi::c_void,
    ) -> vs::ViStatus,
) {
    use std::ffi::c_void;
    let data = Box::into_raw(Box::new(pack));
    unsafe extern "C" fn trampoline<T: Callback>(
        instr: vs::ViSession,
        event_type: vs::ViEventType,
        event: vs::ViEvent,
        user_data: *mut c_void,
    ) -> vs::ViStatus {
        let pack: &mut CallbackPack<T> = &mut *(user_data as *mut CallbackPack<T>);
        let instr = Instrument::from_raw_ss(instr);
        let event = event::Event::new(event, event_type);
        let ret = pack.call(&instr, &event);
        std::mem::forget(event); // The VISA system automatically invokes the viClose() operation on the event context when a user handler returns. Because the event context must still be valid after the user handler returns (so that VISA can free it up), an application should not invoke the viClose() operation on an event context passed to a user handler.
        std::mem::forget(instr); // ? no sure yet, in official example session not closed

        ret
    }

    (
        NonNull::new(data).expect("impossible to pass in a null ptr"),
        trampoline::<C>,
    )
}
impl<F: Callback> CallbackWrapper<F> {
    pub(crate) fn new(f: F) -> (Self, Receiver<F::Output>) {
        let (pack, receiver) = CallbackPack::from_callback(f);
        let (data, fun) = split_pack(pack);
        (Self { f: data, hold: fun }, receiver)
    }
}

/// Lifetime manager for [`Callback`], will uninstall the callback when dropped.
///
/// Internally hold a [`Receiver`](::std::sync::mpsc::Receiver) (accessed by [`Self::receiver`]) to receive output of callback from visa.
pub struct Handler<'b, F: Callback> {
    instr: BorrowedSs<'b>,
    rec: Receiver<F::Output>,
    event_kind: event::EventKind,
    callback: CallbackWrapper<F>,
}

impl<'b, F: Callback> Handler<'b, F> {
    pub(crate) fn new(
        instr: BorrowedSs<'b>,
        event_kind: event::EventKind,
        callback: F,
    ) -> Result<Self> {
        let (callback, rec) = CallbackWrapper::new(callback);
        super::wrap_raw_error_in_unsafe!(vs::viInstallHandler(
            instr.as_raw_ss(),
            event_kind as _,
            Some(callback.hold),
            callback.f.as_ptr() as _
        ))?;
        Ok(Self {
            instr,
            rec,
            event_kind,
            callback,
        })
    }
}

impl<'b, F: Callback> Drop for Handler<'b, F> {
    fn drop(&mut self) {
        unsafe {
            vs::viUninstallHandler(
                self.instr.as_raw_ss(),
                self.event_kind as _,
                Some(self.callback.hold),
                self.callback.f.as_ptr() as _,
            );
            Box::from_raw(self.callback.f.as_ptr());
        }
    }
}

impl<'b, F: Callback> Handler<'b, F> {
    pub fn uninstall(self) {}
}

impl<'b, F: Callback> AsRef<Receiver<F::Output>> for Handler<'b, F> {
    fn as_ref(&self) -> &Receiver<F::Output> {
        &self.rec
    }
}

impl<'b, F: Callback> Handler<'b, F> {
    pub fn receiver(&self) -> &Receiver<F::Output> {
        self.as_ref()
    }
}
