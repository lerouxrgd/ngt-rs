use std::convert::TryFrom;
use std::ffi::CString;
use std::fs;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use super::{NgtObject, NgtObjectType, NgtProperties};
use crate::error::{make_err, Error, Result};
use crate::{SearchResult, VecId};

#[derive(Debug)]
pub struct NgtIndex<T> {
    pub(crate) path: CString,
    pub(crate) prop: NgtProperties<T>,
    pub(crate) index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
    ebuf: sys::NGTError,
}

unsafe impl<T> Send for NgtIndex<T> {}
unsafe impl<T> Sync for NgtIndex<T> {}

impl<T> NgtIndex<T>
where
    T: NgtObjectType,
{
    /// Creates an empty ANNG index with the given [`NgtProperties`][].
    pub fn create<P: AsRef<Path>>(path: P, prop: NgtProperties<T>) -> Result<Self> {
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

            Ok(NgtIndex {
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

            let prop = NgtProperties::from(index)?;

            Ok(NgtIndex {
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
    /// **The index must have been [`built`](NgtIndex::build) beforehand**.
    pub fn search(&self, vec: &[T], res_size: usize, epsilon: f32) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            match T::as_obj() {
                NgtObject::Float => {
                    if !sys::ngt_search_index_as_float(
                        self.index,
                        vec.as_ptr() as *mut f32,
                        self.prop.dimension,
                        res_size,
                        epsilon,
                        -1.0,
                        results,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Uint8 => {
                    if !sys::ngt_search_index_as_uint8(
                        self.index,
                        vec.as_ptr() as *mut u8,
                        self.prop.dimension,
                        res_size,
                        epsilon,
                        -1.0,
                        results,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Float16 => {
                    if !sys::ngt_search_index_as_float16(
                        self.index,
                        vec.as_ptr() as *mut _,
                        self.prop.dimension,
                        res_size,
                        epsilon,
                        -1.0,
                        results,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
            }

            let rsize = sys::ngt_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize {
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

    /// Search the nearest vectors to the specified [`NgtQuery`][].
    ///
    /// **The index must have been [`built`](NgtIndex::build) beforehand**.
    pub fn search_query(&self, query: NgtQuery<T>) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            match T::as_obj() {
                NgtObject::Float => {
                    let q = sys::NGTQueryFloat {
                        query: query.query.as_ptr() as *mut f32,
                        params: query.params(),
                    };
                    if !sys::ngt_search_index_with_query_float(self.index, q, results, self.ebuf) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Uint8 => {
                    let q = sys::NGTQueryUint8 {
                        query: query.query.as_ptr() as *mut u8,
                        params: query.params(),
                    };
                    if !sys::ngt_search_index_with_query_uint8(self.index, q, results, self.ebuf) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Float16 => {
                    let q = sys::NGTQueryFloat16 {
                        query: query.query.as_ptr() as *mut _,
                        params: query.params(),
                    };
                    if !sys::ngt_search_index_with_query_float16(self.index, q, results, self.ebuf)
                    {
                        Err(make_err(self.ebuf))?
                    }
                }
            }

            let rsize = sys::ngt_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize {
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
    /// **The method [`build`](NgtIndex::build) must be called after inserting vectors**.
    pub fn insert(&mut self, mut vec: Vec<T>) -> Result<VecId> {
        unsafe {
            let id = match self.prop.object_type {
                NgtObject::Float => sys::ngt_insert_index_as_float(
                    self.index,
                    vec.as_mut_ptr() as *mut f32,
                    self.prop.dimension as u32,
                    self.ebuf,
                ),
                NgtObject::Uint8 => sys::ngt_insert_index_as_uint8(
                    self.index,
                    vec.as_mut_ptr() as *mut u8,
                    self.prop.dimension as u32,
                    self.ebuf,
                ),
                NgtObject::Float16 => sys::ngt_insert_index_as_float16(
                    self.index,
                    vec.as_mut_ptr() as *mut _,
                    self.prop.dimension as u32,
                    self.ebuf,
                ),
            };
            if id == 0 {
                Err(make_err(self.ebuf))?
            } else {
                Ok(id)
            }
        }
    }

    /// Insert the multiple vectors into the index. However note that they are not
    /// discoverable yet.
    ///
    /// **The method [`build`](NgtIndex::build) must be called after inserting vectors**.
    pub fn insert_batch(&mut self, batch: Vec<Vec<T>>) -> Result<()> {
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
            let mut batch = batch.into_iter().flatten().collect::<Vec<T>>();
            match self.prop.object_type {
                NgtObject::Float => {
                    if !sys::ngt_batch_append_index(
                        self.index,
                        batch.as_mut_ptr() as *mut f32,
                        batch_size,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Uint8 => {
                    if !sys::ngt_batch_append_index_as_uint8(
                        self.index,
                        batch.as_mut_ptr() as *mut u8,
                        batch_size,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
                NgtObject::Float16 => {
                    if !sys::ngt_batch_append_index_as_float16(
                        self.index,
                        batch.as_mut_ptr() as *mut _,
                        batch_size,
                        self.ebuf,
                    ) {
                        Err(make_err(self.ebuf))?
                    }
                }
            }
            Ok(())
        }
    }

    /// Build the index for the vectors that have been inserted so far.
    pub fn build(&mut self, num_threads: usize) -> Result<()> {
        unsafe {
            if !sys::ngt_create_index(self.index, num_threads as u32, self.ebuf) {
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
    pub fn get_vec(&self, id: VecId) -> Result<Vec<T>> {
        unsafe {
            match self.prop.object_type {
                NgtObject::Float => {
                    let results = sys::ngt_get_object_as_float(self.ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    let results = results.iter().copied().collect::<Vec<_>>();
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
                NgtObject::Float16 => {
                    let results = sys::ngt_get_object(self.ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut half::f16,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    let results = results.iter().copied().collect::<Vec<_>>();
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
                NgtObject::Uint8 => {
                    let results = sys::ngt_get_object_as_integer(self.ospace, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    let results = results.iter().copied().collect::<Vec<_>>();
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
            }
        }
    }

    /// The number of vectors inserted (but not necessarily indexed).
    pub fn nb_inserted(&self) -> usize {
        unsafe { sys::ngt_get_number_of_objects(self.index, self.ebuf) as usize }
    }

    /// The number of indexed vectors, available after [`build`](NgtIndex::build).
    pub fn nb_indexed(&self) -> usize {
        unsafe { sys::ngt_get_number_of_indexed_objects(self.index, self.ebuf) as usize }
    }
}

impl<T> Drop for NgtIndex<T> {
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

#[derive(Debug, Clone, PartialEq)]
pub struct NgtQuery<'a, T> {
    query: &'a [T],
    pub size: usize,
    pub epsilon: f32,
    pub edge_size: usize,
    pub radius: f32,
}

impl<'a, T> NgtQuery<'a, T>
where
    T: NgtObjectType,
{
    pub fn new(query: &'a [T]) -> Self {
        Self {
            query,
            size: 10,
            epsilon: crate::EPSILON,
            edge_size: usize::MIN,
            radius: -1.,
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn epsilon(mut self, epsilon: f32) -> Self {
        self.epsilon = epsilon;
        self
    }

    pub fn edge_size(mut self, edge_size: usize) -> Self {
        self.edge_size = edge_size;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    unsafe fn params(&self) -> sys::NGTQueryParameters {
        sys::NGTQueryParameters {
            size: self.size,
            epsilon: self.epsilon,
            edge_size: self.edge_size,
            radius: self.radius,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::iter;
    use std::result::Result as StdResult;

    use half::f16;
    use rayon::prelude::*;
    use tempfile::tempdir;

    use super::*;
    use crate::EPSILON;

    #[test]
    fn test_ngt_f32_basics() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = NgtProperties::<f32>::dimension(3)?;
        let mut index = NgtIndex::create(dir.path(), prop)?;

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

        // Perform a vector search (using the NgtQuery API)
        let query = vec![1.1, 2.1, 3.1];
        let res = index.search_query(NgtQuery::new(&query))?;
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
        index = NgtIndex::open(dir.path())?;
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
    fn test_ngt_batch() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = NgtProperties::<f32>::dimension(3)?;
        let mut index = NgtIndex::create(dir.path(), prop)?;

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
    fn test_ngt_u8() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = NgtProperties::<u8>::dimension(3)?;
        let mut index = NgtIndex::create(dir.path(), prop)?;

        // Insert 3 vectors, build and persist the index
        index.insert_batch(vec![vec![1, 2, 3], vec![4, 5, 6]])?;
        index.insert(vec![7, 8, 9])?;
        index.build(2)?;
        index.persist()?;

        // Verify that the index was built correctly with a vector search
        let res = index.search(&vec![1, 2, 3], 1, EPSILON)?;
        assert_eq!(1, res[0].id);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_ngt_f16() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = NgtProperties::<f16>::dimension(3)?;
        let mut index = NgtIndex::create(dir.path(), prop)?;

        // Insert 3 vectors, build and persist the index
        index.insert_batch(vec![
            vec![1.0, 2.0, 3.0].into_iter().map(f16::from_f32).collect(),
            vec![4.0, 5.0, 6.0].into_iter().map(f16::from_f32).collect(),
        ])?;
        index.insert(vec![7.0, 8.0, 9.0].into_iter().map(f16::from_f32).collect())?;
        index.build(2)?;
        index.persist()?;

        // Verify that the index was built correctly with a vector search
        let res = index.search(
            &vec![1.1, 2.1, 3.1]
                .into_iter()
                .map(f16::from_f32)
                .collect::<Vec<_>>(),
            1,
            EPSILON,
        )?;
        assert_eq!(1, res[0].id);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_ngt_multithreaded() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        // Create an index for vectors of dimension 3
        let prop = NgtProperties::<f32>::dimension(3)?;
        let mut index = NgtIndex::create(dir.path(), prop)?;

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
}
