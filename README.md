# visa-rs

[![crates.io](https://img.shields.io/crates/v/visa-rs.svg)](https://crates.io/crates/visa-rs)
[![docs](https://docs.rs/visa-rs/badge.svg)](https://docs.rs/visa-rs)

Safe Rust bindings for VISA(Virtual Instrument Software Architecture) library

Most documentation comes from [NI-VISA Product Documentation](https://www.ni.com/docs/en-US/bundle/ni-visa-20.0/page/ni-visa/help_file_title.html)

## Requirements

This crate needs to link to an installed visa library, for example, [NI-VISA](https://www.ni.com/en-us/support/downloads/drivers/download.ni-visa.html).

A default link configuration is used for the default installation setup on Windows, Linux and MacOs.

You can overwrite the configuration by specifying the name of the visa library file (default to `visa` for linux, `visa64` or `visa32` for windows) by environment variable `LIB_VISA_NAME`, and the path of the file by environment variable `LIB_VISA_PATH`.

## Usage

Add the dependency below to `Cargo.toml`

```toml
[dependencies]
visa-rs = "0.7"
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

## Async IO

`Instrument::into_async` returns an `AsyncInstrument`, backed by VISA's
`viReadAsync`/`viWriteAsync` and the IO-completion event:

```rust
let instr = instr.into_async()?;
instr.async_write(b"*IDN?\n").await?;

// reads one complete response -- no length to guess, and the timeout bounds each chunk
let resp: Vec<u8> = instr.async_read(Duration::from_secs(3)).await?;

// for binary block data, whose bytes may include the termination character
let block: Vec<u8> = instr.async_read_exact(4096, Duration::from_secs(3)).await?;
```

A VISA transfer keeps using its buffer until the completion event arrives, so buffers are
owned rather than borrowed: `async_write` takes anything `Into<Vec<u8>>` and `async_read`
returns a `Vec<u8>`. Dropping a future terminates the transfer, but its buffer is held by
the session until VISA confirms it has stopped writing -- cancelling costs a buffer, never
memory safety.

With the default `tokio` feature, `Instrument::into_tokio_async` additionally yields an
`InstrumentTokioAdapter` implementing `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`,
so a session works with `AsyncReadExt`/`AsyncWriteExt`, `BufReader`, and the rest of the
Tokio ecosystem.

If you do not want the Tokio dependency, opt out of default features:

```toml
[dependencies]
visa-rs = { version = "0.7", default-features = false }
```

### Known limitation: async does not work on macOS

NI-VISA for macOS never defers an asynchronous transfer. `viReadAsync`/`viWriteAsync` run
it inline, invoke your handler on the *calling* thread, and return `VI_SUCCESS_SYNC` after
blocking for the full session timeout. So on macOS, awaiting a read blocks the calling
thread — wrap it in `tokio::task::spawn_blocking`. Other platforms are unaffected, and the
same code is properly asynchronous wherever VISA does defer.

Run `cargo run --example async_probe` to check your own platform and instruments.

## Features

| Feature | Default | What it does |
| --------------- | ------- | ------------ |
| `tokio` | ✅ | Enables `InstrumentTokioAdapter`, which implements `tokio::io::AsyncRead` and `tokio::io::AsyncWrite` for a VISA session. |
| `cross-compile` | ❌ | Chooses enum `repr`s from a per-target table instead of the host's type sizes, so the crate can be cross-compiled. |
| `custom-repr` | ❌ | Takes enum `repr`s from `VISA_REPR_*` environment variables or a user-supplied config file. Implies `cross-compile`. |

Some enum `repr`s depend on the target architecture, which is why cross-compilation
needs an explicit feature. Note that `custom-repr` **deliberately fails to compile**
until you supply a repr mapping, so do not build this crate with `--all-features`.

See [FEATURES.md](FEATURES.md) for the full guide.

## Feedback

If you run into issues, please share your runtime feedback and device/driver
environment to help improve the next release.

License: MIT OR Apache-2.0
