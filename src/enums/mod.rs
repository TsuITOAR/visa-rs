macro_rules! consts_to_enum {
    {
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:ident $value:literal $des:literal)*
        }
    } => {
        visa_rs_proc::rusty_ident!{
            consts_to_enum!{
                @enum
                pub enum $enum_id $(:$align)?{
                    $($status $value)*
                }
            }
            consts_to_enum!{
                @fmt
                pub enum $enum_id{
                    $($status $value $des)*
                }
            }
        }

    };
    {
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:ident $value:literal)*
        }
    } => {
        visa_rs_proc::rusty_ident!{
            consts_to_enum!{
                @enum
                pub enum $enum_id $(:$align)?{
                    $($status $value)*
                }
            }
        }
    };

    {   @enum
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:tt $value:literal $($des:literal)?)*
        }
    } => {
         $(#[repr($align)])?
        #[derive(num_enum::TryFromPrimitive,num_enum::IntoPrimitive, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        pub enum $enum_id{
            $(
                $(#[doc=$des])?
                $status=$value as _
            ),*
        }

    };
    {
        @fmt
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:tt $value:literal $des:literal)*
        }
    } => {
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
                    $(Self::$status => write!(f,"{:#010X}: {}", $value, $des)),*
                }
            }
        }
    }
}

macro_rules! pass_compile {
    ($($anything:tt)*) => {};
}

mod attribute;
mod status;

pub use attribute::*;
pub use status::*;
