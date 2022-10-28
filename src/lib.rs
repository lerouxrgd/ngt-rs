//! Rust wrappers for [NGT][], which provides high-speed approximate nearest neighbor
//! searches against a large volume of data.
//!
//! Building NGT requires `CMake`. By default `ngt-rs` will be built dynamically, which
//! means that you'll need to make the build artifact `libngt.so` available to your final
//! binary. You'll also need to have `OpenMP` installed on the system where it will run. If
//! you want to build `ngt-rs` statically, then use the `static` Cargo feature.
//!
//! Furthermore, NGT's shared memory and large dataset features are available through Cargo
//! features `shared_mem` and `large_data` respectively.
//!
//! [ngt]: https://github.com/yahoojapan/NGT

// TODO: consider include_str README

#[cfg(all(feature = "quantized", feature = "shared_mem"))]
compile_error!("only one of ['quantized', 'shared_mem'] can be enabled");

mod error;
mod ngt;

#[cfg(feature = "quantized")]
pub mod qg;

#[cfg(feature = "quantized")]
pub mod qbg;

pub type VecId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: VecId,
    pub distance: f32,
}

pub const EPSILON: f32 = 0.1;

pub use crate::error::{Error, Result};
pub use crate::ngt::{NgtDistance, NgtIndex, NgtObject, NgtProperties};

// TODO: search etc only f32, drop support for f64/Into<f64>
// TODO: what about float16 ?
