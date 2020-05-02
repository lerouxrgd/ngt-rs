pub mod properties;

use std::ffi::{CStr, CString};
use std::fmt;
use std::mem;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::properties::{ObjectType, Properties};

pub type VecId = u32;

#[derive(Debug)]
pub struct SearchResult {
    id: u32,
    distance: f32,
}

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(source: std::num::TryFromIntError) -> Self {
        Self(source.to_string())
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(source: std::ffi::NulError) -> Self {
        Self(source.to_string())
    }
}

impl std::error::Error for Error {}

fn make_err(err: sys::NGTError) -> Error {
    let err = unsafe { sys::ngt_get_error_string(err) };
    let err = unsafe { CStr::from_ptr(err) };
    Error(err.to_string_lossy().into())
}

pub struct Ngt {
    index_path: CString,
    prop: Properties,
    index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
}

impl Ngt {
    pub fn new<P: Into<PathBuf>>(index_path: P, prop: Properties) -> Result<Self, Error> {
        let index_path = CString::new(index_path.into().into_os_string().as_bytes())?;

        Ok(Ngt {
            index_path,
            prop,
            index: ptr::null_mut(),
            ospace: ptr::null_mut(),
        })
    }

    pub fn open(&mut self) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            self.index = sys::ngt_open_index(self.index_path.as_ptr(), ebuf);
            if self.index.is_null() {
                let err = make_err(ebuf);
                let is_prop_err = err
                    .0
                    .contains("PropertySet::load: Cannot load the property file ");

                if is_prop_err {
                    self.index = sys::ngt_create_graph_and_tree(
                        self.index_path.as_ptr(),
                        self.prop.raw_prop,
                        ebuf,
                    );
                    if self.index.is_null() {
                        Err(make_err(ebuf))?
                    }
                }
            } else {
                Err(make_err(ebuf))?
            }

            self.ospace = sys::ngt_get_object_space(self.index, ebuf);
            if self.ospace.is_null() {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn search(
        &self,
        vec: &[f64],
        size: u64,
        epsilon: f32,
        radius: f32,
    ) -> Result<Vec<SearchResult>, Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let results = sys::ngt_create_empty_results(ebuf);
            defer! { sys::ngt_destroy_results(results); }
            if results.is_null() {
                Err(make_err(ebuf))?
            }

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

            let mut result = Vec::with_capacity(rsize as usize);
            for i in 0..rsize as u32 {
                let d = sys::ngt_get_result(results, i, ebuf);
                if d.id == 0 && d.distance == 0.0 {
                    // Err(make_err(ebuf))?
                    continue;
                } else {
                    result.push(SearchResult {
                        id: d.id,
                        distance: d.distance,
                    });
                }
            }

            Ok(result)
        }
    }

    pub fn insert(&mut self, mut vec: Vec<f64>) -> Result<VecId, Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

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

    pub fn insert_commit(&mut self, vec: Vec<f64>, num_threads: u32) -> Result<VecId, Error> {
        let id = self.insert(vec)?;
        self.build_index(num_threads)?;
        self.save_index()?;
        Ok(id)
    }

    // pub fn insert_commit_bulk(
    //     &mut self,
    //     vecs: Vec<Vec<f64>>,
    //     num_threads: u32,
    // ) -> Result<Vec<VecId>, Error> {
    //     let mut ids = Vec::with_capacity(vecs.len());

    //     let mut idx = 0;
    //     for vec in vecs {
    //         let id = self.insert(vec)?;
    //         ids.push(id);
    //         idx += 1;
    //         if idx >= self.prop.bulk_insert_chunk_size {
    //             self.build_and_save_index(num_threads)?;
    //         }
    //     }

    //     self.build_and_save_index(num_threads)?;

    //     Ok(ids)
    // }

    pub fn build_index(&mut self, num_threads: u32) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_create_index(self.index, num_threads, ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn build_and_save_index(&mut self, num_threads: u32) -> Result<(), Error> {
        self.build_index(num_threads)?;
        self.save_index()
    }

    pub fn save_index(&mut self) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_save_index(self.index, self.index_path.as_ptr(), ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn remove(&mut self, id: VecId) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_remove_index(self.index, id, ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn get_vec(&self, id: VecId) -> Result<Vec<f32>, Error> {
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

    pub fn close(&mut self) {
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
    fn test_basics() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir().unwrap();

        let prop = Properties::new(3)?;

        let mut ngt = Ngt::new(dir.path(), prop)?;
        ngt.open()?;

        let id1 = ngt.insert(vec![1.0, 2.0, 3.0])?;
        let id2 = ngt.insert(vec![4.0, 5.0, 6.0])?;

        ngt.build_index(4)?;
        ngt.save_index()?;

        let res = ngt.search(&vec![1.1, 2.1, 3.1], 1, 0.01, -1.0)?;
        println!("-----> {:?}", res);

        let v = ngt.get_vec(id1)?;
        println!("-----> {:?}", v);
        let v = ngt.get_vec(id2)?;
        println!("-----> {:?}", v);

        ngt.remove(id1)?;

        let v = ngt.get_vec(id1).unwrap_err();
        println!("-----> {:?}", v);

        let v = ngt.get_vec(id2)?;
        println!("-----> {:?}", v);

        ngt.close();
        dir.close()?;

        Ok(())
    }
}
