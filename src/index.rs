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

// TODO: prevent search and remove on uncommmitted index

pub struct Index {
    path: CString,
    prop: Properties,
    index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
}

impl Index {
    pub fn create<P: AsRef<Path>>(path: P, prop: Properties) -> Result<Self> {
        unsafe {
            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

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
            })
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        unsafe {
            let path = CString::new(path.as_ref().as_os_str().as_bytes())?;

            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

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

            let rsize = sys::ngt_get_size(results, ebuf);
            if rsize < 0 {
                Err(make_err(ebuf))?
            }

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

            Ok(id)
        }
    }

    pub fn insert_commit<F: Into<f64>>(&mut self, vec: Vec<F>, num_threads: u32) -> Result<VecId> {
        let id = self.insert(vec)?;
        self.commit(num_threads)?;
        Ok(id)
    }

    pub fn insert_batch_commit<F: Into<f64>>(
        &mut self,
        batch: Vec<Vec<F>>,
        num_threads: u32,
    ) -> Result<()> {
        unsafe {
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

            self.commit(num_threads)?;

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
}

impl Drop for Index {
    fn drop(&mut self) {
        unsafe {
            if !self.index.is_null() {
                sys::ngt_close_index(self.index);
                self.index = ptr::null_mut();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_basics() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let prop = Properties::new(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];

        let id1 = index.insert(vec1.clone())?;
        let id2 = index.insert(vec2.clone())?;

        index.commit(2)?;

        assert_eq!(vec1, index.get_vec(id1)?);
        assert_eq!(vec2, index.get_vec(id2)?);

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(id1, res[0].id);

        index.remove(id1)?;
        let res = index.get_vec(id1);
        assert!(matches!(res, Result::Err(_)));

        index.persist()?;
        let index = Index::open(dir.path())?;
        assert_eq!(vec2, index.get_vec(id2)?);

        dir.close()?;
        Ok(())
    }

    #[test]
    fn test_batch() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let prop = Properties::new(3)?;
        let mut index = Index::create(dir.path(), prop)?;

        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![4.0, 5.0, 6.0];

        index.insert_batch_commit(vec![vec1, vec2], 2)?;

        let res = index.search(&vec![1.1, 2.1, 3.1], 1, EPSILON, RADIUS)?;
        assert_eq!(1, res[0].id);

        dir.close()?;
        Ok(())
    }
}
