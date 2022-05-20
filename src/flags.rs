use bitflags::bitflags;
use visa_sys as vs;

bitflags! {
    pub struct AccessMode: vs::ViAccessMode  {
        const NO_LOCK = vs::VI_NO_LOCK;
        const EXCLUSIVE_LOCK = vs::VI_EXCLUSIVE_LOCK;
        const SHARED_LOCK = vs::VI_SHARED_LOCK;
        const LOAD_CONFIG = vs::VI_LOAD_CONFIG;
    }
}

impl Default for AccessMode {
    fn default() -> Self {
        Self::NO_LOCK
    }
}

bitflags! {
    pub struct FlushMode: vs::ViUInt16  {
        const READ_BUF = vs::VI_READ_BUF as _ ;
        const READ_BUF_DISCARD = vs::VI_READ_BUF_DISCARD as _ ;
        const WRITE_BUF = vs::VI_WRITE_BUF as _;
        const WRITE_BUF_DISCARD  = vs::VI_WRITE_BUF_DISCARD as _;
        const IO_IN_BUF = vs::VI_IO_IN_BUF as _;
        const IO_IN_BUF_DISCARD = vs::VI_IO_IN_BUF_DISCARD as _;
        const IO_OUT_BUF = vs::VI_IO_OUT_BUF as _;
        const IO_OUT_BUF_DISCARD = vs::VI_IO_OUT_BUF_DISCARD as _;
    }
}


#[derive(enum_kinds::EnumKind)]
#[enum_kind(AttrKind)]
pub enum Attribute{

}