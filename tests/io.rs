use std::{
    ffi::CString,
    io::{BufRead, BufReader, Write},
};

use visa_rs::{flags::AccessMode, *};

#[test]
fn list_instr() -> Result<()> {
    let rm = DefaultRM::new()?;
    let mut list = rm.find_res(&CString::new("?*INSTR").unwrap().into())?;
    while let Some(n) = list.find_next()? {
        eprintln!("{}", n);
    }
    Ok(())
}

#[test]
fn send_idn() -> Result<()> {
    let rm = DefaultRM::new()?;
    let mut list = rm.find_res(&CString::new("*KEYSIGH?*INSTR").unwrap().into())?;
    if let Some(n) = list.find_next()? {
        let mut instr = rm.open(&n, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;
        instr.write_all(b"*IDN?\n").unwrap();
        let mut buf_reader = BufReader::new(instr);
        let mut buf = String::new();
        buf_reader.read_line(&mut buf).unwrap();
        eprintln!("{}", buf);
    }
    Ok(())
}
