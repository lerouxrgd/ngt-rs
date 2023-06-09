use std::ffi::CString;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use super::{QgObject, QgObjectType, QgProperties, QgQuantizationParams};
use crate::error::{make_err, Error, Result};
use crate::ngt::NgtIndex;
use crate::{SearchResult, VecId};

#[derive(Debug)]
pub struct QgIndex<T> {
    pub(crate) prop: QgProperties<T>,
    pub(crate) index: sys::NGTQGIndex,
    ebuf: sys::NGTError,
}

impl<T> QgIndex<T>
where
    T: QgObjectType,
{
    /// Quantize an NGT index
    pub fn quantize(index: NgtIndex<T>, params: QgQuantizationParams) -> Result<Self> {
        //
        if !is_x86_feature_detected!("avx2") {
            return Err(Error(
                "Cannot quantize an index without AVX2 support".into(),
            ));
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let path = index.path.clone();
            drop(index); // Close the index
            if !sys::ngtqg_quantize(path.as_ptr(), params.into_raw(), ebuf) {
                Err(make_err(ebuf))?
            }

            QgIndex::open(path.into_string()?)
        }
    }

    /// Open the already existing quantized index at the specified path.
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

            let index = sys::ngtqg_open_index(path.as_ptr(), ebuf);
            if index.is_null() {
                Err(make_err(ebuf))?
            }

            let prop = QgProperties::from(index)?;

            Ok(QgIndex {
                prop,
                index,
                ebuf: sys::ngt_create_error_object(),
            })
        }
    }

    pub fn search(&self, query: QgQuery<T>) -> Result<Vec<SearchResult>> {
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
    pub fn get_vec(&self, id: VecId) -> Result<Vec<T>> {
        unsafe {
            match self.prop.object_type {
                QgObject::Float => {
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

                    let results = results.iter().copied().collect::<Vec<_>>();
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
                QgObject::Uint8 => {
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

                    let results = results.iter().copied().collect::<Vec<_>>();
                    Ok(mem::transmute::<_, Vec<T>>(results))
                }
            }
        }
    }
}

impl<T> Drop for QgIndex<T> {
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

#[derive(Debug, Clone, PartialEq)]
pub struct QgQuery<'a, T> {
    query: &'a [T],
    pub size: u64,
    pub epsilon: f32,
    pub result_expansion: f32,
    pub radius: f32,
}

impl<'a, T> QgQuery<'a, T> {
    pub fn new(query: &'a [T]) -> Self {
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
    use std::iter::repeat;
    use std::result::Result as StdResult;

    use tempfile::tempdir;

    use super::*;
    use crate::{NgtDistance, NgtProperties};

    #[test]
    fn test_qg() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;

        // Create an NGT index for vectors
        let ndims = 3;
        let props = NgtProperties::<u8>::dimension(ndims)?.distance_type(NgtDistance::L2)?;
        let mut index = NgtIndex::create(dir.path(), props)?;

        // Insert vectors and get their ids
        let nvecs = 64;
        let ids = (1..ndims * nvecs)
            .step_by(ndims)
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
        index.build(1)?;
        index.persist()?;

        // Quantize the index
        let params = QgQuantizationParams {
            dimension_of_subvector: 1.,
            max_number_of_edges: 50,
        };
        let index = QgIndex::quantize(index, params)?;

        // Perform a vector search (with 2 results)
        let v: Vec<u8> = (1..=ndims).into_iter().map(|x| x as u8).collect();
        let query = QgQuery::new(&v).size(2);
        let res = index.search(query)?;
        assert_eq!(ids[0], res[0].id);
        assert_eq!(v, index.get_vec(ids[0])?);

        dir.close()?;
        Ok(())
    }
}
