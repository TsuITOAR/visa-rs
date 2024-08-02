//! Defines [visa attributes](attribute), [visa status](status), [visa event](event), etc as Rust enums

macro_rules! consts_to_enum {
    {
        #[format=$fmt:ident]
        $(#[$metas:meta])*
        pub enum $enum_id:ident{
            $($status:ident $value:literal $($des:literal)?)*
        }
    } => {
        visa_rs_proc::rusty_ident!{
            visa_rs_proc::repr!{
                consts_to_enum!{
                    $fmt
                    $(#[$metas])*
                    pub enum $enum_id{
                        $($status $value $($des)?)*
                    }
                }
            }
        }

    };


    {   dbg
        $(#[$metas:meta])*
        pub enum $enum_id:ident{
            $($status:ident $value:literal $($des:literal)?)*
        }
    } => {
        $(#[$metas])*
        #[derive(num_enum::TryFromPrimitive,num_enum::IntoPrimitive, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        #[allow(clippy::empty_docs)]
        pub enum $enum_id{
            $(
                $(#[doc=$des])?
                $status=$value
            ),*
        }

    };
    {
        doc
        $(#[$metas:meta])*
        pub enum $enum_id:ident{
            $($status:tt $value:literal $des:literal)*
        }
    } => {
        $(#[$metas])*
        #[derive(num_enum::TryFromPrimitive,num_enum::IntoPrimitive, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        #[allow(clippy::empty_docs)]
        pub enum $enum_id{
            $(
                #[doc=$des]
                $status=$value
            ),*
        }
        impl ::std::fmt::Display for $enum_id{
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                write!(
                    f,
                    "{}",
                    match self{
                        $(Self::$status => $des),*
                    }
                )
            }
        }
        impl ::std::fmt::Debug for $enum_id{
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
                match self{
                    $(Self::$status => write!(f,"{}: {}", std::stringify!($status), $des)),*
                }
            }
        }
    }
}

pub mod assert;
pub mod attribute;
pub mod event;
pub mod gpib;
pub mod status;
