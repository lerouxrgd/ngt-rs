use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Result};
use crate::index::Index;

/// Optimizes the number of initial edges of an ANNG index.
///
/// The default number of initial edges for each node in a default graph (ANNG) is a
/// fixed value 10. To optimize this number, follow these steps:
///   1. [`insert`](Index::insert) vectors in the ANNG index at `index_path`, don't
///   [`build`](Index::build) the index yet.
///   2. When all vectors are inserted, [`persist`](Index::persist) the index.
///   3. Call this function with the same `index_path`.
///   4. [`open`](Index::open) the index at `index_path` again, and now
///   [`build`](Index::build) it.
#[cfg(not(feature = "shared_mem"))]
pub fn optimize_anng_edges_number<P: AsRef<Path>>(
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

/// Optimizes the search parameters of an ANNG index.
///
/// Optimizes the search parameters about the explored edges and memory prefetch for the
/// existing indexes. Does not modify the index data structure.
pub fn optimize_anng_search_parameters<P: AsRef<Path>>(index_path: P) -> Result<()> {
    let mut optimizer = GraphOptimizer::new(GraphOptimParams::default())?;
    optimizer.set_processing_modes(true, true, true)?;
    optimizer.adjust_search_coefficients(index_path)?;
    Ok(())
}

/// Refines an ANNG index (RANNG) to improve search performance.
///
/// Improves accuracy of neighboring nodes for each node by searching with each
/// node. Note that refinement takes a long processing time. An ANNG index can be
/// refined only after it has been [`built`](Index::build).
#[cfg(not(feature = "shared_mem"))]
pub fn refine_anng(index: &mut Index, params: AnngRefineParams) -> Result<()> {
    unsafe {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_refine_anng(
            index.index,
            params.epsilon,
            params.expected_accuracy,
            params.nb_edges,
            params.edge_size,
            params.batch_size,
            ebuf,
        ) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }
}

/// Converts the `index_in` ANNG to an ONNG at `index_out`.
///
/// ONNG generation requires an ANNG with more edges than default initial edges as the
/// excess edges are removed to optimize the graph. ONNG requires an ANNG with at least
/// edges that have been optimized with
/// [`optimize_anng_edges_number`](optimize_anng_edges_number).
///
/// If more performance is needed, a larger `creation_edge_size` can be set through
/// [`Properties`](crate::Properties::creation_edge_size) at ANNG index
/// [`create`](Index::create) time.
///
/// Important [`GraphOptimParams`](GraphOptimParams) parameters are `nb_outgoing` edges
/// and `nb_incoming` edges. The latter can be set to an even higher number than the
/// `creation_edge_size` of the original ANNG.
pub fn convert_anng_to_onng<P: AsRef<Path>>(
    index_anng_in: P,
    index_onng_out: P,
    params: GraphOptimParams,
) -> Result<()> {
    let mut optimizer = GraphOptimizer::new(params)?;
    optimizer.convert_anng_to_onng(index_anng_in, index_onng_out)?;
    Ok(())
}

/// Parameters for [`optimize_anng_edges_number`](optimize_anng_edges_number).
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

#[cfg(not(feature = "shared_mem"))]
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

/// Parameters for [`refine_anng`](refine_anng).
#[derive(Debug, Clone, PartialEq)]
pub struct AnngRefineParams {
    epsilon: f32,
    expected_accuracy: f32,
    nb_edges: i32,
    edge_size: i32,
    batch_size: u64,
}

impl Default for AnngRefineParams {
    fn default() -> Self {
        Self {
            epsilon: 0.1,
            expected_accuracy: 0.0,
            nb_edges: 0,
            edge_size: i32::MIN,
            batch_size: 10000,
        }
    }
}

/// Parameters for [`convert_anng_to_onng`](convert_anng_to_onng).
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

struct GraphOptimizer(sys::NGTOptimizer);

impl GraphOptimizer {
    fn new(params: GraphOptimParams) -> Result<Self> {
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

    fn set_processing_modes(
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

    /// Optimize for the search parameters of an ANNG.
    fn adjust_search_coefficients<P: AsRef<Path>>(&mut self, index_path: P) -> Result<()> {
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

    /// Converts the `index_in` ANNG to an ONNG at `index_out`.
    fn convert_anng_to_onng<P: AsRef<Path>>(
        &mut self,
        index_anng_in: P,
        index_onng_out: P,
    ) -> Result<()> {
        let _ = Index::open(&index_anng_in)?;

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let index_in = CString::new(index_anng_in.as_ref().as_os_str().as_bytes())?;
            let index_out = CString::new(index_onng_out.as_ref().as_os_str().as_bytes())?;

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

    #[ignore]
    #[test]
    #[cfg(not(feature = "shared_mem"))]
    fn test_optimize_anng() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::dimension(3)?.distance_type(DistanceType::Cosine)?;
        let mut index = Index::create(dir.path(), prop)?;

        // Populate the index, but don't build it yet
        for i in 0..1_000_000 {
            let _ = index.insert(vec![i, i + 1, i + 2])?;
        }
        index.persist()?;

        // Optimize the persisted index
        optimize_anng_edges_number(dir.path(), AnngEdgeOptimParams::default())?;

        // Now build and persist again the optimized index
        let mut index = Index::open(dir.path())?;
        index.build(4)?;
        index.persist()?;

        // Further optimize the index
        optimize_anng_search_parameters(dir.path())?;

        dir.close()?;
        Ok(())
    }

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

        // Refine the index
        refine_anng(&mut index, AnngRefineParams::default())?;

        dir.close()?;
        Ok(())
    }

    #[ignore]
    #[test]
    #[cfg(not(feature = "shared_mem"))]
    fn test_convert_anng_to_onng() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the ANNG index
        let dir_in = tempdir()?;

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::dimension(3)?
            .distance_type(DistanceType::Cosine)?
            .creation_edge_size(100)?; // More than default value, improves the final ONNG

        let mut index = Index::create(dir_in.path(), prop)?;

        // Populate and persist (but don't build yet) the index
        for i in 0..1000 {
            let _ = index.insert(vec![i, i + 1, i + 2])?;
        }
        index.persist()?;

        // Optimize and build the index
        optimize_anng_edges_number(dir_in.path(), AnngEdgeOptimParams::default())?;

        // Now build and persist again the optimized index
        let mut index = Index::open(dir_in.path())?;
        index.build(4)?;
        index.persist()?;

        // Create an output directory for the ONNG index
        let dir_out = tempdir()?;
        std::fs::remove_dir(dir_out.path())?;

        // Convert the input ANNG to an ONNG
        let mut params = GraphOptimParams::default();
        params.nb_outgoing = 10;
        params.nb_incoming = 100; // An even larger number of incoming edges can be specified
        convert_anng_to_onng(dir_in.path(), dir_out.path(), params)?;

        dir_out.close()?;
        dir_in.close()?;
        Ok(())
    }
}
