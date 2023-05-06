//! Defining the properties of a new index:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{NgtProperties, NgtDistance, NgtObject};
//!
//! // Defaut properties with vectors of dimension 3
//! let prop = NgtProperties::dimension(3)?;
//!
//! // Or customize values (here are the defaults)
//! let prop = NgtProperties::dimension(3)?
//!     .creation_edge_size(10)?
//!     .search_edge_size(40)?
//!     .object_type(NgtObject::Float)?
//!     .distance_type(NgtDistance::L2)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! Creating/Opening an index and using it:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::{NgtIndex, NgtProperties, EPSILON};
//!
//! // Create a new index
//! let prop = NgtProperties::dimension(3)?;
//! let index = NgtIndex::create("target/path/to/index/dir", prop)?;
//!
//! // Open an existing index
//! let mut index = NgtIndex::open("target/path/to/index/dir")?;
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
//! # std::fs::remove_dir_all("target/path/to/index/dir").unwrap();
//! # Ok(())
//! # }
//! ```

mod index;
pub mod optim;
mod properties;

pub use self::index::NgtIndex;
pub use self::properties::{NgtDistance, NgtObject, NgtProperties};
