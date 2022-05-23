macro_rules! consts_to_enum {
    {
        pub enum $enum_id:ident{
            Completion Codes	Values	Meaning
            $($status:ident $value:literal $des:literal)*
        }
    } => {
        visa_rs_proc::rusty_ident!{
            consts_to_enum!{
                @inner
                pub enum $enum_id{
                    Completion Codes	Values	Meaning
                    $($status $value $des)*
                }
            }
        }
    };
    {   @inner
        pub enum $enum_id:ident{
            Completion Codes	Values	Meaning
            $($status:tt $value:literal $des:literal)*
        }
    } => {
        #[repr(i32)]
        #[derive(num_enum::TryFromPrimitive,num_enum::IntoPrimitive, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        pub enum $enum_id{
            $(
                #[doc=$des]
                $status=$value as _
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
                    $(Self::$status => write!(f,"{:#010X}: {}", $value, $des)),*
                }
            }
        }
    };
}

mod attribute;
mod status;

pub use attribute::*;
pub use status::*;
