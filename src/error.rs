use std::ffi::CStr;
use std::fmt;

use ngt_sys as sys;

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

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self(err)
    }
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

impl From<std::ffi::IntoStringError> for Error {
    fn from(source: std::ffi::IntoStringError) -> Self {
        Self(source.to_string())
    }
}

impl From<num_enum::TryFromPrimitiveError<crate::NgtObject>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::NgtObject>) -> Self {
        Self(source.to_string())
    }
}

impl From<num_enum::TryFromPrimitiveError<crate::NgtDistance>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::NgtDistance>) -> Self {
        Self(source.to_string())
    }
}

#[cfg(feature = "quantized")]
impl From<num_enum::TryFromPrimitiveError<crate::qg::QgObject>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::qg::QgObject>) -> Self {
        Self(source.to_string())
    }
}

#[cfg(feature = "quantized")]
impl From<num_enum::TryFromPrimitiveError<crate::qg::QgDistance>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::qg::QgDistance>) -> Self {
        Self(source.to_string())
    }
}

#[cfg(feature = "quantized")]
impl From<num_enum::TryFromPrimitiveError<crate::qbg::QbgObject>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::qbg::QbgObject>) -> Self {
        Self(source.to_string())
    }
}

#[cfg(feature = "quantized")]
impl From<num_enum::TryFromPrimitiveError<crate::qbg::QbgDistance>> for Error {
    fn from(source: num_enum::TryFromPrimitiveError<crate::qbg::QbgDistance>) -> Self {
        Self(source.to_string())
    }
}
