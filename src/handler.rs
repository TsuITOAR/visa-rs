use std::{
    ptr::NonNull,
    sync::mpsc::{Receiver, Sender},
};
use visa_sys as vs;

use crate::{
    event,
    session::{AsRawSs, BorrowedSs, FromRawSs},
    Instrument, Result, SUCCESS,
};

pub trait Callback: Sized {
    type Output;
    fn call(&mut self, instr: &mut Instrument, event: event::Event) -> Result<Self::Output>;
}

impl<F, Out> Callback for F
where
    F: FnMut(&mut Instrument, event::Event) -> Result<Out>,
{
    type Output = Out;
    fn call(&mut self, instr: &mut Instrument, event: event::Event) -> Result<Self::Output> {
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
    fn call(&mut self, instr: &mut Instrument, event: event::Event) -> vs::ViStatus {
        let ret = self.core.call(instr, event);
        match ret {
            Err(e) => e.into(),
            Ok(r) => {
                self.sender.send(r).expect("receiver side should be valid");
                SUCCESS
            }
        }
    }
}

pub(crate) struct CallbackWrapper<F: Callback> {
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
        let mut instr = Instrument::from_raw_ss(instr);
        let ret = pack.call(&mut instr, event::Event::new(event, event_type));
        std::mem::forget(instr);
        // ? no sure yet, in official example session not closed
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

pub struct Handler<'b, F: Callback> {
    instr: BorrowedSs<'b>,
    rec: Receiver<F::Output>,
    event_kind: super::event::EventKind,
    callback: CallbackWrapper<F>,
}

impl<'b, F: Callback> Handler<'b, F> {
    pub(crate) fn new(
        instr: BorrowedSs<'b>,
        event_kind: super::event::EventKind,
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
