# visa-rs

[![crates.io](https://img.shields.io/crates/v/visa-rs.svg)](https://crates.io/crates/visa-rs)
[![docs](https://docs.rs/visa-rs/badge.svg)](https://docs.rs/visa-rs)


Safe Rust bindings for VISA(Virtual Instrument Software Architecture) library

Most documentation comes from [NI-VISA Product Documentation](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/help_file_title.html)

## Requirements
This crate needs to link to an installed visa library, for example, [NI-VISA](https://www.ni.com/en-us/support/downloads/drivers/download.ni-visa.html).

You can specify path of `visa64.lib` file (or `visa32.lib` on 32-bit systems) by setting environment variable `LIB_VISA_PATH`.

On Windows and macOS, the default installation path will be added if no path is specified.

## Example
Add dependencies below to `Cargo.toml`
```
[dependencies]
visa_rs = "^0.5"
anyhow = "^1"
```
Codes below will find the first Keysight instrument in your environment and print out its `*IDN` response.
```rust
fn find_a_instr() -> anyhow::Result<()>{
  use std::ffi::CString;
  use std::io::{BufRead, BufReader, Read, Write};
  use visa_rs::{flags::AccessMode, AsResourceManager, DefaultRM, TIMEOUT_IMMEDIATE};
  let rm = DefaultRM::new()?; //open default resource manager
  let expr = CString::new("?*KEYSIGH?*INSTR")?.into(); //expr used to match resource name
  let rsc = rm.find_res(&expr)?; // find the first resource matched
  let mut instr = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?; //open a session to resource
  instr.write_all(b"*IDN?\n")?; //write message
  let mut buf_reader = BufReader::new(&instr);
  let mut buf = String::new();
  buf_reader.read_line(&mut buf)?; //read response
  eprintln!("{}", buf);
  Ok(())
}
```

License: MIT OR Apache-2.0
