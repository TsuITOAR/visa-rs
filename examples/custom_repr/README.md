# Custom Repr Config Example

This example demonstrates how to customize VISA type repr mappings using a local configuration file.

## How It Works

When building with the `cross-compile` feature, the proc macro looks for a file named `visa_repr_config.toml` in the current crate's root directory (same directory as `Cargo.toml`). If found, it uses that configuration instead of the default bundled configuration.

## Configuration File

The `visa_repr_config.toml` in this example forces all types to use 32-bit representations:

```toml
[invariant]
ViUInt16 = "u16"
ViInt16 = "i16"

[platform_dependent.unsigned]
types = ["ViUInt32", "ViEvent", "ViEventType", "ViEventFilter", "ViAttr"]

[[platform_dependent.unsigned.mappings]]
condition = 'any()'  # Matches all platforms
repr = "u32"

[platform_dependent.signed]
types = ["ViStatus", "ViInt32"]

[[platform_dependent.signed.mappings]]
condition = 'any()'  # Matches all platforms
repr = "i32"
```

This overrides the default platform-specific mappings (where Unix 64-bit would use `u64`/`i64`).

## Building

```bash
# From the example directory
cargo build

# Or from the repository root
cargo build --manifest-path examples/custom_repr/Cargo.toml
```

## Use Cases

Custom repr configurations are useful when:

1. **Testing**: Force specific type sizes to test edge cases
2. **Custom platforms**: Define repr types for platforms not in the default config
3. **Special requirements**: Your application has specific type size requirements
4. **Migration**: Gradually migrate between different repr schemes

## Note

The build will fail at the linking stage if VISA libraries are not installed, but the important part is that it compiles successfully, proving the custom configuration is being used correctly by the proc macro.
