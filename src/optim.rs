use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Error, Result};
use crate::index::Index;

#[cfg(not(shared_mem))]
pub fn refine_anng(
    index: &mut Index,
    epsilon: f32,
    expected_accuracy: f32,
    nb_edges: i32,
    edge_size: i32,
    batch_size: u64,
) -> Result<()> {
    if !index.is_built {
        Err(Error("Cannot refine an unbuilt index".into()))?
    }

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
pub struct GraphOptimParams {
    pub outgoing: i32,
    pub incoming: i32,
    pub queries: i32,
    pub low_accuracy_from: f32,
    pub low_accuracy_to: f32,
    pub high_accuracy_from: f32,
    pub high_accuracy_to: f32,
    pub gt_epsilon: f64,
    pub merge: f64,
}

impl Default for GraphOptimParams {
    fn default() -> Self {
        Self {
            outgoing: 10,
            incoming: 120,
            queries: 100,
            low_accuracy_from: 0.3,
            low_accuracy_to: 0.5,
            high_accuracy_from: 0.8,
            high_accuracy_to: 0.9,
            gt_epsilon: 0.1,
            merge: 0.2,
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
                params.outgoing,
                params.incoming,
                params.queries,
                params.low_accuracy_from,
                params.low_accuracy_to,
                params.high_accuracy_from,
                params.high_accuracy_to,
                params.gt_epsilon,
                params.merge,
                ebuf,
            ) {
                Err(make_err(ebuf))?
            }

            Ok(Self(optim))
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
    #[cfg(not(shared_mem))]
    fn test_refine_anng() -> StdResult<(), Box<dyn StdError>> {
        // Get a temporary directory to store the index
        let dir = tempdir()?;

        // Create an index for vectors of dimension 3 with cosine distance
        let prop = Properties::new(3)?.distance_type(DistanceType::Cosine)?;
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
}
