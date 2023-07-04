use std::ffi::CString;
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{mem, ptr};

use half::f16;
use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Error, Result};
use crate::{SearchResult, VecId};

use super::{QbgBuildParams, QbgConstructParams, QbgObject, QbgObjectType};

#[derive(Debug)]
pub struct QbgIndex<T, M> {
    pub(crate) index: sys::QBGIndex,
    path: CString,
    _mode: M,
    dimension: u32,
    ebuf: sys::NGTError,
    _marker: PhantomData<T>,
}

impl<T> QbgIndex<T, ModeWrite>
where
    T: QbgObjectType,
{
    pub fn create<P>(path: P, create_params: QbgConstructParams<T>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        if !is_x86_feature_detected!("avx2") {
            return Err(Error(
                "Cannot quantize an index without AVX2 support".into(),
            ));
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            if !sys::qbg_create(path.as_ptr(), &mut create_params.into_raw() as *mut _, ebuf) {
                Err(make_err(ebuf))?
            }

            let index = sys::qbg_open_index(path.as_ptr(), false, ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let dimension = sys::qbg_get_dimension(index, ebuf) as u32;
            if dimension == 0 {
                Err(make_err(ebuf))?
            }

            Ok(QbgIndex {
                index,
                path,
                _mode: ModeWrite,
                dimension,
                ebuf: sys::ngt_create_error_object(),
                _marker: PhantomData,
            })
        }
    }

    pub fn insert(&mut self, mut vec: Vec<T>) -> Result<VecId> {
        unsafe {
            let id = match T::as_obj() {
                QbgObject::Float => sys::qbg_append_object(
                    self.index,
                    vec.as_mut_ptr() as *mut _,
                    self.dimension,
                    self.ebuf,
                ),
                QbgObject::Uint8 => sys::qbg_append_object_as_uint8(
                    self.index,
                    vec.as_mut_ptr() as *mut _,
                    self.dimension,
                    self.ebuf,
                ),
                QbgObject::Float16 => sys::qbg_append_object_as_float16(
                    self.index,
                    vec.as_mut_ptr() as *mut _,
                    self.dimension,
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

    pub fn build(&mut self, build_params: QbgBuildParams) -> Result<()> {
        unsafe {
            if !sys::qbg_build_index(
                self.path.as_ptr(),
                &mut build_params.into_raw() as *mut _,
                self.ebuf,
            ) {
                Err(make_err(self.ebuf))?
            }
            Ok(())
        }
    }

    pub fn persist(&mut self) -> Result<()> {
        unsafe {
            if !sys::qbg_save_index(self.index, self.ebuf) {
                Err(make_err(self.ebuf))?
            }
            Ok(())
        }
    }

    pub fn into_readable(self) -> Result<QbgIndex<T, ModeRead>> {
        let path = self.path.clone();
        drop(self);
        QbgIndex::open(path.into_string()?)
    }
}

impl<T> QbgIndex<T, ModeRead>
where
    T: QbgObjectType,
{
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !is_x86_feature_detected!("avx2") {
            return Err(Error(
                "Cannot use a quantized index without AVX2 support".into(),
            ));
        }

        if !path.as_ref().exists() {
            Err(Error(format!("Path {:?} does not exist", path.as_ref())))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
            let index = sys::qbg_open_index(path.as_ptr(), true, ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let dimension = sys::qbg_get_dimension(index, ebuf) as u32;
            if dimension == 0 {
                Err(make_err(ebuf))?
            }

            Ok(QbgIndex {
                index,
                path,
                _mode: ModeRead,
                dimension,
                ebuf: sys::ngt_create_error_object(),
                _marker: PhantomData,
            })
        }
    }

    pub fn search(&self, query: QbgQuery<T>) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::qbg_destroy_results(results); }

            match T::as_obj() {
                QbgObject::Float => {
                    let q = sys::QBGQueryFloat {
                        query: query.query.as_ptr() as *mut f32,
                        params: query.params(),
                    };
                    if !sys::qbg_search_index_float(self.index, q, results, self.ebuf) {
                        Err(make_err(self.ebuf))?
                    }
                }
                QbgObject::Uint8 => {
                    let q = sys::QBGQueryUint8 {
                        query: query.query.as_ptr() as *mut u8,
                        params: query.params(),
                    };
                    if !sys::qbg_search_index_uint8(self.index, q, results, self.ebuf) {
                        Err(make_err(self.ebuf))?
                    }
                }
                QbgObject::Float16 => {
                    let q = sys::QBGQueryFloat16 {
                        query: query.query.as_ptr() as *mut _,
                        params: query.params(),
                    };
                    if !sys::qbg_search_index_float16(self.index, q, results, self.ebuf) {
                        Err(make_err(self.ebuf))?
                    }
                }
            }

            let rsize = sys::qbg_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize {
                let d = sys::qbg_get_result(results, i, self.ebuf);
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

    pub fn into_writable(self) -> Result<QbgIndex<T, ModeWrite>> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = self.path.clone();
            drop(self);

            let index = sys::qbg_open_index(path.as_ptr(), false, ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let dimension = sys::qbg_get_dimension(index, ebuf) as u32;
            if dimension == 0 {
                Err(make_err(ebuf))?
            }

            Ok(QbgIndex {
                index,
                path,
                _mode: ModeWrite,
                dimension,
                ebuf: sys::ngt_create_error_object(),
                _marker: PhantomData,
            })
        }
    }
}

impl<T, M> QbgIndex<T, M>
where
    T: QbgObjectType,
    M: IndexMode,
{
    /// Get the specified vector.
    pub fn get_vec(&self, id: VecId) -> Result<Vec<T>> {
        unsafe {
            match T::as_obj() {
                QbgObject::Float => {
                    let results = sys::qbg_get_object(self.index, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }
                    let results = Vec::from_raw_parts(
                        results as *mut f32,
                        self.dimension as usize,
                        self.dimension as usize,
                    );
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
                QbgObject::Uint8 => {
                    let results = sys::qbg_get_object_as_uint8(self.index, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }
                    let results = Vec::from_raw_parts(
                        results as *mut u8,
                        self.dimension as usize,
                        self.dimension as usize,
                    );
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
                QbgObject::Float16 => {
                    let results = sys::qbg_get_object_as_float16(self.index, id, self.ebuf);
                    if results.is_null() {
                        Err(make_err(self.ebuf))?
                    }
                    let results = Vec::from_raw_parts(
                        results as *mut f16,
                        self.dimension as usize,
                        self.dimension as usize,
                    );
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
            }
        }
    }
}

impl<T, M> Drop for QbgIndex<T, M> {
    fn drop(&mut self) {
        if !self.index.is_null() {
            unsafe { sys::qbg_close_index(self.index) };
            self.index = ptr::null_mut();
        }
        if !self.ebuf.is_null() {
            unsafe { sys::ngt_destroy_error_object(self.ebuf) };
            self.ebuf = ptr::null_mut();
        }
    }
}

mod private {
    pub trait Sealed {}
}

pub trait IndexMode: private::Sealed {}

#[derive(Debug, Clone, Copy)]
pub struct ModeRead;

impl private::Sealed for ModeRead {}
impl IndexMode for ModeRead {}

#[derive(Debug, Clone, Copy)]
pub struct ModeWrite;

impl private::Sealed for ModeWrite {}
impl IndexMode for ModeWrite {}

#[derive(Debug, Clone, PartialEq)]
pub struct QbgQuery<'a, T> {
    query: &'a [T],
    pub size: u64,
    pub epsilon: f32,
    pub blob_epsilon: f32,
    pub result_expansion: f32,
    pub number_of_explored_blobs: u64,
    pub number_of_edges: u64,
    pub radius: f32,
}

impl<'a, T> QbgQuery<'a, T>
where
    T: QbgObjectType,
{
    pub fn new(query: &'a [T]) -> Self {
        Self {
            query,
            size: 20,
            epsilon: 0.1,
            blob_epsilon: 0.0,
            result_expansion: 3.0,
            number_of_explored_blobs: 256,
            number_of_edges: 0,
            radius: 0.0,
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

    pub fn blob_epsilon(mut self, blob_epsilon: f32) -> Self {
        self.blob_epsilon = blob_epsilon;
        self
    }

    pub fn result_expansion(mut self, result_expansion: f32) -> Self {
        self.result_expansion = result_expansion;
        self
    }

    pub fn number_of_explored_blobs(mut self, number_of_explored_blobs: u64) -> Self {
        self.number_of_explored_blobs = number_of_explored_blobs;
        self
    }

    pub fn number_of_edges(mut self, number_of_edges: u64) -> Self {
        self.number_of_edges = number_of_edges;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    unsafe fn params(&self) -> sys::QBGQueryParameters {
        sys::QBGQueryParameters {
            number_of_results: self.size,
            epsilon: self.epsilon,
            blob_epsilon: self.blob_epsilon,
            result_expansion: self.result_expansion,
            number_of_explored_blobs: self.number_of_explored_blobs,
            number_of_edges: self.number_of_edges,
            radius: self.radius,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::iter::repeat;
    use std::result::Result as StdResult;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_qbg_f32() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        std::fs::remove_dir(dir.path())?;

        // Create a QGB index
        let ndims = 3;
        let mut index = QbgIndex::create(dir.path(), QbgConstructParams::dimension(ndims))?;

        // Insert vectors and get their ids
        let nvecs = 64;
        let ids = (1..ndims * nvecs)
            .step_by(ndims as usize)
            .map(|i| i as f32)
            .map(|i| {
                repeat(i)
                    .zip((0..ndims).map(|j| j as f32))
                    .map(|(i, j)| i + j)
                    .collect()
            })
            .map(|vector| index.insert(vector))
            .collect::<Result<Vec<_>>>()?;

        // Build and persist the index
        index.build(QbgBuildParams::default())?;
        index.persist()?;

        let index = index.into_readable()?;

        // Perform a vector search (with 2 results)
        let v: Vec<f32> = (1..=ndims).into_iter().map(|x| x as f32).collect();
        let query = QbgQuery::new(&v).size(2);
        let res = index.search(query)?;
        assert_eq!(ids[0], res[0].id);
        assert_eq!(v, index.get_vec(ids[0])?);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_qbg_f16() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        std::fs::remove_dir(dir.path())?;

        // Create a QGB index
        let ndims = 3;
        let mut index = QbgIndex::create(dir.path(), QbgConstructParams::dimension(ndims))?;

        // Insert vectors and get their ids
        let nvecs = 64;
        let ids = (1..ndims * nvecs)
            .step_by(ndims as usize)
            .map(|i| f16::from_f32(i as f32))
            .map(|i| {
                repeat(i)
                    .zip((0..ndims).map(|j| f16::from_f32(j as f32)))
                    .map(|(i, j)| i + j)
                    .collect()
            })
            .map(|vector| index.insert(vector))
            .collect::<Result<Vec<_>>>()?;

        // Build and persist the index
        index.build(QbgBuildParams::default())?;
        index.persist()?;

        let index = index.into_readable()?;

        // Perform a vector search (with 2 results)
        let v: Vec<f16> = (1..=ndims)
            .into_iter()
            .map(|x| f16::from_f32(x as f32))
            .collect();
        let query = QbgQuery::new(&v).size(2);
        let res = index.search(query)?;
        assert_eq!(ids[0], res[0].id);
        assert_eq!(v, index.get_vec(ids[0])?);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_qbg_u8() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        std::fs::remove_dir(dir.path())?;

        // Create a QGB index
        let ndims = 3;
        let mut index = QbgIndex::create(dir.path(), QbgConstructParams::dimension(ndims))?;

        // Insert vectors and get their ids
        let nvecs = 64;
        let ids = (1..ndims * nvecs)
            .step_by(ndims as usize)
            .map(|i| i as u8)
            .map(|i| {
                repeat(i)
                    .zip((0..ndims).map(|j| j as u8))
                    .map(|(i, j)| i + j)
                    .collect()
            })
            .map(|vector| index.insert(vector))
            .collect::<Result<Vec<_>>>()?;

        // Build and persist the index
        index.build(QbgBuildParams::default())?;
        index.persist()?;

        let index = index.into_readable()?;

        // Perform a vector search (with 3 results)
        let v: Vec<u8> = (1..=ndims).into_iter().map(|x| x as u8).collect();
        let query = QbgQuery::new(&v).size(3);
        let res = index.search(query)?;
        assert!(Vec::from_iter(res[0..3].iter().map(|r| r.id)).contains(&ids[0]));
        assert!(v == index.get_vec(ids[0])?);

        dir.close()?;
        Ok(())
    }
}
