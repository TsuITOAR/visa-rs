# Custom Repr Config Example

This example demonstrates how to customize VISA type repr mappings using environment variables.

## How It Works

The proc macro resolves repr mappings in this order:

1. `VISA_REPR_<TYPENAME>` environment variables (per type)
2. `VISA_REPR_CONFIG_PATH` (explicit path; must be absolute)

Behavior by feature:

- With `cross-compile` enabled, the file is optional. If present, it overrides the bundled default.
- With `custom-repr` enabled, you must provide **either** environment variables for all VISA types used **or** a config file via `VISA_REPR_CONFIG_PATH`.

## Environment Variable Mapping (Recommended)

The `.cargo/config.toml` in this example sets per-type environment variables so the
proc macro does not need to load any config file:

```toml
[env]
VISA_REPR_VIUINT16 = "u16"
VISA_REPR_VIINT16 = "i16"
VISA_REPR_VIUINT32 = "u64"
VISA_REPR_VIEVENT = "u64"
VISA_REPR_VIEVENTTYPE = "u64"
VISA_REPR_VIEVENTFILTER = "u64"
VISA_REPR_VIATTR = "u64"
VISA_REPR_VISTATUS = "i64"
VISA_REPR_VIINT32 = "i64"
```

## Configuration File (Optional)

If you prefer a file, set `VISA_REPR_CONFIG_PATH` to an **absolute** path. This example
file forces all types to use 32-bit representations on all platforms:

```toml
[[platforms]]
condition = 'all()' # Matches all platforms
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
```

This overrides the default platform-specific mappings (where Unix 64-bit would use `u64`/`i64`).

## Building

```bash
# From the example directory
cargo build

# Or from the repository root
cargo build --manifest-path examples/custom_repr/Cargo.toml
```
