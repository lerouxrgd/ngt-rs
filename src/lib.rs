#![doc = include_str!("../README.md")]

#[cfg(all(feature = "quantized", feature = "shared_mem"))]
compile_error!("only one of ['quantized', 'shared_mem'] can be enabled");

mod error;
mod ngt;
#[cfg(feature = "quantized")]
pub mod qbg;
#[cfg(feature = "quantized")]
pub mod qg;

pub type VecId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: VecId,
    pub distance: f32,
}

pub const EPSILON: f32 = 0.1;

pub use crate::error::{Error, Result};
pub use crate::ngt::{optim, NgtDistance, NgtIndex, NgtObject, NgtProperties};

#[doc(inline)]
pub use half;
