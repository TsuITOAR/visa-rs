use std::{
    ffi::CString,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use visa_rs::{
    enums::event::{self, Event},
    enums::status::ErrorCode,
    flags::AccessMode,
    AsResourceManager, DefaultRM, Error, Instrument, TIMEOUT_IMMEDIATE,
};
fn init_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace"))
        .is_test(true)
        .try_init();
}

fn start_tcp_virtual_resource_idn(
) -> std::io::Result<(u16, thread::JoinHandle<std::io::Result<()>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || -> std::io::Result<()> {
        let (mut stream, _) = listener.accept()?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        let mut buf = [0u8; 1024];
        let mut total = Vec::new();
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total.extend_from_slice(&buf[..n]);
            if total.windows(5).any(|w| w == b"*IDN?") {
                stream.write_all(b"TEST_INSTRUMENT\n")?;
                break;
            }
        }
        Ok(())
    });
    Ok((port, server))
}

fn try_default_rm() -> Result<Option<DefaultRM>> {
    match DefaultRM::new() {
        Ok(rm) => Ok(Some(rm)),
        Err(Error(ErrorCode::ErrorSystemError)) | Err(Error(ErrorCode::ErrorLibraryNfound)) => {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// Searches for `expr`, returning `None` when no such instrument is attached so a
/// hardware-dependent test can skip instead of failing.
fn find_or_skip(rm: &DefaultRM, expr: &str) -> Result<Option<visa_rs::ResList>> {
    match rm.find_res_list(&CString::new(expr)?.into()) {
        Ok(list) => Ok(Some(list)),
        Err(Error(ErrorCode::ErrorRsrcNfound)) => {
            log::warn!("no instrument matching {expr}; skipping");
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// Accepts one connection and never replies, so a read on it stays outstanding.
fn start_tcp_silent() -> std::io::Result<(u16, thread::JoinHandle<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let server = thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            thread::sleep(Duration::from_secs(10));
            drop(stream);
        }
    });
    Ok((port, server))
}

/// The cancellation design rests on visa posting an io completion for a *terminated*
/// job: that event is what tells us the driver has stopped writing, and so what releases
/// the buffer. If visa stayed silent, every cancelled transfer would leak its buffer.
#[test]
fn terminate_posts_io_completion() -> Result<()> {
    use visa_rs::enums::attribute::{AttrTmoValue, HasAttribute};
    init_logger();
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };
    let (port, _server) = start_tcp_silent()?;
    let instr = rm.open(
        &CString::new(format!("TCPIP0::127.0.0.1::{}::SOCKET", port))?.into(),
        AccessMode::NO_LOCK,
        Duration::from_secs(3),
    )?;
    // Long enough that the read cannot finish by timing out during the test, so any
    // completion we see is attributable to `viTerminate`.
    instr.set_attr(AttrTmoValue::new_checked(1_000).expect("1s is a valid timeout"))?;

    let kind = event::EventKind::EventIoCompletion;
    let handler = instr.install_handler(kind, |_: &Instrument, e: &Event| e.kind())?;
    instr.enable_event(kind, event::Mechanism::Handler)?;

    // Leaked on purpose: visa keeps writing into it until the completion arrives, and
    // this test must not free it early even if an assertion below fails.
    let buf: &'static mut [u8] = Box::leak(vec![0u8; 64].into_boxed_slice());
    let (job, completion) = unsafe { instr.visa_read_async(buf)? };
    if completion == visa_rs::enums::status::CompletionCode::SuccessSync {
        // Some visa implementations run `viReadAsync` synchronously -- it blocks for the
        // session timeout and returns with the transfer already finished. There is then
        // no outstanding job to terminate, so this property cannot be observed here.
        log::warn!("viReadAsync completed synchronously; skipping (nothing to terminate)");
        return Ok(());
    }
    assert!(
        handler.receiver().try_recv().is_err(),
        "the read completed on its own, so this proves nothing about terminate"
    );

    instr.terminate(job)?;
    let got = handler
        .receiver()
        .recv_timeout(Duration::from_secs(5))
        .expect("viTerminate must post an io completion event");
    assert_eq!(got, kind);
    Ok(())
}

#[test]
fn list_instr() -> Result<()> {
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };
    let mut list = rm.find_res_list(&CString::new("?*INSTR")?.into())?;
    while let Some(n) = list.find_next()? {
        eprintln!("{}", n);
    }
    Ok(())
}

#[test]
fn send_idn() -> Result<()> {
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };
    let mut list = match find_or_skip(&rm, "?*KEYSIGH?*INSTR")? {
        Some(list) => list,
        None => return Ok(()),
    };
    if let Some(n) = list.find_next()? {
        let mut instr = rm.open(&n, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
        instr.write_all(b"*IDN?\n")?;
        let mut buf_reader = BufReader::new(instr);
        let mut buf = String::new();
        buf_reader.read_line(&mut buf)?;
        eprintln!("{}", buf);
    }
    Ok(())
}

#[test]
fn handler() -> Result<()> {
    // tried EventKind::Trig, but not supported by my keysight osc :(
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };
    let mut list = match find_or_skip(&rm, "?*KEYSIGH?*INSTR")? {
        Some(list) => list,
        None => return Ok(()),
    };
    if let Some(n) = list.find_next()? {
        let instr = rm.open(&n, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
        let call_back1 = |ins: &Instrument, t: &Event| -> () {
            log::info!("call1: {:?} {:?}", ins, t);
        };
        let call_back2 = |ins: &Instrument, t: &Event| -> () {
            log::info!("call2: {:?} {:?}", ins, t);
        };
        let event = event::EventKind::EventIoCompletion;
        let h1 = instr.install_handler(event, call_back1)?;
        let h2 = instr.install_handler(event, call_back2)?;
        instr.enable_event(event, event::Mechanism::Handler)?;
        unsafe { instr.visa_write_async(b"*IDN?\n")? };
        h1.receiver().recv()?;
        h2.receiver().recv()?;
        h1.uninstall();
        let mut v = vec![0u8; 256];
        unsafe { instr.visa_read_async(&mut v)? };
        log::info!("{}", String::from_utf8_lossy(v.as_ref()));
        h2.receiver().recv()?;
        log::info!("{}", String::from_utf8_lossy(v.as_ref()))
    }
    Ok(())
}

#[test]
fn async_io() -> Result<()> {
    init_logger();
    log::info!("start async_io test");
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };
    let mut list = match find_or_skip(&rm, "?*KEYSIGH?*INSTR")? {
        Some(list) => list,
        None => return Ok(()),
    };
    if let Some(n) = list.find_next()? {
        log::debug!("connecting to {}", n);
        let instr = rm.open(&n, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
        log::debug!("connected");
        let task = async move {
            let instr = instr.into_async()?;
            instr.async_write(b"*IDN?\n").await?;
            let resp = instr.async_read(Duration::from_secs(3)).await?;
            log::info!("get response: {}", String::from_utf8_lossy(&resp));
            Result::<()>::Ok(())
        };
        use tokio::runtime::Builder;
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        runtime.block_on(task)?;
    }
    log::info!("end async_io test");
    Ok(())
}

#[test]
fn async_io_virtual() -> Result<()> {
    init_logger();
    log::info!("start async_io_virtual test");
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };

    let (port, server) = start_tcp_virtual_resource_idn()?;
    let resources = [
        format!("TCPIP::127.0.0.1::{}::SOCKET", port),
        format!("TCPIP0::127.0.0.1::{}::SOCKET", port),
    ];
    let mut last_err = None;
    let mut instr_opt = None;
    for resource in resources {
        match rm.open(
            &CString::new(resource)?.into(),
            AccessMode::NO_LOCK,
            Duration::from_secs(3),
        ) {
            Ok(instr) => {
                instr_opt = Some(instr);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let instr = instr_opt.ok_or_else(|| anyhow!("open TCPIP SOCKET failed: {:?}", last_err))?;

    let task = async move {
        let instr = instr.into_async()?;
        instr.async_write(b"*IDN?\n").await?;
        let resp = instr.async_read(Duration::from_secs(3)).await?;
        let resp = String::from_utf8_lossy(&resp);
        assert!(resp.contains("TEST_INSTRUMENT"));
        Result::<()>::Ok(())
    };
    use tokio::runtime::Builder;
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(task)?;

    server.join().expect("server thread panicked")?;
    log::info!("end async_io_virtual test");
    Ok(())
}

#[cfg(feature = "tokio")]
#[test]
fn tokio_async_rw_virtual() -> Result<()> {
    init_logger();
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };

    log::info!("start tokio_async_rw_virtual test");
    let (port, server) = start_tcp_virtual_resource_idn()?;
    let resources = [
        format!("TCPIP::127.0.0.1::{}::SOCKET", port),
        format!("TCPIP0::127.0.0.1::{}::SOCKET", port),
    ];
    let mut last_err = None;
    let mut instr_opt = None;
    for resource in resources {
        match rm.open(
            &CString::new(resource)?.into(),
            AccessMode::NO_LOCK,
            Duration::from_secs(3),
        ) {
            Ok(instr) => {
                instr_opt = Some(instr);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let instr = instr_opt.ok_or_else(|| anyhow!("open TCPIP SOCKET failed: {:?}", last_err))?;

    let task = async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let async_instr = instr.into_async().map_err(std::io::Error::other)?;
        let mut adapter = visa_rs::InstrumentTokioAdapter::new(async_instr);
        adapter.write_all(b"*IDN?\n").await?;
        let mut buf = [0u8; 64];
        let n = adapter.read(&mut buf).await?;
        let resp = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(resp.trim_end(), "TEST_INSTRUMENT");
        Ok::<(), std::io::Error>(())
    };

    use tokio::runtime::Builder;
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(task)?;

    server.join().expect("server thread panicked")?;
    log::info!("end tokio_async_rw_virtual test");
    Ok(())
}

#[test]
fn tcpip_socket_idn() -> Result<()> {
    init_logger();
    let rm = match try_default_rm()? {
        Some(rm) => rm,
        None => return Ok(()),
    };

    let (port, server) = start_tcp_virtual_resource_idn()?;

    let resources = [
        format!("TCPIP::127.0.0.1::{}::SOCKET", port),
        format!("TCPIP0::127.0.0.1::{}::SOCKET", port),
    ];
    let mut last_err = None;
    let mut instr_opt = None;
    for resource in resources {
        match rm.open(
            &CString::new(resource)?.into(),
            AccessMode::NO_LOCK,
            Duration::from_secs(3),
        ) {
            Ok(instr) => {
                instr_opt = Some(instr);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let mut instr = instr_opt.ok_or_else(|| anyhow!("open TCPIP SOCKET failed: {:?}", last_err))?;
    // Without this `viRead` waits for the full count and the peer closes first, which
    // surfaces as ErrorIo rather than the response.
    {
        use visa_rs::enums::attribute::{AttrTermcharEn, HasAttribute};
        instr.set_attr(AttrTermcharEn::VI_TRUE)?;
    }

    instr
        .write_all(b"*IDN?\n")
        .map_err(|e| anyhow!("write failed: {}", e))?;
    let mut buf = [0u8; 64];
    let mut total = Vec::new();
    loop {
        let n = instr
            .read(&mut buf)
            .map_err(|e| anyhow!("read failed: {}", e))?;
        if n == 0 {
            break;
        }
        total.extend_from_slice(&buf[..n]);
        if total.contains(&b'\n') {
            break;
        }
    }
    let resp = String::from_utf8_lossy(&total);
    assert_eq!(resp.trim_end(), "TEST_INSTRUMENT");

    server.join().expect("server thread panicked")?;
    Ok(())
}
