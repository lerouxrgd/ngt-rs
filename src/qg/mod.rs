//! Quantized graph index (QG Index)
//!
//! ## Defining the properties of a new QG index:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::qg::{QgProperties, QgDistance};
//!
//! // Defaut properties with vectors of dimension 3
//! let prop = QgProperties::<f32>::dimension(3)?;
//!
//! // Or customize values (here are the defaults)
//! let prop = QgProperties::<f32>::dimension(3)?
//!     .creation_edge_size(10)?
//!     .search_edge_size(40)?
//!     .distance_type(QgDistance::L2)?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating/Opening a QG index and using it:
//!
//! ```rust
//! # fn main() -> Result<(), ngt::Error> {
//! use ngt::NgtIndex;
//! use ngt::qg::{QgDistance, QgIndex, QgProperties, QgQuantizationParams, QgQuery};
//!
//! // Create a new quantizable NGT index
//! let prop = QgProperties::dimension(3)?;
//! let mut index: NgtIndex<f32> =
//!     NgtIndex::create("target/path/to/qg_index/dir", prop.try_into()?)?;
//!
//! // Insert two vectors and get their id
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
//! index.build(1)?;
//! index.persist()?;
//!
//! // Quantize the NGT index
//! let params = QgQuantizationParams {
//!     dimension_of_subvector: 1.,
//!     max_number_of_edges: 50,
//! };
//! let index = QgIndex::quantize(index, params)?;
//!
//! // Open an existing QG index
//! let index = QgIndex::open("target/path/to/qg_index/dir")?;
//!
//! // Perform a vector search (with 1 result)
//! let query = vec![1.1, 2.1, 3.1];
//! let res = index.search(QgQuery::new(&query).size(1))?;
//! assert_eq!(res[0].id, id1);
//! assert_eq!(index.get_vec(id1)?, vec![1.0, 2.0, 3.0]);
//!
//! # std::fs::remove_dir_all("target/path/to/qg_index/dir").unwrap();
//! # Ok(())
//! # }
//! ```

mod index;
mod properties;

pub use self::index::{QgIndex, QgQuery};
pub use self::properties::{
    QgDistance, QgObject, QgObjectType, QgProperties, QgQuantizationParams,
};
