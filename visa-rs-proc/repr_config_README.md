# repr_config.toml

This configuration file defines how VISA type names should be mapped to Rust repr attributes for cross-platform compatibility.

## Structure

The configuration is divided into two main sections:

### 1. Platform-Invariant Types (`[invariant]`)

These types have the same representation on all platforms:

```toml
[invariant]
ViUInt16 = "u16"  # c_ushort, always u16
ViInt16 = "i16"   # c_short, always i16
```

### 2. Platform-Dependent Types (`[platform_dependent]`)

These types have different representations depending on the target platform, because they are based on C types like `c_long` and `c_ulong` which vary by platform.

#### Unsigned Types (`[platform_dependent.unsigned]`)

Types based on `c_ulong` (ViUInt32, ViEvent, ViEventType, ViEventFilter, ViAttr):

- Windows (all architectures): `u32`
- Unix 64-bit (Linux, macOS): `u64`
- Unix 32-bit: `u32`

#### Signed Types (`[platform_dependent.signed]`)

Types based on `c_long` (ViStatus, ViInt32):

- Windows (all architectures): `i32`
- Unix 64-bit (Linux, macOS): `i64`
- Unix 32-bit: `i32`

## Adding New Type Mappings

To add a new VISA type mapping:

1. **For platform-invariant types**: Add an entry to the `[invariant]` section:
   ```toml
   [invariant]
   YourType = "u32"
   ```

2. **For platform-dependent types**: Add the type name to the appropriate `types` array and ensure the mappings are correct:
   ```toml
   [platform_dependent.unsigned]
   types = ["ViUInt32", "YourNewType", ...]
   ```

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
#[cfg_attr(target_os = "windows", repr(i32))]
#[cfg_attr(all(not(target_os = "windows"), target_pointer_width = "64"), repr(i64))]
#[cfg_attr(all(not(target_os = "windows"), not(target_pointer_width = "64")), repr(i32))]
pub enum ViStatus { ... }
```

This ensures the correct representation is chosen at compile time based on the target platform.
