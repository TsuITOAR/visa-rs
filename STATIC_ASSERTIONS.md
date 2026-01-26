# Static Size Assertions

Starting from this PR, the `repr!` proc macro automatically generates static assertions to verify that the size of generated enums matches the size of the corresponding VISA-sys types.

## How It Works

When you use `#[repr(ViStatus)]` or any other VISA type, the proc macro:

1. Determines the appropriate Rust repr type (e.g., `i32`, `i64`, `u32`, `u64`)
2. Applies the `#[repr(...)]` attribute to the enum
3. Generates a const assertion that verifies the enum size matches the VISA-sys type size
4. Provides a helpful error message if sizes don't match

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
    struct SizeAssert<const L: usize, const R: usize>;
    
    impl<const L: usize, const R: usize> SizeAssert<L, R> {
        const VALID: () = assert!(
            L == R,
            "Size mismatch: ErrorCode enum does not have the same size as visa_sys::ViStatus. \
             This likely means the #[repr(...)] attribute is incorrect for the target platform. \
             If cross-compiling, ensure you're using the 'cross-compile' feature or have set \
             the correct environment variables with 'custom-repr' feature."
        );
    }
    
    let _ = SizeAssert::<
        {std::mem::size_of::<ErrorCode>()},
        {std::mem::size_of::<visa_sys::ViStatus>()}
    >::VALID;
    
    // Fallback for older Rust versions
    const fn _assert_size_eq() {
        let _ = std::mem::transmute::<ErrorCode, visa_sys::ViStatus>;
    }
};
```

## Benefits

- **Compile-time safety**: If the repr attribute is incorrect for the target platform, compilation will fail
- **Helpful error messages**: Clear explanation of what went wrong and how to fix it
- **Platform verification**: Ensures cross-compilation produces correct type sizes
- **No runtime overhead**: Assertions are evaluated at compile time

## Error Detection

If there's a size mismatch, you'll get a clear compile error like:

```
error[E0080]: evaluation of `<SizeAssert<4, 8>>::VALID` failed
  --> src/enums/status.rs:12:18
   |
12 |         pub enum ErrorCode{
   |                  ^^^^^^^^^
   |
   = note: the evaluated program panicked at 'Size mismatch: ErrorCode enum does not have 
           the same size as visa_sys::ViStatus. This likely means the #[repr(...)] 
           attribute is incorrect for the target platform. If cross-compiling, ensure 
           you're using the 'cross-compile' feature or have set the correct environment 
           variables with 'custom-repr' feature.', src/enums/status.rs:12:18
```

This ensures that any repr configuration errors are caught at compile time with a clear explanation, rather than causing undefined behavior at runtime.

## Fallback Mechanism

The assertion includes both:
1. **Primary**: A const assert with a helpful custom error message (modern Rust)
2. **Fallback**: A transmute check for older Rust versions

This ensures compatibility across different Rust versions while providing the best possible error messages when available.
