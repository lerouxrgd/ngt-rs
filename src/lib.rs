#![allow(dead_code)]

use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::mem;
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

use ngt_sys as sys;
use num_enum::TryFromPrimitive;
use scopeguard::defer;

type VecId = u32;

#[derive(Debug, TryFromPrimitive)]
#[repr(i32)]
enum ObjectType {
    Uint8 = 1,
    Float = 2,
}

#[derive(Debug)]
#[non_exhaustive]
enum DistanceType {
    L1,
    L2,
    Angle,
    Hamming,
    Cosine,
    // NormalizedAngle,  // Not implemented in C API
    // NormalizedCosine, // Not implemented in C API
}

#[derive(Debug)]
struct SearchResult {
    id: u32,
    distance: f32,
}

#[derive(Debug)]
struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

fn make_err(err: sys::NGTError) -> Error {
    let err = unsafe { sys::ngt_get_error_string(err) };
    let err = unsafe { CStr::from_ptr(err) };
    Error(err.to_string_lossy().into())
}

#[derive(Debug)]
pub struct Property {
    dimension: i32,
    creation_edge_size: i16,
    search_edge_size: i16,
    object_type: ObjectType,
    distance_type: DistanceType,
    index_path: CString,
    bulk_insert_chunk_size: usize,
}

impl Default for Property {
    fn default() -> Self {
        Property {
            dimension: 0,
            creation_edge_size: 10,
            search_edge_size: 40,
            object_type: ObjectType::Float,
            distance_type: DistanceType::L2,
            index_path: CString::new(format!(
                "/tmp/ngt-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("back to the future")
                    .as_secs()
            ))
            .expect("default index_path contains an internal 0 byte"),
            bulk_insert_chunk_size: 100,
        }
    }
}

struct Ngt {
    prop: Property,
    index: sys::NGTIndex,
    ospace: sys::NGTObjectSpace,
}

impl Ngt {
    fn new(prop: Property) -> Self {
        Ngt {
            prop,
            index: ptr::null_mut(),
            ospace: ptr::null_mut(),
        }
    }

    fn open(&mut self) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let prop = sys::ngt_create_property(ebuf);
            if prop.is_null() {
                Err(make_err(ebuf))?
            }
            defer! { sys::ngt_destroy_property(prop); }

            if !sys::ngt_set_property_dimension(prop, self.prop.dimension, ebuf) {
                Err(make_err(ebuf))?
            }

            if !sys::ngt_set_property_edge_size_for_creation(
                prop,
                self.prop.creation_edge_size,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            if !sys::ngt_set_property_edge_size_for_search(prop, self.prop.search_edge_size, ebuf) {
                Err(make_err(ebuf))?
            }

            match self.prop.object_type {
                ObjectType::Uint8 => {
                    if !sys::ngt_set_property_object_type_integer(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
                ObjectType::Float => {
                    if !sys::ngt_set_property_object_type_float(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
            }

            match self.prop.distance_type {
                DistanceType::L1 => {
                    if !sys::ngt_set_property_distance_type_l1(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
                DistanceType::L2 => {
                    if !sys::ngt_set_property_distance_type_l2(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
                DistanceType::Angle => {
                    if !sys::ngt_set_property_distance_type_angle(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
                DistanceType::Hamming => {
                    if !sys::ngt_set_property_distance_type_hamming(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
                DistanceType::Cosine => {
                    if !sys::ngt_set_property_distance_type_cosine(prop, ebuf) {
                        Err(make_err(ebuf))?
                    }
                }
            }

            self.index = sys::ngt_open_index(self.prop.index_path.as_ptr(), ebuf);
            if self.index.is_null() {
                let err = make_err(ebuf);
                let is_prop_err = err
                    .0
                    .contains("PropertySet::load: Cannot load the property file ")
                    || err
                        .0
                        .contains("PropertSet::load: Cannot load the property file ");

                if is_prop_err {
                    self.index =
                        sys::ngt_create_graph_and_tree(self.prop.index_path.as_ptr(), prop, ebuf);
                    if self.index.is_null() {
                        Err(make_err(ebuf))?
                    }
                }
            } else {
                Err(make_err(ebuf))?
            }

            if !sys::ngt_get_property(self.index, prop, ebuf) {
                Err(make_err(ebuf))?
            }

            let object_type = sys::ngt_get_property_object_type(prop, ebuf);
            if object_type < 0 {
                Err(make_err(ebuf))?
            }
            let object_type =
                ObjectType::try_from(object_type).map_err(|e| Error(e.to_string()))?;
            self.prop.object_type = object_type;

            self.ospace = sys::ngt_get_object_space(self.index, ebuf);
            if self.ospace.is_null() {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    fn search(
        &self,
        vec: &[f64],
        size: usize,
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

    fn insert(&mut self, mut vec: Vec<f64>) -> Result<VecId, Error> {
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

    fn insert_commit(&mut self, vec: Vec<f64>, pool_size: u32) -> Result<VecId, Error> {
        let id = self.insert(vec)?;
        self.create_index(pool_size)?;
        self.save_index()?;
        Ok(id)
    }

    fn insert_commit_bulk(
        &mut self,
        vecs: Vec<Vec<f64>>,
        pool_size: u32,
    ) -> Result<Vec<VecId>, Error> {
        let mut ids = Vec::with_capacity(vecs.len());

        let mut idx = 0;
        for vec in vecs {
            let id = self.insert(vec)?;
            ids.push(id);
            idx += 1;
            if idx >= self.prop.bulk_insert_chunk_size {
                self.create_and_save_index(pool_size)?;
            }
        }

        self.create_and_save_index(pool_size)?;

        Ok(ids)
    }

    fn create_index(&mut self, pool_size: u32) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_create_index(self.index, pool_size, ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    fn create_and_save_index(&mut self, pool_size: u32) -> Result<(), Error> {
        self.create_index(pool_size)?;
        self.save_index()
    }

    fn save_index(&mut self) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_save_index(self.index, self.prop.index_path.as_ptr(), ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    fn remove(&mut self, id: VecId) -> Result<(), Error> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_remove_index(self.index, id, ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    fn get_vec(&self, id: VecId) -> Result<Vec<f32>, Error> {
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

    fn close(&mut self) {
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

        let prop = Property {
            index_path: CString::new(dir.path().to_string_lossy().as_bytes())?,
            dimension: 3,
            ..Default::default()
        };

        let mut ngt = Ngt::new(prop);
        ngt.open()?;

        let id1 = ngt.insert(vec![1.0, 2.0, 3.0])?;
        let id2 = ngt.insert(vec![4.0, 5.0, 6.0])?;

        ngt.create_index(4)?;
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
