# visa-rs Feature Guide: Type Representation

This document explains the different ways to configure VISA type-to-Rust repr mapping in the `visa-rs` crate to support various build scenarios.

## Overview

The `visa-rs` crate provides three different modes for determining how VISA types are represented in Rust:

1. **Default** (no features): Uses `size_of` at proc macro compile time (host platform)
2. **`cross-compile` feature**: Uses predefined platform configurations from TOML
3. **`custom-repr` feature**: Allows custom mapping via environment variables

**Note:** When both `cross-compile` and `custom-repr` features are enabled simultaneously, the `custom-repr` behavior takes precedence. This ensures consistent and predictable behavior.

## Default Behavior (No Features)

**When to use:** Native compilation on the same platform where you'll run the code.

```toml
[dependencies]
visa-rs = "0.7"
```

This uses the original `size_of`-based approach where type sizes are evaluated at proc macro compile time. Works fine for native builds but **will fail for cross-compilation** because the proc macro runs on the host platform.

**Limitations:**

- ❌ Does not support cross-compilation (e.g., Linux → Windows)
- ✅ Simple and straightforward for native builds
- ✅ No configuration needed

## Cross-Compile Feature

**When to use:** Cross-compilation between platforms with different C type sizes.

```toml
[dependencies]
visa-rs = { version = "0.7", features = ["cross-compile"] }
```

This enables config-based type mapping using a bundled default config. Platform-specific repr attributes are generated based on the target platform, not the host.

**Configuration:** The proc macro uses a bundled default config (from `visa-rs-proc/default_repr_config.toml`) by default, but you can override it by setting `VISA_REPR_CONFIG_PATH` to an absolute config file path.

Default config structure:

```toml
[[platforms]]
condition = 'all(target_os = "windows", target_pointer_width = "64")'
[platforms.types]
ViUInt16 = "u16"
ViInt16 = "i16"
ViUInt32 = "u32"
ViEvent = "u32"
ViEventType = "u32"
ViEventFilter = "u32"
ViAttr = "u32"
ViStatus = "i32"
ViInt32 = "i32"

[[platforms]]
condition = 'all(target_os = "linux", target_pointer_width = "64")'
[platforms.types]
ViUInt16 = "u16"
ViInt16 = "i16"
ViUInt32 = "u64"
ViEvent = "u64"
ViEventType = "u64"
ViEventFilter = "u64"
ViAttr = "u64"
ViStatus = "i64"
ViInt32 = "i64"
```

**Features:**

- ✅ Supports cross-compilation
- ✅ Generates compile errors if platform configuration is missing
- ✅ Easy to extend for new platforms
- ✅ **Can be customized per-project** with local `visa_repr_config.toml` or `VISA_REPR_CONFIG_PATH`
- ⚙️ Requires maintaining config file

**Example - Cross-compile Linux to Windows:**

```bash
cargo build --features cross-compile --target x86_64-pc-windows-gnu
```

## Custom-Repr Feature

```toml
[dependencies]
visa-rs = { version = "0.7", features = ["custom-repr"] }
```

This allows specifying repr mappings via environment variables or a config file at build time.

**Usage:**

Set environment variables with the format `VISA_REPR_<TYPENAME>`:

```bash
export VISA_REPR_VISTATUS="i32"

export VISA_REPR_VISTATUS='target_os = "windows":i32,target_os = "linux":i64'

cargo build --features custom-repr
```

**Format:**

- Simple: `VISA_REPR_TYPENAME="repr_type"`
- Conditional: `VISA_REPR_TYPENAME="condition1:type1,condition2:type2"`

You can also set `VISA_REPR_CONFIG_PATH` to an **absolute** path or place
`visa_repr_config.toml` in the crate root. The config file is used only when a
type does not have a corresponding `VISA_REPR_<TYPENAME>` environment variable.

**Features:**

- ✅ Maximum flexibility
- ✅ Can override types per build
- ✅ **Triggers compile error** if a type mapping is missing (via env or config)
- ⚙️ Requires mapping for all VISA types used (env vars or config file)

**Example - Custom cross-compilation (via .cargo/config.toml):**

```toml
# .cargo/config.toml
[env]
VISA_REPR_VISTATUS = 'target_os = "windows":i32,not(target_os = "windows"):i64'
VISA_REPR_VIUINT32 = 'target_os = "windows":u32,not(target_os = "windows"):u64'
VISA_REPR_VIINT32 = 'target_os = "windows":i32,not(target_os = "windows"):i64'
VISA_REPR_VIUINT16 = "u16"
VISA_REPR_VIINT16 = "i16"
VISA_REPR_VIEVENT = 'target_os = "windows":u32,not(target_os = "windows"):u64'
VISA_REPR_VIEVENTTYPE = 'target_os = "windows":u32,not(target_os = "windows"):u64'
VISA_REPR_VIEVENTFILTER = 'target_os = "windows":u32,not(target_os = "windows"):u64'
VISA_REPR_VIATTR = 'target_os = "windows":u32,not(target_os = "windows"):u64'
```

```bash
cargo build --features custom-repr --target x86_64-pc-windows-gnu
```

## Feature Comparison

| Feature | Cross-Compile Support | Configuration Method | Compile Error on Missing Config | Best For |
| --------- | ---------------------- | --------------------- | -------------------------------- | ---------- |
| Default | ❌ | None (automatic) | N/A | Native builds |
| `cross-compile` | ✅ | TOML file | ✅ Yes | Standard cross-compilation |
| `custom-repr` | ✅ | Environment variables or config file | ✅ Yes | Custom/advanced scenarios with explicit control |
| Both features | ✅ | Environment variables or config file (custom-repr precedence) | ✅ Yes | Maximum flexibility |

### Tool: generate-repr-config

For easier configuration, you can use the `generate-repr-config` tool to automatically detect type sizes on the target platform:

```bash
cd generate-repr-config
cargo build --release --target x86_64-pc-windows-gnu
```

```bash
# On the target platform:
generate-repr-config --format cargo-config > .cargo/config.toml
```

```bash
# Back in the visa-rs repo:
cargo build --features custom-repr --target x86_64-pc-windows-gnu
```

See `generate-repr-config/README.md` for detailed usage instructions.

### Adding New Platform Support

Edit `visa-rs-proc/default_repr_config.toml`:

```toml
[[platforms]]
condition = 'all(target_os = "your-platform", target_pointer_width = "64")'
[platforms.types]
ViStatus = "i32"
```

Then rebuild:

```bash
cargo clean
cargo build --features cross-compile --target your-platform
```
