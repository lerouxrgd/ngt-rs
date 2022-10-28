use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{mem, ptr};

use ngt_sys as sys;
use num_enum::TryFromPrimitive;
use scopeguard::defer;

use super::{QbgDistance, QbgObject};
use crate::error::{make_err, Error, Result};
use crate::{SearchResult, VecId};

#[derive(Debug)]
pub struct QbgIndex<T> {
    pub(crate) index: sys::QBGIndex,
    path: CString,
    _mode: T,
    dimension: u32,
    ebuf: sys::NGTError,
}

impl QbgIndex<ModeWrite> {
    pub fn create<P>(path: P, create_params: QbgConstructParams) -> Result<Self>
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
            })
        }
    }

    pub fn insert(&mut self, mut vec: Vec<f32>) -> Result<VecId> {
        unsafe {
            let id =
                sys::qbg_append_object(self.index, vec.as_mut_ptr(), self.dimension, self.ebuf);
            if id == 0 {
                Err(make_err(self.ebuf))?
            }

            Ok(id)
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

    pub fn into_readable(self) -> Result<QbgIndex<ModeRead>> {
        let path = self.path.clone();
        drop(self);
        QbgIndex::open(path.into_string()?)
    }
}

impl QbgIndex<ModeRead> {
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
            })
        }
    }

    pub fn search(&self, query: QbgQuery) -> Result<Vec<SearchResult>> {
        unsafe {
            let results = sys::ngt_create_empty_results(self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }
            defer! { sys::qbg_destroy_results(results); }

            if !sys::qbg_search_index(self.index, query.into_raw(), results, self.ebuf) {
                Err(make_err(self.ebuf))?
            }

            let rsize = sys::qbg_get_result_size(results, self.ebuf);
            let mut ret = Vec::with_capacity(rsize as usize);

            for i in 0..rsize as u32 {
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

    pub fn into_writable(self) -> Result<QbgIndex<ModeWrite>> {
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
            })
        }
    }
}

impl<T> QbgIndex<T>
where
    T: IndexMode,
{
    pub fn get_vec(&self, id: VecId) -> Result<Vec<f32>> {
        unsafe {
            let results = sys::qbg_get_object(self.index, id, self.ebuf);
            if results.is_null() {
                Err(make_err(self.ebuf))?
            }

            let results = Vec::from_raw_parts(
                results as *mut f32,
                self.dimension as usize,
                self.dimension as usize,
            );
            let results = mem::ManuallyDrop::new(results);
            let results = results.iter().copied().collect::<Vec<_>>();

            Ok(results)
        }
    }
}

impl<T> Drop for QbgIndex<T> {
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
pub struct QbgConstructParams {
    extended_dimension: u64,
    dimension: u64,
    number_of_subvectors: u64,
    number_of_blobs: u64,
    internal_data_type: QbgObject,
    data_type: QbgObject,
    distance_type: QbgDistance,
}

impl Default for QbgConstructParams {
    fn default() -> Self {
        Self {
            extended_dimension: 0,
            dimension: 0,
            number_of_subvectors: 1,
            number_of_blobs: 0,
            internal_data_type: QbgObject::Float,
            data_type: QbgObject::Float,
            distance_type: QbgDistance::L2,
        }
    }
}

impl QbgConstructParams {
    pub fn extended_dimension(mut self, extended_dimension: u64) -> Self {
        self.extended_dimension = extended_dimension;
        self
    }

    pub fn dimension(mut self, dimension: u64) -> Self {
        self.dimension = dimension;
        self
    }

    pub fn number_of_subvectors(mut self, number_of_subvectors: u64) -> Self {
        self.number_of_subvectors = number_of_subvectors;
        self
    }

    pub fn number_of_blobs(mut self, number_of_blobs: u64) -> Self {
        self.number_of_blobs = number_of_blobs;
        self
    }

    pub fn internal_data_type(mut self, internal_data_type: QbgObject) -> Self {
        self.internal_data_type = internal_data_type;
        self
    }

    pub fn data_type(mut self, data_type: QbgObject) -> Self {
        self.data_type = data_type;
        self
    }

    pub fn distance_type(mut self, distance_type: QbgDistance) -> Self {
        self.distance_type = distance_type;
        self
    }

    unsafe fn into_raw(self) -> sys::QBGConstructionParameters {
        sys::QBGConstructionParameters {
            extended_dimension: self.extended_dimension,
            dimension: self.dimension,
            number_of_subvectors: self.number_of_subvectors,
            number_of_blobs: self.number_of_blobs,
            internal_data_type: self.internal_data_type as i32,
            data_type: self.data_type as i32,
            distance_type: self.distance_type as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum QbgClusteringInitMode {
    Head = 0,
    Random = 1,
    KmeansPlusPlus = 2,
    RandomFixedSeed = 3,
    KmeansPlusPlusFixedSeed = 4,
    Best = 5,
}

#[derive(Debug, Clone)]
pub struct QbgBuildParams {
    // hierarchical kmeans
    hierarchical_clustering_init_mode: QbgClusteringInitMode,
    number_of_first_objects: u64,
    number_of_first_clusters: u64,
    number_of_second_objects: u64,
    number_of_second_clusters: u64,
    number_of_third_clusters: u64,
    // optimization
    number_of_objects: u64,
    number_of_subvectors: u64,
    optimization_clustering_init_mode: QbgClusteringInitMode,
    rotation_iteration: u64,
    subvector_iteration: u64,
    number_of_matrices: u64,
    rotation: bool,
    repositioning: bool,
}

impl Default for QbgBuildParams {
    fn default() -> Self {
        Self {
            hierarchical_clustering_init_mode: QbgClusteringInitMode::KmeansPlusPlus,
            number_of_first_objects: 0,
            number_of_first_clusters: 0,
            number_of_second_objects: 0,
            number_of_second_clusters: 0,
            number_of_third_clusters: 0,
            number_of_objects: 1000,
            number_of_subvectors: 1,
            optimization_clustering_init_mode: QbgClusteringInitMode::KmeansPlusPlus,
            rotation_iteration: 2000,
            subvector_iteration: 400,
            number_of_matrices: 3,
            rotation: true,
            repositioning: false,
        }
    }
}

impl QbgBuildParams {
    pub fn hierarchical_clustering_init_mode(
        mut self,
        clustering_init_mode: QbgClusteringInitMode,
    ) -> Self {
        self.hierarchical_clustering_init_mode = clustering_init_mode;
        self
    }

    pub fn number_of_first_objects(mut self, number_of_first_objects: u64) -> Self {
        self.number_of_first_objects = number_of_first_objects;
        self
    }

    pub fn number_of_first_clusters(mut self, number_of_first_clusters: u64) -> Self {
        self.number_of_first_clusters = number_of_first_clusters;
        self
    }

    pub fn number_of_second_objects(mut self, number_of_second_objects: u64) -> Self {
        self.number_of_second_objects = number_of_second_objects;
        self
    }

    pub fn number_of_second_clusters(mut self, number_of_second_clusters: u64) -> Self {
        self.number_of_second_clusters = number_of_second_clusters;
        self
    }

    pub fn number_of_third_clusters(mut self, number_of_third_clusters: u64) -> Self {
        self.number_of_third_clusters = number_of_third_clusters;
        self
    }

    pub fn number_of_objects(mut self, number_of_objects: u64) -> Self {
        self.number_of_objects = number_of_objects;
        self
    }
    pub fn number_of_subvectors(mut self, number_of_subvectors: u64) -> Self {
        self.number_of_subvectors = number_of_subvectors;
        self
    }
    pub fn optimization_clustering_init_mode(
        mut self,
        clustering_init_mode: QbgClusteringInitMode,
    ) -> Self {
        self.optimization_clustering_init_mode = clustering_init_mode;
        self
    }

    pub fn rotation_iteration(mut self, rotation_iteration: u64) -> Self {
        self.rotation_iteration = rotation_iteration;
        self
    }

    pub fn subvector_iteration(mut self, subvector_iteration: u64) -> Self {
        self.subvector_iteration = subvector_iteration;
        self
    }

    pub fn number_of_matrices(mut self, number_of_matrices: u64) -> Self {
        self.number_of_matrices = number_of_matrices;
        self
    }

    pub fn rotation(mut self, rotation: bool) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn repositioning(mut self, repositioning: bool) -> Self {
        self.repositioning = repositioning;
        self
    }

    unsafe fn into_raw(self) -> sys::QBGBuildParameters {
        sys::QBGBuildParameters {
            hierarchical_clustering_init_mode: self.hierarchical_clustering_init_mode as i32,
            number_of_first_objects: self.number_of_first_objects,
            number_of_first_clusters: self.number_of_first_clusters,
            number_of_second_objects: self.number_of_second_objects,
            number_of_second_clusters: self.number_of_second_clusters,
            number_of_third_clusters: self.number_of_third_clusters,
            number_of_objects: self.number_of_objects,
            number_of_subvectors: self.number_of_subvectors,
            optimization_clustering_init_mode: self.optimization_clustering_init_mode as i32,
            rotation_iteration: self.rotation_iteration,
            subvector_iteration: self.subvector_iteration,
            number_of_matrices: self.number_of_matrices,
            rotation: self.rotation,
            repositioning: self.repositioning,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QbgQuery<'a> {
    query: &'a [f32],
    pub size: u64,
    pub epsilon: f32,
    pub blob_epsilon: f32,
    pub result_expansion: f32,
    pub number_of_explored_blobs: u64,
    pub number_of_edges: u64,
    pub radius: f32,
}

impl<'a> QbgQuery<'a> {
    pub fn new(query: &'a [f32]) -> Self {
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

    unsafe fn into_raw(self) -> sys::QBGQuery {
        sys::QBGQuery {
            query: self.query.as_ptr() as *mut f32,
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
    fn test_qbg() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;
        std::fs::remove_dir(dir.path())?;

        // Create a QGB index
        let ndims = 3;
        let mut index =
            QbgIndex::create(dir.path(), QbgConstructParams::default().dimension(ndims))?;

        // Insert vectors and get their ids
        let nvecs = 16;
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
}
