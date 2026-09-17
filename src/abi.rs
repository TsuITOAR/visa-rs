//! Validate the repr macro's fixed-width assumptions on the build target, not
//! the host running the proc macro. These checks also run outside test builds.

macro_rules! assert_visa_repr {
    ($($visa:ident => $rust:ty),* $(,)?) => {
        $(
            const _: () = {
                assert!(
                    std::mem::size_of::<visa_sys::$visa>() == std::mem::size_of::<$rust>(),
                    concat!(stringify!($visa), " has an unexpected size"),
                );
                assert!(
                    std::mem::align_of::<visa_sys::$visa>() == std::mem::align_of::<$rust>(),
                    concat!(stringify!($visa), " has an unexpected alignment"),
                );
                assert!(
                    visa_sys::$visa::MIN as i128 == <$rust>::MIN as i128
                        && visa_sys::$visa::MAX as i128 == <$rust>::MAX as i128,
                    concat!(stringify!($visa), " has an unexpected integer range"),
                );
            };
        )*
    };
}

assert_visa_repr! {
    ViUInt8 => u8,
    ViInt8 => i8,
    ViUInt16 => u16,
    ViInt16 => i16,
    ViUInt32 => u32,
    ViInt32 => i32,
    ViUInt64 => u64,
    ViInt64 => i64,
    ViEvent => u32,
    ViEventType => u32,
    ViEventFilter => u32,
    ViAttr => u32,
    ViStatus => i32,
}
