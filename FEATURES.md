# visa-rs Feature Guide: Type Representation

This document explains the different ways to configure VISA type-to-Rust repr mapping in the `visa-rs` crate to support various build scenarios.

## Overview

The `visa-rs` crate provides three different modes for determining how VISA types are represented in Rust:

1. **Default** (no features): Uses `size_of` at proc macro compile time (host platform)
2. **`cross-compile` feature**: Uses predefined platform configurations from TOML
3. **`custom-repr` feature**: Allows custom mapping via environment variables

**Important:** When both `cross-compile` and `custom-repr` features are enabled simultaneously, the `custom-repr` behavior takes precedence. This ensures consistent and predictable behavior.

## Default Behavior (No Features)

**When to use:** Native compilation on the same platform where you'll run the code.

```toml
[dependencies]
visa-rs = "0.6"
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
visa-rs = { version = "0.6", features = ["cross-compile"] }
```

This enables config-based type mapping using `repr_config.toml`. Platform-specific repr attributes are generated based on the target platform, not the host.

**Configuration:** Edit `visa-rs-proc/repr_config.toml` to define type mappings:

```toml
[invariant]
ViUInt16 = "u16"  # Platform-invariant types

[platform_dependent.unsigned]
types = ["ViUInt32", "ViEvent", ...]

[[platform_dependent.unsigned.mappings]]
condition = 'target_os = "windows"'
repr = "u32"

[[platform_dependent.unsigned.mappings]]
condition = 'all(not(target_os = "windows"), target_pointer_width = "64")'
repr = "u64"
```

**Features:**
- ✅ Supports cross-compilation
- ✅ Generates compile errors if platform configuration is missing
- ✅ Easy to extend for new platforms
- ⚙️ Requires maintaining config file

**Example - Cross-compile Linux to Windows:**
```bash
cargo build --features cross-compile --target x86_64-pc-windows-gnu
```

## Custom-Repr Feature

**When to use:** Advanced scenarios requiring runtime configuration or per-type customization.

```toml
[dependencies]
visa-rs = { version = "0.6", features = ["custom-repr"] }
```

This allows specifying repr mappings via environment variables at build time.

**Usage:**

Set environment variables with the format `VISA_REPR_<TYPENAME>`:

```bash
# Simple unconditional repr
export VISA_REPR_VISTATUS="i32"

# Platform-dependent repr with conditions
export VISA_REPR_VISTATUS='target_os = "windows":i32,target_os = "linux":i64'

# Build with custom mappings
cargo build --features custom-repr
```

**Format:**
- Simple: `VISA_REPR_TYPENAME="repr_type"`
- Conditional: `VISA_REPR_TYPENAME="condition1:type1,condition2:type2"`

**Features:**
- ✅ Maximum flexibility
- ✅ Can override types per build
- ✅ Falls back to default behavior if env var not set
- ⚙️ Requires setting environment variables

**Example - Custom cross-compilation:**
```bash
export VISA_REPR_VISTATUS='target_os = "windows":i32,not(target_os = "windows"):i64'
export VISA_REPR_VIUINT32='target_os = "windows":u32,not(target_os = "windows"):u64'
cargo build --features custom-repr --target x86_64-pc-windows-gnu
```

## Feature Comparison

| Feature | Cross-Compile Support | Configuration Method | Compile Error on Missing Config | Best For |
|---------|----------------------|---------------------|--------------------------------|----------|
| Default | ❌ | None (automatic) | N/A | Native builds |
| `cross-compile` | ✅ | TOML file | ✅ Yes | Standard cross-compilation |
| `custom-repr` | ✅ | Environment variables | ❌ Fallback to default | Custom/advanced scenarios |
| Both features | ✅ | Environment variables (custom-repr precedence) | ❌ Fallback to default | Maximum flexibility |

**Note:** When both `cross-compile` and `custom-repr` are enabled, `custom-repr` takes precedence. This ensures that environment variables can override the TOML configuration if needed.

## Recommendations

- **Native development:** Use default (no features)
- **Cross-compilation in CI/CD:** Use `cross-compile` feature with `repr_config.toml`
- **Custom build systems:** Use `custom-repr` feature with environment variables
- **Maximum flexibility:** Enable both features - use env vars when needed, fall back to TOML config
- **Library authors:** Consider supporting `cross-compile` feature for users

## Feature Interaction

### When Both Features Are Enabled

When you enable both `cross-compile` and `custom-repr` features:

```toml
[dependencies]
visa-rs = { version = "0.6", features = ["cross-compile", "custom-repr"] }
```

The behavior is **identical** to enabling only `custom-repr`:
- If environment variable `VISA_REPR_<TYPENAME>` is set, it uses that value
- If environment variable is NOT set, it falls back to the default `size_of` behavior
- The TOML configuration is **not used** in this mode

This design allows you to:
1. Have TOML config as a safety net for cross-compilation
2. Override specific types via environment variables when needed
3. Get predictable behavior regardless of feature combination

## Troubleshooting

### Error: "Type 'X' not found in repr_config.toml"

This error occurs when using the `cross-compile` feature and a type isn't defined in the config.

**Solution:** Add the type to `visa-rs-proc/repr_config.toml`:
```toml
[invariant]
YourType = "u32"  # or add to platform_dependent section
```

### Cross-compilation still fails

Make sure you're using the correct feature:
```bash
# Correct
cargo build --features cross-compile --target x86_64-pc-windows-gnu

# Wrong (will fail)
cargo build --target x86_64-pc-windows-gnu
```

### Custom repr not applied

Verify the environment variable:
```bash
# Check it's set
echo $VISA_REPR_VISTATUS

# Verify the format
export VISA_REPR_VISTATUS='target_os = "windows":i32'  # Must use quotes
```

## Migration Guide

### From Default to Cross-Compile

```diff
  [dependencies]
- visa-rs = "0.6"
+ visa-rs = { version = "0.6", features = ["cross-compile"] }
```

No code changes needed - the configuration is already provided in `repr_config.toml`.

### Adding New Platform Support

Edit `visa-rs-proc/repr_config.toml`:

```toml
[[platform_dependent.signed.mappings]]
condition = 'target_os = "your-platform"'
repr = "i32"  # or appropriate type
```

Then rebuild:
```bash
cargo clean
cargo build --features cross-compile --target your-platform
```
