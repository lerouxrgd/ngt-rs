//! Rust wrappers for [NGT][], which provides high-speed approximate nearest neighbor
//! searches against a large volume of data.
//!
//! Note that NGT's shared memory and large dataset features are available through
//! cargo features `shared_mem` and `large_data` respectively.
//!
//! ## Usage
//!
//! Defining the properties of a new index:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{Properties, DistanceType, ObjectType};
//!
//! // Defaut properties with vectors of dimension 3
//! let prop = Properties::new(3)?;
//!
//! // Or customize values (here are the defaults)
//! let prop = Properties::new(3)?
//!     .creation_edge_size(10)?
//!     .search_edge_size(40)?
//!     .object_type(ObjectType::Float)?
//!     .distance_type(DistanceType::L2)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! Creating/Opening an index and using it:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{Index, Properties, EPSILON};
//!
//! // Create a new index
//! let prop = Properties::new(3)?;
//! let index = Index::create("target/path/to/index/dir", prop)?;
//!
//! // Open an existing index
//! let mut index = Index::open("target/path/to/index/dir")?;
//!
//! // Insert two vectors and get their id
//! let vec1 = vec![1.0, 2.0, 3.0];
//! let vec2 = vec![4.0, 5.0, 6.0];
//! let id1 = index.insert(vec1)?;
//! let id2 = index.insert(vec2)?;
//!
//! // Actually build the index (not yet persisted on disk)
//! // This is required in order to be able to search vectors
//! index.build(2)?;
//!
//! // Perform a vector search (with 1 result)
//! let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
//! assert_eq!(res[0].id, id1);
//! assert_eq!(index.get_vec(id1)?, vec![1.0, 2.0, 3.0]);
//!
//! // Remove a vector and check that it is not present anymore
//! index.remove(id1)?;
//! let res = index.get_vec(id1);
//! assert!(matches!(res, Result::Err(_)));
//!
//! // Verify that now our search result is different
//! let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
//! assert_eq!(res[0].id, id2);
//! assert_eq!(index.get_vec(id2)?, vec![4.0, 5.0, 6.0]);
//!
//! // Persist index on disk
//! index.persist()?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! [ngt]: https://github.com/yahoojapan/NGT

mod error;
mod index;
pub mod optim;
mod properties;

pub use crate::error::Error;
pub use crate::index::{Index, SearchResult, VecId, EPSILON};
pub use crate::properties::{DistanceType, ObjectType, Properties};
