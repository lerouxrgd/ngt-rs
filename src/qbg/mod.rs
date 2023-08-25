//! Quantized blob graph index (QBG Index)
//!
//! ## Defining the properties of a new QBG index:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::qbg::{QbgConstructParams, QbgDistance};
//!
//! // Defaut parameters with vectors of dimension 3
//! let params = QbgConstructParams::<f32>::dimension(3);
//!
//! // Or customize values (here are the defaults)
//! let params = QbgConstructParams::<f32>::dimension(3)
//!     .extended_dimension(16)? // next multiple of 16 after 3
//!     .number_of_subvectors(1)
//!     .number_of_subvectors(0)
//!     .distance_type(QbgDistance::L2);
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating/Opening a QBG index and using it:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! # std::fs::create_dir_all("target/path/to/qbg_index").unwrap();
//! use ngt::qbg::{
//!     ModeRead, ModeWrite, QbgBuildParams, QbgConstructParams, QbgDistance, QbgIndex, QbgQuery,
//! };
//!
//! // Create a new index
//! let params = QbgConstructParams::dimension(3);
//! let mut index: QbgIndex<f32, ModeWrite> =
//!     QbgIndex::create("target/path/to/qbg_index/dir", params)?;
//!
//! // Insert vectors and get their id
//! let vec1 = vec![1.0, 2.0, 3.0];
//! let vec2 = vec![4.0, 5.0, 6.0];
//! let id1 = index.insert(vec1)?;
//! let id2 = index.insert(vec2)?;
//!
//! // Add enough dummy vectors to build an index
//! for i in 0..64 {
//!     index.insert(vec![100. + i as f32; 3])?;
//! }
//! // Build the index in RAM and persist it on disk
//! index.build(QbgBuildParams::default())?;
//! index.persist()?;
//!
//! // Open an existing index
//! let index: QbgIndex<f32, ModeRead> = QbgIndex::open("target/path/to/qbg_index/dir")?;
//!
//! // Perform a vector search (with 1 result)
//! let query = vec![1.1, 2.1, 3.1];
//! let res = index.search(QbgQuery::new(&query).size(1))?;
//! assert_eq!(res[0].id, id1);
//! assert_eq!(index.get_vec(id1)?, vec![1.0, 2.0, 3.0]);
//!
//! # std::fs::remove_dir_all("target/path/to/qbg_index").unwrap();
//! # Ok(())
//! # }
//! ```

mod index;
mod properties;

pub use self::index::{IndexMode, ModeRead, ModeWrite, QbgIndex, QbgQuery};
pub use self::properties::{
    QbgBuildParams, QbgConstructParams, QbgDistance, QbgObject, QbgObjectType,
};
