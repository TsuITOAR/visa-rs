# visa-rs

[![crates.io](https://img.shields.io/crates/v/visa-rs.svg)](https://crates.io/crates/visa-rs)
[![docs](https://docs.rs/visa-rs/badge.svg)](https://docs.rs/visa-rs)

Safe Rust bindings for VISA(Virtual Instrument Software Architecture) library

Most documentation comes from [NI-VISA Product Documentation](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/help_file_title.html)

## Requirements

This crate needs to link to an installed visa library, for example, [NI-VISA](https://www.ni.com/en-us/support/downloads/drivers/download.ni-visa.html).

A default link configuration is used for the default installation setup on Windows, Linux and MacOs.  

You can overwrite the configuration by specifying the name of the visa library file (`visa` for linux, `visa64` or `visa32` for windows) by environment variable `LIB_VISA_NAME`, and the path of the file by environment variable `LIB_VISA_PATH`.

## Example

Add dependencies below to `Cargo.toml`

```toml
[dependencies]
visa_rs = { git = "https://github.com/TsuITOAR/visa-rs.git" }
```

Codes below will find the first Keysight instrument in your environment and print out its `*IDN?` response.

```rust
fn find_an_instr() -> visa_rs::Result<()>{
  use std::ffi::CString;
  use std::io::{BufRead, BufReader, Read, Write};
  use visa_rs::prelude::*;

  // open default resource manager
  let rm: DefaultRM = DefaultRM::new()?;

  // expression to match resource name
  let expr = CString::new("?*KEYSIGH?*INSTR").unwrap().into();

  // find the first resource matched
  let rsc = rm.find_res(&expr)?;

  // open a session to the resource, the session will be closed when rm is dropped
  let instr: Instrument = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;

  // write message
  (&instr).write_all(b"*IDN?\n").map_err(io_to_vs_err)?;

  // read response
  let mut buf_reader = BufReader::new(&instr);
  let mut buf = String::new();
  buf_reader.read_line(&mut buf).map_err(io_to_vs_err)?;

  eprintln!("{}", buf);
  Ok(())
}
```

License: MIT OR Apache-2.0
