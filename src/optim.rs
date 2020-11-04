use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Result};
use crate::index::Index;

#[cfg(not(feature = "shared_mem"))]
pub fn refine_anng(
    index: &mut Index,
    epsilon: f32,
    expected_accuracy: f32,
    nb_edges: i32,
    edge_size: i32,
    batch_size: u64,
) -> Result<()> {
    unsafe {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_refine_anng(
            index.index,
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

#[derive(Debug, Clone, PartialEq)]
pub struct AnngEdgeOptimParams {
    pub nb_queries: u64,
    pub nb_results: u64,
    pub nb_threads: u64,
    pub target_accuracy: f32,
    pub target_nb_objects: u64,
    pub nb_sample_objects: u64,
    pub nb_edges_max: u64,
    pub log: bool,
}

impl Default for AnngEdgeOptimParams {
    fn default() -> Self {
        Self {
            nb_queries: 200,
            nb_results: 50,
            nb_threads: 16,
            target_accuracy: 0.9,
            target_nb_objects: 0,
            nb_sample_objects: 100_000,
            nb_edges_max: 100,
            log: false,
        }
    }
}

impl AnngEdgeOptimParams {
    unsafe fn into_raw(self) -> sys::NGTAnngEdgeOptimizationParameter {
        let mut params = sys::ngt_get_anng_edge_optimization_parameter();
        params.no_of_queries = self.nb_queries;
        params.no_of_results = self.nb_results;
        params.no_of_threads = self.nb_threads;
        params.target_accuracy = self.target_accuracy;
        params.target_no_of_objects = self.target_nb_objects;
        params.no_of_sample_objects = self.nb_sample_objects;
        params.max_of_no_of_edges = self.nb_edges_max;
        params
    }
}

#[cfg(not(feature = "shared_mem"))]
pub fn optimize_edges_number<P: AsRef<Path>>(
    index_path: P,
    params: AnngEdgeOptimParams,
) -> Result<()> {
    unsafe {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        let index_path = CString::new(index_path.as_ref().as_os_str().as_bytes())?;

        if !sys::ngt_optimize_number_of_edges(index_path.as_ptr(), params.into_raw(), ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphOptimParams {
    pub nb_outgoing: i32,
    pub nb_incoming: i32,
    pub nb_queries: i32,
    pub low_accuracy_from: f32,
    pub low_accuracy_to: f32,
    pub high_accuracy_from: f32,
    pub high_accuracy_to: f32,
    pub gt_epsilon: f64,
    pub margin: f64,
}

impl Default for GraphOptimParams {
    fn default() -> Self {
        Self {
            nb_outgoing: 10,
            nb_incoming: 120,
            nb_queries: 100,
            low_accuracy_from: 0.3,
            low_accuracy_to: 0.5,
            high_accuracy_from: 0.8,
            high_accuracy_to: 0.9,
            gt_epsilon: 0.1,
            margin: 0.2,
        }
    }
}

pub struct GraphOptimizer(sys::NGTOptimizer);

impl GraphOptimizer {
    pub fn new(params: GraphOptimParams) -> Result<Self> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let optim = sys::ngt_create_optimizer(true, ebuf);
            if optim.is_null() {
                Err(make_err(ebuf))?
            }

            if !sys::ngt_optimizer_set(
                optim,
                params.nb_outgoing,
                params.nb_incoming,
                params.nb_queries,
                params.low_accuracy_from,
                params.low_accuracy_to,
                params.high_accuracy_from,
                params.high_accuracy_to,
                params.gt_epsilon,
                params.margin,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            Ok(Self(optim))
        }
    }

    pub fn set_processing_modes(
        &mut self,
        search_param: bool,
        prefetch_param: bool,
        accuracy_table: bool,
    ) -> Result<()> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            if !sys::ngt_optimizer_set_processing_modes(
                self.0,
                search_param,
                prefetch_param,
                accuracy_table,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn adjust_search_coefficients<P: AsRef<Path>>(&mut self, index_path: P) -> Result<()> {
        let _ = Index::open(&index_path)?;

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let index_path = CString::new(index_path.as_ref().as_os_str().as_bytes())?;

            if !sys::ngt_optimizer_adjust_search_coefficients(self.0, index_path.as_ptr(), ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }

    pub fn execute<P: AsRef<Path>>(&mut self, index_in: P, index_out: P) -> Result<()> {
        let _ = Index::open(&index_in)?;

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let index_in = CString::new(index_in.as_ref().as_os_str().as_bytes())?;
            let index_out = CString::new(index_out.as_ref().as_os_str().as_bytes())?;

            if !sys::ngt_optimizer_execute(self.0, index_in.as_ptr(), index_out.as_ptr(), ebuf) {
                Err(make_err(ebuf))?
            }

            Ok(())
        }
    }
}

impl Drop for GraphOptimizer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { sys::ngt_destroy_optimizer(self.0) };
            self.0 = ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::result::Result as StdResult;

    use tempfile::tempdir;

    use crate::{DistanceType, Index, Properties};

    use super::*;

    #[test]
    #[cfg(not(feature = "shared_mem"))]
    fn test_refine_anng() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::dimension(3)?.distance_type(DistanceType::Cosine)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Populate and build the index
        for i in 0..1000 {
            let _ = index.insert(vec![i, i + 1, i + 2])?;
        }
        index.build(4)?;

        // Optimize the index
        refine_anng(&mut index, 0.1, 0.0, 0, i32::MIN, 10000)?;

        dir.close()?;
        Ok(())
    }

    #[ignore]
    #[test]
    fn test_optimize_edges_number() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::dimension(3)?.distance_type(DistanceType::Cosine)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Populate and build the index
        for i in 0..100_000 {
            let _ = index.insert(vec![i, i + 1, i + 2])?;
        }
        index.build(4)?;

        // Optimize the index
        optimize_edges_number(dir.path(), AnngEdgeOptimParams::default())?;

        dir.close()?;
        Ok(())
    }

    #[ignore]
    #[test]
    fn test_graph_optimizer() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir_in = tempdir()?;
        if cfg!(feature = "shared_mem") {
            std::fs::remove_dir(dir_in.path())?;
        }

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::dimension(3)?.distance_type(DistanceType::Cosine)?;
        let mut index = Index::create(dir_in.path(), prop)?;

        // Populate, build, and persist the index
        for i in 0..1000 {
            let _ = index.insert(vec![i, i + 1, i + 2])?;
        }
        index.build(2)?;
        index.persist()?;

        // Close the index (by dropping it)
        drop(index);

        // Create a index optimizer
        let mut optimizer = GraphOptimizer::new(GraphOptimParams::default())?;
        optimizer.adjust_search_coefficients(dir_in.path())?;

        // Create an output directory for the optimized index
        let dir_out = tempdir()?;
        std::fs::remove_dir(dir_out.path())?;

        // Convert the input ANNG to an ONNG
        optimizer.execute(dir_in.path(), dir_out.path())?;

        dir_out.close()?;
        dir_in.close()?;
        Ok(())
    }
}
