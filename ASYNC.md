# Async IO

`Instrument::into_async` returns an `AsyncInstrument` for async reads and writes:

```rust
let instr = instr.into_async()?;
instr.async_write(b"*IDN?\n").await?;

// Read one response; the timeout applies to each underlying read, not the whole response.
let resp: Vec<u8> = instr.async_read(Duration::from_secs(3)).await?;

// for binary block data, whose bytes may include the termination character
let block: Vec<u8> = instr.async_read_exact(4096, Duration::from_secs(3)).await?;
```

`async_write` accepts inputs convertible to `Vec<u8>` and returns the byte count after
completion. Reads return a `Vec<u8>`. Dropping these futures requests cancellation.

The default `tokio` feature adds `Instrument::into_tokio_async`, which supports Tokio's
`AsyncRead` and `AsyncWrite` traits. Writes start VISA transfers; `flush().await` waits
for completion and reports write errors. Flush before reading a response or
dropping/converting the adapter.

Dropping a Tokio operation's future does not cancel an already started transfer.
Dropping or converting the adapter requests cancellation of pending I/O.

If you do not want the Tokio dependency, opt out of default features:

```toml
[dependencies]
visa-rs = { version = "0.7", default-features = false }
```

## Warning: async may block on macOS

Tests with NI-VISA 23.5 indicate that async I/O on macOS may block until timeout.

Run `cargo run --example async_probe` to check your own platform and instruments.
