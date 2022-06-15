macro_rules! consts_to_enum {
    {
        #[format=$fmt:ident]
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:ident $value:literal $($des:literal)?)*
        }
    } => {
        visa_rs_proc::rusty_ident!{
            consts_to_enum!{
                $fmt
                pub enum $enum_id $(:$align)?{
                    $($status $value $($des)?)*
                }
            }
        }

    };
    

    {   dbg
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:ident $value:literal $($des:literal)?)*
        }
    } => {
        $(#[repr($align)])?
        #[derive(num_enum::TryFromPrimitive,num_enum::IntoPrimitive, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
        pub enum $enum_id{
            $(
                $(#[doc=$des])?
                $status=$value as _
            ),*
        }

    };
    {
        doc
        pub enum $enum_id:ident $(:$align:ty)?{
            $($status:tt $value:literal $des:literal)*
        }
    } => {
        $(#[repr($align)])?
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
    }
}

pub mod attribute;
pub mod event;
pub mod status;
