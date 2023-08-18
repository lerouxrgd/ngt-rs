#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc = include_str!("../README.md")]
//!
//! # Usage
//!
//! Graph and tree based index (NGT Index)
//!
//! ## Defining the properties of a new NGT index:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{NgtProperties, NgtDistance};
//!
//! // Defaut properties with vectors of dimension 3
//! let prop = NgtProperties::<f32>::dimension(3)?;
//!
//! // Or customize values (here are the defaults)
//! let prop = NgtProperties::<f32>::dimension(3)?
//!     .creation_edge_size(10)?
//!     .search_edge_size(40)?
//!     .distance_type(NgtDistance::L2)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating/Opening a NGT index and using it:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{NgtIndex, NgtProperties};
//!
//! // Create a new index
//! let prop = NgtProperties::dimension(3)?;
//! let index: NgtIndex<f32> = NgtIndex::create("target/path/to/ngt_index/dir", prop)?;
//!
//! // Open an existing index
//! let mut index = NgtIndex::open("target/path/to/ngt_index/dir")?;
//!
//! // Insert two vectors and get their id
//! let vec1 = vec![1.0, 2.0, 3.0];
//! let vec2 = vec![4.0, 5.0, 6.0];
//! let id1 = index.insert(vec1)?;
//! let id2 = index.insert(vec2)?;
//!
//! // Build the index in RAM (not yet persisted on disk)
//! // This is required in order to be able to search vectors
//! index.build(2)?;
//!
//! // Perform a vector search (with 1 result)
//! let res = index.search(&vec![1.1, 2.1, 3.1], 1, ngt::EPSILON)?;
//! assert_eq!(res[0].id, id1);
//! assert_eq!(index.get_vec(id1)?, vec![1.0, 2.0, 3.0]);
//!
//! // Remove a vector and check that it is not present anymore
//! index.remove(id1)?;
//! let res = index.get_vec(id1);
//! assert!(res.is_err());
//!
//! // Verify that now our search result is different
//! let res = index.search(&vec![1.1, 2.1, 3.1], 1, ngt::EPSILON)?;
//! assert_eq!(res[0].id, id2);
//! assert_eq!(index.get_vec(id2)?, vec![4.0, 5.0, 6.0]);
//!
//! // Persist index on disk
//! index.persist()?;
//!
//! # std::fs::remove_dir_all("target/path/to/ngt_index/dir").unwrap();
//! # Ok(())
//! # }
//! ```

#[cfg(all(feature = "quantized", feature = "shared_mem"))]
compile_error!(r#"only one of ["quantized", "shared_mem"] can be enabled"#);

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

pub use half;
