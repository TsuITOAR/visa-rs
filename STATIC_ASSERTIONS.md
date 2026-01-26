# Static Size Assertions

Starting from this PR, the `repr!` proc macro automatically generates static assertions to verify that the size of generated enums matches the size of the corresponding VISA-sys types.

## How It Works

When you use `#[repr(ViStatus)]` or any other VISA type, the proc macro:

1. Determines the appropriate Rust repr type (e.g., `i32`, `i64`, `u32`, `u64`)
2. Applies the `#[repr(...)]` attribute to the enum
3. Generates a const assertion that verifies the enum size matches the VISA-sys type size

## Example

Given this code:

```rust
#[repr(ViStatus)]
pub enum ErrorCode {
    VI_ERROR_SYSTEM_ERROR = 0xBFFF0000,
    // ... more variants
}
```

The proc macro generates (simplified):

```rust
#[repr(i32)]  // or i64 depending on platform
pub enum ErrorCode {
    VI_ERROR_SYSTEM_ERROR = 0xBFFF0000,
    // ... more variants
}

const _ASSERT_SIZE_EQ_ERRORCODE: () = {
    const fn _assert_size_eq() {
        let _ = ::std::mem::transmute::<ErrorCode, visa_sys::ViStatus>;
    }
};
```

## Benefits

- **Compile-time safety**: If the repr attribute is incorrect for the target platform, compilation will fail
- **Platform verification**: Ensures cross-compilation produces correct type sizes
- **No runtime overhead**: Assertions are evaluated at compile time

## Error Detection

If there's a size mismatch, you'll get a compile error like:

```
error[E0512]: cannot transmute between types of different sizes, or dependently-sized types
```

This ensures that any repr configuration errors are caught at compile time rather than causing undefined behavior at runtime.
