# visa_repr_config.toml

This configuration file defines how VISA type names should be mapped to Rust repr attributes for cross-platform compatibility.

## Structure

The configuration is a list of platform entries. Each platform entry must explicitly list **all** VISA types and their reprs for that platform. This keeps the mapping fully explicit and avoids implicit invariants.

### Platforms (`[[platforms]]`)

Each platform entry has:

- `condition`: a Rust `cfg` condition
- `types`: a map from VISA type name to Rust repr type

## Adding New Type Mappings

To add a new VISA type mapping, add it to the `types` map of **every** `[[platforms]]` entry.

## Custom Config Location

The proc macro resolves mappings in this order:

1. `VISA_REPR_<TYPENAME>` environment variables (per type)
2. `VISA_REPR_CONFIG_PATH` (explicit path; must be absolute)
3. `visa_repr_config.toml` in the crate root (same directory as `Cargo.toml`)

## Condition Syntax

Conditions use Rust's `cfg` syntax and support:

- `target_os = "windows"` - Target OS is Windows
- `target_pointer_width = "64"` - Target has 64-bit pointers
- `all(...)` - All conditions must be true
- `any(...)` - At least one condition must be true
- `not(...)` - Condition must be false

## Example Generated Code

For a type like `ViStatus` with the current configuration, the proc macro generates:

```rust
#[cfg_attr(all(target_os = "windows", target_pointer_width = "64"), repr(i32))]
#[cfg_attr(all(target_os = "linux", target_pointer_width = "64"), repr(i64))]
pub enum ViStatus { ... }
```

This ensures the correct representation is chosen at compile time based on the target platform.

## Example Configuration

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
