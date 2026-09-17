// This test executable overrides the dynamically linked VISA symbols on macOS.
// No hardware or VISA sessions are opened. Other platforms still compile the adapter.
#![cfg(all(feature = "tokio", target_os = "macos"))]
#![allow(non_snake_case)]

use std::{
    ffi::c_void,
    future::Future,
    io,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll, Wake, Waker},
};
use tokio::io::{AsyncWrite, AsyncWriteExt};
use visa_rs::{session::FromRawSs, vs, Instrument};

#[derive(Default)]
struct Mock {
    handler: vs::ViHndlr,
    user: usize,
    job: vs::ViJobId,
    ptr: usize,
    len: usize,
    count: usize,
    status: vs::ViStatus,
    submit_error: vs::ViStatus,
    uninstall_error: vs::ViStatus,
    enable_error: vs::ViStatus,
    close_error: vs::ViStatus,
    uninstalls: usize,
    synchronous: bool,
    submissions: Vec<Vec<u8>>,
    flushes: usize,
    terminations: usize,
}

static MOCK: Mutex<Option<Mock>> = Mutex::new(None);

fn with_mock<T>(f: impl FnOnce(&mut Mock) -> T) -> T {
    f(MOCK.lock().unwrap().as_mut().unwrap())
}

fn complete(count: usize, status: vs::ViStatus) {
    let (handler, user) = with_mock(|m| {
        m.count = count;
        m.status = status;
        // Read the submitted pointer at completion, after the caller can have freed
        // or changed its original input. The adapter must still own these bytes.
        let bytes = unsafe { std::slice::from_raw_parts(m.ptr as *const u8, m.len) };
        assert_eq!(bytes, m.submissions.last().unwrap());
        (m.handler.unwrap(), m.user)
    });
    unsafe { handler(42, vs::VI_EVENT_IO_COMPLETION, 77, user as *mut c_void) };
}

#[no_mangle]
unsafe extern "system" fn viInstallHandler(
    _: vs::ViSession,
    _: vs::ViEventType,
    handler: vs::ViHndlr,
    user: *mut c_void,
) -> vs::ViStatus {
    with_mock(|m| {
        m.handler = handler;
        m.user = user as usize;
    });
    0
}

#[no_mangle]
unsafe extern "system" fn viUninstallHandler(
    _: vs::ViSession,
    _: vs::ViEventType,
    _: vs::ViHndlr,
    user: *mut c_void,
) -> vs::ViStatus {
    with_mock(|m| {
        assert_eq!(user as usize, m.user);
        m.uninstalls += 1;
        if m.uninstall_error == 0 {
            m.handler = None;
        }
        m.uninstall_error
    })
}

#[no_mangle]
unsafe extern "system" fn viEnableEvent(
    _: vs::ViSession,
    _: vs::ViEventType,
    _: vs::ViUInt16,
    _: vs::ViEventFilter,
) -> vs::ViStatus {
    with_mock(|m| m.enable_error)
}

#[no_mangle]
unsafe extern "system" fn viDisableEvent(
    _: vs::ViSession,
    _: vs::ViEventType,
    _: vs::ViUInt16,
) -> vs::ViStatus {
    0
}

#[no_mangle]
unsafe extern "system" fn viSetAttribute(
    _: vs::ViObject,
    _: vs::ViAttr,
    _: vs::ViAttrState,
) -> vs::ViStatus {
    0
}

#[no_mangle]
unsafe extern "system" fn viClose(_: vs::ViObject) -> vs::ViStatus {
    with_mock(|m| m.close_error)
}

#[no_mangle]
unsafe extern "system" fn viGetAttribute(
    _: vs::ViObject,
    attr: vs::ViAttr,
    out: *mut c_void,
) -> vs::ViStatus {
    with_mock(|m| match attr {
        vs::VI_ATTR_JOB_ID => *(out as *mut vs::ViJobId) = m.job,
        vs::VI_ATTR_STATUS => *(out as *mut vs::ViStatus) = m.status,
        vs::VI_ATTR_RET_COUNT => *(out as *mut vs::ViUIntPtrSize) = m.count as _,
        _ => panic!("unexpected attribute {attr:#x}"),
    });
    0
}

#[no_mangle]
unsafe extern "system" fn viWriteAsync(
    _: vs::ViSession,
    buf: *const u8,
    count: vs::ViUInt32,
    job: *mut vs::ViJobId,
) -> vs::ViStatus {
    let (error, synchronous) = with_mock(|m| {
        let error = std::mem::take(&mut m.submit_error);
        if error == 0 {
            m.job += 1;
            *job = m.job;
            m.ptr = buf as usize;
            m.len = count as usize;
            m.submissions
                .push(std::slice::from_raw_parts(buf, count as usize).to_vec());
        }
        (error, m.synchronous)
    });
    if error != 0 {
        return error;
    }
    if synchronous {
        complete(count as usize, 0);
        vs::VI_SUCCESS_SYNC as _
    } else {
        0
    }
}

#[no_mangle]
unsafe extern "system" fn viFlush(_: vs::ViSession, _: vs::ViUInt16) -> vs::ViStatus {
    with_mock(|m| m.flushes += 1);
    0
}

#[no_mangle]
unsafe extern "system" fn viTerminate(
    _: vs::ViSession,
    _: vs::ViUInt16,
    job: vs::ViJobId,
) -> vs::ViStatus {
    with_mock(|m| {
        assert_eq!(job, m.job);
        m.terminations += 1;
    });
    complete(0, vs::VI_ERROR_ABORT);
    0
}

#[derive(Default)]
struct WakeCount(AtomicUsize);

impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn owned_writes_handle_cancellation_completion_and_errors() {
    *MOCK.lock().unwrap() = Some(Mock::default());
    let wakes = Arc::new(WakeCount::default());
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);
    let mut adapter = unsafe { Instrument::from_raw_ss(42) }
        .into_tokio_async()
        .unwrap();

    let mut input = b"abcd".to_vec();
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, &input),
        Poll::Ready(Ok(4))
    ));
    input.fill(b'!');
    drop(input);

    // A new future cannot accept anything yet. Dropping it must not cancel or
    // overwrite the previous, already accepted transfer.
    let mut cancelled = Box::pin(adapter.write(b"discard"));
    assert!(cancelled.as_mut().poll(&mut cx).is_pending());
    assert!(cancelled.as_mut().poll(&mut cx).is_pending());
    drop(cancelled);
    assert_eq!(with_mock(|m| m.submissions.len()), 1);
    assert_eq!(with_mock(|m| m.terminations), 0);
    assert!(Pin::new(&mut adapter).poll_flush(&mut cx).is_pending());
    assert_eq!(with_mock(|m| m.flushes), 0);

    complete(4, 0);
    assert!(wakes.0.load(Ordering::SeqCst) > 0);
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"x"),
        Poll::Ready(Ok(1))
    ));
    assert_eq!(
        with_mock(|m| m.submissions.clone()),
        vec![b"abcd".to_vec(), b"x".to_vec()]
    );
    assert!(Pin::new(&mut adapter).poll_shutdown(&mut cx).is_pending());
    complete(1, 0);
    assert!(matches!(
        Pin::new(&mut adapter).poll_shutdown(&mut cx),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(with_mock(|m| m.flushes), 1);

    // Deferred VISA errors are returned before accepting another buffer.
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"bad"),
        Poll::Ready(Ok(3))
    ));
    complete(0, vs::VI_ERROR_TMO);
    let before = with_mock(|m| m.submissions.len());
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"new"),
        Poll::Ready(Err(_))
    ));
    assert_eq!(with_mock(|m| m.submissions.len()), before);

    // Flush must not silently succeed for short or failed transfers.
    for (count, status) in [(2, 0), (0, 0), (5, 0), (0, vs::VI_ERROR_TMO)] {
        assert!(matches!(
            Pin::new(&mut adapter).poll_write(&mut cx, b"abcd"),
            Poll::Ready(Ok(4))
        ));
        complete(count, status);
        let result = Pin::new(&mut adapter).poll_flush(&mut cx);
        assert!(matches!(result, Poll::Ready(Err(_))));
        if count == 0 && status == 0 {
            assert!(matches!(result, Poll::Ready(Err(e)) if e.kind() == io::ErrorKind::WriteZero));
        }
    }

    // Submission failure accepts nothing; inline completion is collected by flush.
    with_mock(|m| m.submit_error = vs::VI_ERROR_IN_PROGRESS);
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"new"),
        Poll::Ready(Err(_))
    ));
    with_mock(|m| m.synchronous = true);
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"sync"),
        Poll::Ready(Ok(4))
    ));
    assert!(matches!(
        Pin::new(&mut adapter).poll_flush(&mut cx),
        Poll::Ready(Ok(()))
    ));
    let before = with_mock(|m| m.submissions.len());
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b""),
        Poll::Ready(Ok(0))
    ));
    assert_eq!(with_mock(|m| m.submissions.len()), before);

    with_mock(|m| m.synchronous = false);
    assert!(matches!(
        Pin::new(&mut adapter).poll_write(&mut cx, b"drop"),
        Poll::Ready(Ok(4))
    ));
    drop(adapter);
    assert_eq!(with_mock(|m| m.terminations), 1);

    // Direct async writes still wait for completion and preserve short counts.
    *MOCK.lock().unwrap() = Some(Mock::default());
    let instr = unsafe { Instrument::from_raw_ss(42) }.into_async().unwrap();
    let mut write = Box::pin(instr.async_write(b"abcd"));
    assert!(write.as_mut().poll(&mut cx).is_pending());
    complete(3, 0);
    assert!(matches!(write.as_mut().poll(&mut cx), Poll::Ready(Ok(3))));
    drop(write);
    assert_eq!(with_mock(|m| m.terminations), 0);
    let mut empty = Box::pin(instr.async_write(Vec::new()));
    assert!(matches!(empty.as_mut().poll(&mut cx), Poll::Ready(Ok(0))));
    drop(empty);
    assert_eq!(with_mock(|m| m.submissions.len()), 1);
    let mut cancelled = Box::pin(instr.async_write(b"cancel"));
    assert!(cancelled.as_mut().poll(&mut cx).is_pending());
    drop(cancelled);
    assert_eq!(with_mock(|m| m.terminations), 1);
    drop(instr);

    // A failed uninstall must retain the original map (including its waker),
    // not just leak the buffer. A later callback may still use the same user_data.
    for convert in [true, false] {
        *MOCK.lock().unwrap() = Some(Mock {
            uninstall_error: vs::VI_ERROR_SYSTEM_ERROR,
            // Exercise drop without relying on a successful session close either.
            close_error: vs::VI_ERROR_SYSTEM_ERROR,
            ..Mock::default()
        });
        let instr = unsafe { Instrument::from_raw_ss(42) }.into_async().unwrap();
        let mut pending = Box::pin(instr.async_write(b"held"));
        let baseline = Arc::strong_count(&wakes);
        assert!(pending.as_mut().poll(&mut cx).is_pending());
        assert_eq!(Arc::strong_count(&wakes), baseline + 1);
        // Forget is safe Rust: the session must protect outstanding buffers even
        // when a future's Drop is skipped, and conversion is now allowed.
        std::mem::forget(pending);
        if convert {
            let plain: Instrument = instr.into();
            assert_eq!(Arc::strong_count(&wakes), baseline + 1);
            complete(4, 0);
            drop(plain);
        } else {
            drop(instr);
            assert_eq!(Arc::strong_count(&wakes), baseline + 1);
            complete(4, 0);
        }
        assert_eq!(Arc::strong_count(&wakes), baseline);
        assert_eq!(with_mock(|m| m.uninstalls), 1);
    }

    // Installation succeeded but enabling failed: constructor cleanup must also
    // attempt uninstall, including when that attempt itself fails.
    for uninstall_error in [0, vs::VI_ERROR_SYSTEM_ERROR] {
        *MOCK.lock().unwrap() = Some(Mock {
            enable_error: vs::VI_ERROR_SYSTEM_ERROR,
            uninstall_error,
            ..Mock::default()
        });
        assert!(unsafe { Instrument::from_raw_ss(42) }.into_async().is_err());
        assert_eq!(with_mock(|m| m.uninstalls), 1);
        assert_eq!(with_mock(|m| m.handler.is_some()), uninstall_error != 0);
    }
}
