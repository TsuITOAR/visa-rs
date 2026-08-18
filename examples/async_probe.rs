//! Checks whether your VISA implementation really defers asynchronous transfers.
//!
//! `viReadAsync` is allowed to perform a transfer synchronously and report
//! `VI_SUCCESS_SYNC`. Some implementations always do, which makes `AsyncInstrument`'s
//! futures block the calling thread for up to the session timeout. This probe calls the
//! C API directly -- no code from this crate is in the measured path -- so its result is
//! about your VISA, not about `visa-rs`.
//!
//! Run it against a loopback socket (default) or a real instrument:
//!
//! ```text
//! cargo run --example async_probe
//! cargo run --example async_probe -- "GPIB0::12::INSTR"
//! ```
//!
//! `VI_TMO_INFINITE` is deliberately not probed: where the transfer runs inline it never
//! returns at all.

use std::{
    ffi::CString,
    io::Write,
    net::TcpListener,
    sync::OnceLock,
    thread::{self, ThreadId},
    time::{Duration, Instant},
};
use visa_rs::vs;

const VI_ATTR_TMO_VALUE: u32 = 0x3FFF_001A;
const VI_EVENT_IO_COMPLETION: u32 = 0x3FFF_2009;
const VI_SUCCESS_SYNC: i32 = 0x3FFF_009B;

static CALLER: OnceLock<ThreadId> = OnceLock::new();
static HANDLER: OnceLock<ThreadId> = OnceLock::new();

unsafe extern "system" fn handler(
    _vi: vs::ViSession,
    _et: vs::ViEventType,
    _ev: vs::ViEvent,
    _ud: *mut std::ffi::c_void,
) -> vs::ViStatus {
    let _ = HANDLER.set(thread::current().id());
    0
}

/// A peer that accepts, waits, then answers, so a read can plausibly still be pending.
fn loopback_peer() -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut s, _)) = l.accept() {
            thread::sleep(Duration::from_millis(150));
            let _ = s.write_all(b"PROBE\n");
            thread::sleep(Duration::from_secs(5));
        }
    });
    format!("TCPIP0::127.0.0.1::{}::SOCKET", port)
}

fn main() {
    CALLER.set(thread::current().id()).unwrap();
    let resource = std::env::args().nth(1).unwrap_or_else(loopback_peer);
    println!("probing {resource}\n");

    unsafe {
        let mut rm: vs::ViSession = 0;
        assert!(vs::viOpenDefaultRM(&mut rm) >= 0, "viOpenDefaultRM failed");

        for (mech_name, mech) in [("VI_HNDLR", 2u16), ("VI_SUSPEND_HNDLR", 4), ("VI_QUEUE", 1)] {
            for tmo in [0u32, 400] {
                let name = CString::new(resource.clone()).unwrap();
                let mut vi: vs::ViSession = 0;
                if vs::viOpen(rm, name.as_ptr(), 0, 3000, &mut vi) < 0 {
                    println!("  viOpen failed; is the resource name right?");
                    return;
                }
                vs::viSetAttribute(vi, VI_ATTR_TMO_VALUE, tmo as _);
                vs::viInstallHandler(
                    vi,
                    VI_EVENT_IO_COMPLETION,
                    Some(handler),
                    std::ptr::null_mut(),
                );
                if vs::viEnableEvent(vi, VI_EVENT_IO_COMPLETION, mech, 0) < 0 {
                    println!("{mech_name:<17} tmo={tmo:<5} enable unsupported");
                    vs::viClose(vi);
                    continue;
                }

                let buf = Box::leak(vec![0u8; 4096].into_boxed_slice());
                let mut job: vs::ViJobId = 0;
                let t = Instant::now();
                let st = vs::viReadAsync(vi, buf.as_mut_ptr(), 4096, &mut job);
                let elapsed = t.elapsed();

                let verdict = if st < 0 {
                    "error"
                } else if st == VI_SUCCESS_SYNC {
                    "ran INLINE (not deferred)"
                } else {
                    "deferred (job queued)"
                };
                println!("{mech_name:<17} tmo={tmo:<5} {elapsed:>12?}  {verdict}");
                if st >= 0 && st != VI_SUCCESS_SYNC {
                    vs::viTerminate(vi, 0, job);
                }
                vs::viClose(vi);
            }
        }

        match HANDLER.get() {
            Some(h) if Some(h) == CALLER.get() => println!(
                "\nhandler ran on the calling thread: this VISA has no event thread, so \
                 there is nothing to wake a future from"
            ),
            Some(_) => println!("\nhandler ran on a VISA-owned thread: push notification works"),
            None => println!("\nhandler never ran"),
        }
    }
}
