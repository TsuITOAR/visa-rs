# generate-repr-config

A utility tool to detect VISA type sizes on the target platform and generate configuration for the `custom-repr` feature of `visa-rs`.

## Purpose

When cross-compiling `visa-rs`, you may need to use the `custom-repr` feature with environment variables. This tool helps by:

1. Running on the target platform to detect actual type sizes
2. Generating configuration in various formats (shell script, batch file, Cargo config, TOML, JSON)
3. Providing the exact repr types needed for that platform

## Usage

### Basic Usage

```bash
# Generate shell script (default)
generate-repr-config > set_vars.sh

# Generate Windows batch file
generate-repr-config --format batch > set_vars.bat

# Generate TOML config
generate-repr-config --format toml > detected_config.toml

# Generate .cargo/config.toml
generate-repr-config --format cargo-config > .cargo/config.toml

# Generate JSON
generate-repr-config --format json
```

### Cross-Compilation Workflow

1. **Build the tool for your target platform:**

   ```bash
   cd generate-repr-config
   cargo build --release --target x86_64-pc-windows-gnu
   ```

2. **Copy the binary to the target platform:**

   ```bash
   # The binary will be at:
   # target/x86_64-pc-windows-gnu/release/generate-repr-config.exe
   ```

3. **Run on the target platform:**

   ```bash
   # On Windows:
   generate-repr-config.exe --format batch > set_repr_vars.bat
   
   # Or for shell (Git Bash, WSL, etc.):
   generate-repr-config.exe --format shell > set_repr_vars.sh
   ```

4. **Use the generated configuration:**

   ```bash
   # On Windows (cmd.exe):
   set_repr_vars.bat
   
   # On Unix/Linux/macOS or Git Bash:
   source set_repr_vars.sh
   ```

   Or drop it into `.cargo/config.toml`:

   ```toml
   [env]
   VISA_REPR_VISTATUS = "i32"
   # ... etc
   ```

5. **Build visa-rs with custom-repr:**

   ```bash
   cd ../..  # Back to visa-rs root
   cargo build --features custom-repr --target x86_64-pc-windows-gnu
   ```

## Output Formats

### Shell Script (`--format shell`)

Generates a POSIX shell script with `export` commands. Can be sourced to set environment variables.

```bash
export VISA_REPR_VISTATUS="i32"
export VISA_REPR_VIUINT32="u32"
# ... etc
```

### Batch File (`--format batch`)

Generates a Windows batch file with `set` commands.

```batch
set VISA_REPR_VISTATUS=i32
set VISA_REPR_VIUINT32=u32
REM ... etc
```

### TOML (`--format toml`)

Generates a TOML configuration snippet that shows the detected types. You can paste these
under a `[[platforms]]` entry in `visa_repr_config.toml`.

```toml
[[platforms]]
condition = 'all()'
[platforms.types]
ViStatus = "i32"
ViUInt32 = "u32"
# ... etc
```

### Cargo Config (`--format cargo-config`)

Generates a `.cargo/config.toml` snippet with `[env]` entries.

```toml
[env]
VISA_REPR_VISTATUS = "i32"
VISA_REPR_VIUINT32 = "u32"
# ... etc
```

### JSON (`--format json`)

Generates JSON output for programmatic use.

```json
{
  "ViStatus": "i32",
  "ViUInt32": "u32"
}
```

## Detected Types

The tool detects repr types for the following VISA types:

- `ViUInt16` - Usually `u16` (platform-independent)
- `ViInt16` - Usually `i16` (platform-independent)
- `ViUInt32` - `u32` on Windows, `u64` on 64-bit Unix
- `ViInt32` - `i32` on Windows, `i64` on 64-bit Unix
- `ViStatus` - `i32` on Windows, `i64` on 64-bit Unix
- `ViEvent` - `u32` on Windows, `u64` on 64-bit Unix
- `ViEventType` - `u32` on Windows, `u64` on 64-bit Unix
- `ViEventFilter` - `u32` on Windows, `u64` on 64-bit Unix
- `ViAttr` - `u32` on Windows, `u64` on 64-bit Unix

## Example: Linux to Windows Cross-Compilation

```bash
# 1. Build the detection tool for Windows
cd generate-repr-config
cargo build --release --target x86_64-pc-windows-gnu

# 2. Copy to Windows machine and run
# (On Windows machine:)
generate-repr-config.exe --format batch > set_repr_vars.bat
set_repr_vars.bat

# 3. Build visa-rs (back on Linux, with env vars set)
cd ..
cargo build --features custom-repr --target x86_64-pc-windows-gnu
```

## Why This Tool Exists

The `visa-rs` proc macro runs on the build machine (host), not the target machine. When cross-compiling, the host and target may have different type sizes for C types like `c_long` and `c_ulong`:

- On Linux x86_64: `c_long` = 64-bit
- On Windows x86_64: `c_long` = 32-bit

This tool solves the problem by running on the target machine to detect the actual sizes, ensuring correct configuration for cross-compilation.

## License

Same as `visa-rs` - MIT OR Apache-2.0
