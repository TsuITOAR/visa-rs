use std::{ptr::NonNull, sync::mpsc::Receiver};
use visa_sys as vs;
pub struct Handler<R, F> {
    instr: vs::ViSession,
    rec: Receiver<R>,
    event_kind: super::event::EventKind,
    call: unsafe extern "C" fn(
        vs::ViSession,
        vs::ViEventType,
        vs::ViEvent,
        *mut std::ffi::c_void,
    ) -> vs::ViStatus,
    inner_closure: NonNull<F>,
}

impl<R, F> Handler<R, F> {
    pub(crate) fn new(
        instr: vs::ViSession,
        rec: Receiver<R>,
        event_kind: super::event::EventKind,
        call: unsafe extern "C" fn(
            vs::ViSession,
            vs::ViEventType,
            vs::ViEvent,
            *mut std::ffi::c_void,
        ) -> vs::ViStatus,
        inner_closure: NonNull<F>,
    ) -> Self {
        Self {
            instr,
            rec,
            event_kind,
            call,
            inner_closure,
        }
    }
}

impl<R, F> Drop for Handler<R, F> {
    fn drop(&mut self) {
        let ptr_closure = self.inner_closure.as_ptr();
        unsafe {
            vs::viUninstallHandler(
                self.instr,
                self.event_kind as _,
                Some(self.call),
                ptr_closure as _,
            );
            Box::from_raw(ptr_closure);
        }
    }
}

impl<R, F> Handler<R, F> {
    pub fn uninstall(self) {}
}

impl<R, F> AsMut<Receiver<R>> for Handler<R, F> {
    fn as_mut(&mut self) -> &mut Receiver<R> {
        &mut self.rec
    }
}
