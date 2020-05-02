use std::convert::TryFrom;
use std::ffi::CString;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Error, Result};
use crate::properties::{ObjectType, Properties};

pub type VecId = u32;

pub const EPSILON: f32 = 0.01;
pub const RADIUS: f32 = -1.0;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: u32,
    pub distance: f32,
}

#[derive(Debug)]
pub struct Index {
    path: CString,
    prop: Properties,
    index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
    is_committed: bool,
}

impl Index {
    pub fn create<P: AsRef<Path>>(path: P, prop: Properties) -> Result<Self> {
        if cfg!(feature = "shared_mem") && path.as_ref().exists() {
            Err(Error(format!("Path {:?} already exists", path.as_ref())))?
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
                is_committed: false,
            })
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
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
                is_committed: true,
            })
        }
    }

    pub fn search(
        &self,
        vec: &[f64],
        size: u64,
        epsilon: f32,
        radius: f32,
    ) -> Result<Vec<SearchResult>> {
        if !self.is_committed {
            Err(Error("Cannot search vecs in an uncommitted index".into()))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let results = sys::ngt_create_empty_results(ebuf);
            if results.is_null() {
                Err(make_err(ebuf))?
            }
            defer! { sys::ngt_destroy_results(results); }

            if !sys::ngt_search_index(
                self.index,
                vec.as_ptr() as *mut f64,
                self.prop.dimension,
                size,
                epsilon,
                radius,
                results,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            let rsize = sys::ngt_get_result_size(results, ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize as u32 {
                let d = sys::ngt_get_result(results, i, ebuf);
                if d.id == 0 && d.distance == 0.0 {
                    Err(make_err(ebuf))?
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

    pub fn insert<F: Into<f64>>(&mut self, vec: Vec<F>) -> Result<VecId> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let mut vec = vec.into_iter().map(Into::into).collect::<Vec<f64>>();

            let id = sys::ngt_insert_index(
                self.index,
                vec.as_mut_ptr(),
                self.prop.dimension as u32,
                ebuf,
            );
            if id == 0 {
                Err(make_err(ebuf))?
            }

            self.is_committed = false;
            Ok(id)
        }
    }

    pub fn insert_batch<F: Into<f64>>(&mut self, batch: Vec<Vec<F>>) -> Result<()> {
        let batch_size = u32::try_from(batch.len())?;

        if batch_size > 0 {
            let dim = batch[0].len();
            if dim != self.prop.dimension as usize {
                Err(Error(
                    format!(
                        "Inconsistent batch dim, expected: {} got: {}",
                        self.prop.dimension, dim
                    )
                    .into(),
                ))?;
            }
        } else {
            return Ok(());
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let mut batch = batch
                .into_iter()
                .flatten()
                .map(|v| v.into() as f32)
                .collect::<Vec<f32>>();

            if !sys::ngt_batch_append_index(self.index, batch.as_mut_ptr(), batch_size, ebuf) {
                Err(make_err(ebuf))?
            }

            self.is_committed = false;
            Ok(())
        }
    }

    pub fn commit(&mut self, num_threads: u32) -> Result<()> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_create_index(self.index, num_threads, ebuf) {
                Err(make_err(ebuf))?
            }

            self.is_committed = true;
            Ok(())
        }
    }

    pub fn commit_and_persist(&mut self, num_threads: u32) -> Result<()> {
        self.commit(num_threads)?;
        self.persist()
    }

    pub fn persist(&mut self) -> Result<()> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_save_index(self.index, self.path.as_ptr(), ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn remove(&mut self, id: VecId) -> Result<()> {
        if !self.is_committed {
            Err(Error("Cannot remove vec from an uncommitted index".into()))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_remove_index(self.index, id, ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn get_vec(&self, id: VecId) -> Result<Vec<f32>> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let results = match self.prop.object_type {
                ObjectType::Float => {
                    let results = sys::ngt_get_object_as_float(self.ospace, id, ebuf);
                    if results.is_null() {
                        Err(make_err(ebuf))?
                    }

                    let results = Vec::from_raw_parts(
                        results as *mut f32,
                        self.prop.dimension as usize,
                        self.prop.dimension as usize,
                    );
                    let results = mem::ManuallyDrop::new(results);

                    results.iter().map(|v| *v).collect::<Vec<_>>()
                }
                ObjectType::Uint8 => {
                    let results = sys::ngt_get_object_as_integer(self.ospace, id, ebuf);
                    if results.is_null() {
                        Err(make_err(ebuf))?
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

    pub fn refine_anng(
        &mut self,
        epsilon: f32,
        expected_accuracy: f32,
        nb_edges: i32,
        edge_size: i32,
        batch_size: u64,
    ) -> Result<()> {
        if !self.is_committed {
            Err(Error("Cannot refine an uncommitted index".into()))?
        }

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_refine_anng(
                self.index,
                epsilon,
                expected_accuracy,
                nb_edges,
                edge_size,
                batch_size,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        if !self.index.is_null() {
            unsafe { sys::ngt_close_index(self.index) };
            self.index = ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::result::Result as StdResult;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_basics() -> StdResult<(), Box<dyn StdError>> {
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        let prop = Properties::new(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];
        let id1 = index.insert(vec1.clone())?;
        let id2 = index.insert(vec2.clone())?;

        index.commit(2)?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id1, res[0].id);
        assert_eq!(vec1, index.get_vec(id1)?);

        index.remove(id1)?;

        let res = index.get_vec(id1);
        assert!(matches!(res, Result::Err(_)));
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id2, res[0].id);
        assert_eq!(vec2, index.get_vec(id2)?);

        index.persist()?;
        let index = Index::open(dir.path())?;

        let res = index.get_vec(id1);
        assert!(matches!(res, Result::Err(_)));
        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id2, res[0].id);
        assert_eq!(vec2, index.get_vec(id2)?);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_batch() -> StdResult<(), Box<dyn StdError>> {
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        let prop = Properties::new(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];

        index.insert_batch(vec![vec1, vec2])?;
        index.commit_and_persist(2)?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(1, res[0].id);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_uncommitted() -> StdResult<(), Box<dyn StdError>> {
        let dir = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir.path())?;
        }

        let prop = Properties::new(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS);
        assert!(matches!(res, Result::Err(_)));

        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];

        let id1 = index.insert(vec1.clone())?;
        let id2 = index.insert(vec2.clone())?;
        assert_eq!(vec1, index.get_vec(id1)?);
        assert_eq!(vec2, index.get_vec(id2)?);

        let res = index.remove(id1);
        assert!(matches!(res, Result::Err(_)));

        index.commit(2)?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id1, res[0].id);

        index.remove(id1)?;
        let res = index.get_vec(id1);
        assert!(matches!(res, Result::Err(_)));

        index.persist()?;
        let index = Index::open(dir.path())?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id2, res[0].id);
        assert_eq!(vec2, index.get_vec(id2)?);

        dir.close()?;
        Ok(())
    }
}
