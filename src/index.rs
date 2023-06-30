use std::convert::TryFrom;
use std::ffi::CString;
use std::fs;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Error, Result};
use crate::properties::{ObjectType, Properties};

pub const EPSILON: f32 = 0.1;

pub type VecId = u32;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: VecId,
    pub distance: f32,
}

#[derive(Debug)]
pub struct Index {
    pub(crate) path: CString,
    pub(crate) prop: Properties,
    pub(crate) index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
    ebuf: sys::NGTError,
}

unsafe impl Send for Index {}
unsafe impl Sync for Index {}

impl Index {
    /// Creates an empty ANNG index with the given [`Properties`]().
    pub fn create<P: AsRef<Path>>(path: P, prop: Properties) -> Result<Self> {
        if cfg!(feature = "shared_mem") && path.as_ref().exists() {
            Err(Error(format!("Path {:?} already exists", path.as_ref())))?
        }

        if let Some(path) = path.as_ref().parent() {
            fs::create_dir_all(path)?;
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let index = sys::ngt_create_graph_and_tree(path.as_ptr(), prop.raw_prop, ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }
            sys::ngt_close_index(index);

            let index = sys::ngt_open_index(path.as_ptr(), ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let ospace = sys::ngt_get_object_space(index, ebuf);
            if ospace.is_null() {
                Err(make_err(ebuf))?
            }

            Ok(Index {
                path,
                prop,
                index,
                ospace,
                ebuf: sys::ngt_create_error_object(),
            })
        }
    }

    /// Open the already existing index at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            Err(Error(format!("Path {:?} does not exist", path.as_ref())))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let index = sys::ngt_open_index(path.as_ptr(), ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let ospace = sys::ngt_get_object_space(index, ebuf);
            if ospace.is_null() {
                Err(make_err(ebuf))?
            }

            let prop = Properties::from(index)?;

            Ok(Index {
                path,
                prop,
                index,
                ospace,
                ebuf: sys::ngt_create_error_object(),
            })
        }
    }

    /// Search the nearest vectors to the specified query vector.
    ///
    /// **The index must have been [`built`](Index::build) beforehand**.
    pub fn search(&self, vec: &[f64], res_size: u64, epsilon: f32) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            if !sys::ngt_search_index(
                self.index,
                vec.as_ptr() as *mut f64,
                self.prop.dimension,
                res_size,
                epsilon,
                -1.0,
                results,
                self.ebuf,
            ) {
                Err(make_err(self.ebuf))?
            }

            let rsize = sys::ngt_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize as u32 {
                let d = sys::ngt_get_result(results, i, self.ebuf);
                if d.id == 0 && d.distance == 0.0 {
                    Err(make_err(self.ebuf))?
                } else {
                    ret.push(SearchResult {
                        id: d.id,
                        distance: d.distance,
                    });
                }
            }

            Ok(ret)
        }
    }

    /// Search linearly the nearest vectors to the specified query vector.
    ///
    /// **The index must have been [`built`](Index::build) beforehand**.
    pub fn linear_search(&self, vec: &[f64], res_size: u64) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            if !sys::ngt_linear_search_index(
                self.index,
                vec.as_ptr() as *mut f64,
                self.prop.dimension,
                res_size,
                results,
                self.ebuf,
            ) {
                Err(make_err(self.ebuf))?
            }

            let rsize = sys::ngt_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize as u32 {
                let d = sys::ngt_get_result(results, i, self.ebuf);
                if d.id == 0 && d.distance == 0.0 {
                    Err(make_err(self.ebuf))?
                } else {
                    ret.push(SearchResult {
                        id: d.id,
                        distance: d.distance,
                    });
                }
            }

            Ok(ret)
        }
    }

    /// Insert the specified vector into the index. However note that it is not
    /// discoverable yet.
    ///
    /// **The method [`build`](Index::build) must be called after inserting vectors**.
    pub fn insert<F: Into<f64>>(&mut self, vec: Vec<F>) -> Result<VecId> {
        unsafe {
            let mut vec = vec.into_iter().map(Into::into).collect::<Vec<f64>>();

            let id = sys::ngt_insert_index(
                self.index,
                vec.as_mut_ptr(),
                self.prop.dimension as u32,
                self.ebuf,
            );
            if id == 0 {
                Err(make_err(self.ebuf))?
            }

            Ok(id)
        }
    }

    /// Insert the multiple vectors into the index. However note that they are not
    /// discoverable yet.
    ///
    /// **The method [`build`](Index::build) must be called after inserting vectors**.
    pub fn insert_batch<F: Into<f64>>(&mut self, batch: Vec<Vec<F>>) -> Result<()> {
        let batch_size = u32::try_from(batch.len())?;

        if batch_size > 0 {
            let dim = batch[0].len();
            if dim != self.prop.dimension as usize {
                Err(Error(format!(
                    "Inconsistent batch dim, expected: {} got: {}",
                    self.prop.dimension, dim
                )))?;
            }
        } else {
            return Ok(());
        }

        unsafe {
            let mut batch = batch
                .into_iter()
                .flatten()
                .map(|v| v.into() as f32)
                .collect::<Vec<f32>>();

            if !sys::ngt_batch_append_index(self.index, batch.as_mut_ptr(), batch_size, self.ebuf) {
                Err(make_err(self.ebuf))?
            }

            Ok(())
        }
    }

    /// Build the index for the vectors that have been inserted so far.
    pub fn build(&mut self, num_threads: u32) -> Result<()> {
        unsafe {
            if !sys::ngt_create_index(self.index, num_threads, self.ebuf) {
                Err(make_err(self.ebuf))?
            }
            Ok(())
        }
    }

    /// Persist the index to the disk.
    pub fn persist(&mut self) -> Result<()> {
        unsafe {
            if !sys::ngt_save_index(self.index, self.path.as_ptr(), self.ebuf) {
                Err(make_err(self.ebuf))?
            }
            Ok(())
        }
    }

    /// Remove the specified vector.
    pub fn remove(&mut self, id: VecId) -> Result<()> {
        unsafe {
            if !sys::ngt_remove_index(self.index, id, self.ebuf) {
                Err(make_err(self.ebuf))?
            }
            Ok(())
        }
    }

    /// Get the specified vector.
    pub fn get_vec(&self, id: VecId) -> Result<Vec<f32>> {
        unsafe {
            let results = match self.prop.object_type {
                ObjectType::Float => {
                    let results = sys::ngt_get_object_as_float(self.ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut f32,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    results.iter().copied().collect::<Vec<_>>()
                }
                ObjectType::Uint8 => {
                    let results = sys::ngt_get_object_as_integer(self.ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut u8,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    results.iter().map(|byte| *byte as f32).collect::<Vec<_>>()
                }
            };

            Ok(results)
        }
    }

    /// The number of vectors inserted (but not necessarily indexed).
    pub fn nb_inserted(&self) -> u32 {
        unsafe { sys::ngt_get_number_of_objects(self.index, self.ebuf) }
    }

    /// The number of indexed vectors, available after [`build`](Index::build).
    pub fn nb_indexed(&self) -> u32 {
        unsafe { sys::ngt_get_number_of_indexed_objects(self.index, self.ebuf) }
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        if !self.index.is_null() {
            unsafe { sys::ngt_close_index(self.index) };
            self.index = ptr::null_mut();
        }
        if !self.ebuf.is_null() {
            unsafe { sys::ngt_destroy_error_object(self.ebuf) };
            self.ebuf = ptr::null_mut();
        }
    }
}

#[cfg(not(feature = "shared_mem"))]
#[derive(Debug)]
pub struct QGIndex {
    pub(crate) prop: Properties,
    pub(crate) index: sys::NGTQGIndex,
    ebuf: sys::NGTError,
}

#[cfg(not(feature = "shared_mem"))]
impl QGIndex {
    pub fn quantize(index: Index, params: QGQuantizationParams) -> Result<Self> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = index.path.clone();

            drop(index); // Close the index
            if !sys::ngtqg_quantize(path.as_ptr(), params.into_raw(), ebuf) {
                Err(make_err(ebuf))?
            }

            QGIndex::open(path.to_str().unwrap())
        }
    }

    /// Open the already existing quantized index at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            Err(Error(format!("Path {:?} does not exist", path.as_ref())))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let index = sys::ngtqg_open_index(path.as_ptr(), ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let prop = Properties::from(index)?;

            Ok(QGIndex {
                prop,
                index,
                ebuf: sys::ngt_create_error_object(),
            })
        }
    }

    pub fn search(&self, query: QGQuery) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            if !sys::ngtqg_search_index(self.index, query.into_raw(), results, self.ebuf) {
                Err(make_err(self.ebuf))?
            }

            let rsize = sys::ngt_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize as u32 {
                let d = sys::ngt_get_result(results, i, self.ebuf);
                if d.id == 0 && d.distance == 0.0 {
                    Err(make_err(self.ebuf))?
                } else {
                    ret.push(SearchResult {
                        id: d.id,
                        distance: d.distance,
                    });
                }
            }

            Ok(ret)
        }
    }

    /// Get the specified vector.
    pub fn get_vec(&self, id: VecId) -> Result<Vec<f32>> {
        unsafe {
            let results = match self.prop.object_type {
                ObjectType::Float => {
                    let ospace = sys::ngt_get_object_space(self.index, self.ebuf);
                    if ospace.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = sys::ngt_get_object_as_float(ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut f32,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    results.iter().copied().collect::<Vec<_>>()
                }
                ObjectType::Uint8 => {
                    let ospace = sys::ngt_get_object_space(self.index, self.ebuf);
                    if ospace.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = sys::ngt_get_object_as_integer(ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut u8,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    results.iter().map(|byte| *byte as f32).collect::<Vec<_>>()
                }
            };

            Ok(results)
        }
    }
}

#[cfg(not(feature = "shared_mem"))]
impl Drop for QGIndex {
    fn drop(&mut self) {
        if !self.index.is_null() {
            unsafe { sys::ngtqg_close_index(self.index) };
            self.index = ptr::null_mut();
        }
        if !self.ebuf.is_null() {
            unsafe { sys::ngt_destroy_error_object(self.ebuf) };
            self.ebuf = ptr::null_mut();
        }
    }
}

#[cfg(not(feature = "shared_mem"))]
#[derive(Debug, Clone, PartialEq)]
pub struct QGQuantizationParams {
    pub dimension_of_subvector: f32,
    pub max_number_of_edges: u64,
}

#[cfg(not(feature = "shared_mem"))]
impl Default for QGQuantizationParams {
    fn default() -> Self {
        Self {
            dimension_of_subvector: 0.0,
            max_number_of_edges: 128,
        }
    }
}

#[cfg(not(feature = "shared_mem"))]
impl QGQuantizationParams {
    unsafe fn into_raw(self) -> sys::NGTQGQuantizationParameters {
        sys::NGTQGQuantizationParameters {
            dimension_of_subvector: self.dimension_of_subvector,
            max_number_of_edges: self.max_number_of_edges,
        }
    }
}

#[cfg(not(feature = "shared_mem"))]
#[derive(Debug, Clone, PartialEq)]
pub struct QGQuery<'a> {
    query: &'a [f32],
    pub size: u64,
    pub epsilon: f32,
    pub result_expansion: f32,
    pub radius: f32,
}

#[cfg(not(feature = "shared_mem"))]
impl<'a> QGQuery<'a> {
    pub fn new(query: &'a [f32]) -> Self {
        Self {
            query,
            size: 20,
            epsilon: 0.03,
            result_expansion: 3.0,
            radius: f32::MAX,
        }
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn result_expansion(mut self, result_expansion: f32) -> Self {
        self.result_expansion = result_expansion;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    unsafe fn into_raw(self) -> sys::NGTQGQuery {
        sys::NGTQGQuery {
            query: self.query.as_ptr() as *mut f32,
            size: self.size,
            epsilon: self.epsilon,
            result_expansion: self.result_expansion,
            radius: self.radius,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::iter;
    use std::result::Result as StdResult;

    use rayon::prelude::*;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_basics() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = Properties::dimension(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Insert two vectors and get their id
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];
        let id1 = index.insert(vec1.clone())?;
        let id2 = index.insert(vec2.clone())?;
        assert!(index.nb_inserted() == 2);
        assert!(index.nb_indexed() == 0);

        // Actually build the index (not yet persisted on disk)
        // This is required in order to be able to search vectors
        index.build(2)?;
        assert!(index.nb_inserted() == 2);
        assert!(index.nb_indexed() == 2);

        // Perform a vector search (with 1 result)
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
        assert_eq!(id1, res[0].id);
        assert_eq!(vec1, index.get_vec(id1)?);

        // Perform a linear vector search (with 1 result)
        let res = index.linear_search(&vec![1.1, 2.1, 3.1], 1)?;
        assert_eq!(id1, res[0].id);
        assert_eq!(vec1, index.get_vec(id1)?);

        // Remove a vector and check that it is not present anymore
        index.remove(id1)?;
        let res = index.get_vec(id1);
        assert!(res.is_err());
        assert!(index.nb_inserted() == 1);
        assert!(index.nb_indexed() == 1);

        // Verify that now our search result is different
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
        assert_eq!(id2, res[0].id);
        assert_eq!(vec2, index.get_vec(id2)?);

        // Persist index on disk, and open it again
        index.persist()?;
        index = Index::open(dir.path())?;
        assert!(index.nb_inserted() == 1);
        assert!(index.nb_indexed() == 1);

        // Check that the removed vector wasn't persisted
        let res = index.get_vec(id1);
        assert!(res.is_err());

        // Verify that out search result is still consistent
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
        assert_eq!(id2, res[0].id);
        assert_eq!(vec2, index.get_vec(id2)?);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_batch() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = Properties::dimension(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Batch insert 2 vectors, build and persist the index
        index.insert_batch(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]])?;
        index.build(2)?;
        index.persist()?;

        // Verify that the index was built correctly with a vector search
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON)?;
        assert_eq!(1, res[0].id);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_multithreaded() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = Properties::dimension(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let vecs = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
            vec![8.0, 7.0, 6.0],
            vec![5.0, 4.0, 3.0],
            vec![2.0, 1.0, 6.0],
        ];

        // Batch insert multiple vectors, build and persist the index
        index.insert_batch(vecs.clone())?;
        index.build(2)?;
        index.persist()?;

        // Search the index with multiple threads
        iter::repeat(vecs.into_iter())
            .take(10_000)
            .flatten()
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|mut v| {
                v.iter_mut().for_each(|val| *val += 0.1);
                v
            })
            .map(|v| index.search(&v, 2, EPSILON))
            .collect::<Result<Vec<_>>>()?;

        dir.close()?;
        Ok(())
    }

    #[cfg(not(feature = "shared_mem"))]
    #[test]
    fn test_quantize() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = Properties::dimension(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Insert two vectors and get their id
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];
        let id1 = index.insert(vec1.clone())?;
        let _id2 = index.insert(vec2.clone())?;

        // Build and persist the index
        index.build(1)?;
        index.persist()?;

        let params = QGQuantizationParams::default();
        let index = QGIndex::quantize(index, params)?;

        // Perform a vector search (with 1 result)
        let vec = vec![1.1, 2.1, 3.1];
        let query = QGQuery::new(&vec).size(2);
        let res = index.search(query)?;
        assert_eq!(id1, res[0].id);
        assert_eq!(vec1, index.get_vec(id1)?);

        dir.close()?;
        Ok(())
    }

    // test adding and calling build multiple times
    #[cfg(not(feature = "shared_mem"))]
    #[test]
    fn test_incremental_insert_and_build() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = Properties::dimension(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        for _ in 0..120 {
            let vec = vec![1.0, 2.0, 3.0];
            let id = index.insert(vec.clone())?;
            println!("inserted vector with id {}", id);

            // Build and persist the index
            index.build(1)?;
            index.persist()?;
        }

        assert_eq!(0, 0);
        Ok(())
    }
}
