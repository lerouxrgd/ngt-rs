use std::ffi::CStr;
use std::fmt;

use ngt_sys as sys;

use crate::properties::{DistanceType, ObjectType};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error(pub(crate) String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

pub(crate) fn make_err(err: sys::NGTError) -> Error {
    let err_str = unsafe { CStr::from_ptr(sys::ngt_get_error_string(err)) };
    let err_msg = err_str.to_string_lossy().into();
    unsafe { sys::ngt_clear_error_string(err) };
    Error(err_msg)
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self(source.to_string())
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(source: std::num::TryFromIntError) -> Self {
        Self(source.to_string())
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(source: std::ffi::NulError) -> Self {
        Self(source.to_string())
    }
}

impl From<num_enum::TryFromPrimitiveError<ObjectType>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<ObjectType>) -> Self {
        Self(source.to_string())
    }
}

impl From<num_enum::TryFromPrimitiveError<DistanceType>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<DistanceType>) -> Self {
        Self(source.to_string())
    }
}
