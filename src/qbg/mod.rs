mod index;

use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum QbgObject {
    Uint8 = 0,
    Float = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum QbgDistance {
    L2 = 1,
}

pub use self::index::{
    IndexMode, ModeRead, ModeWrite, QbgBuildParams, QbgConstructParams, QbgIndex, QbgQuery,
};
