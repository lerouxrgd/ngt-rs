use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::error::{make_err, Result};
use crate::index::Index;

#[derive(Debug, Clone)]
pub struct OptimParams {
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

impl Default for OptimParams {
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

pub struct Optimizer(sys::NGTOptimizer);

impl Optimizer {
    pub fn new(params: OptimParams) -> Result<Self> {
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

impl Drop for Optimizer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { sys::ngt_destroy_optimizer(self.0) };
            self.0 = ptr::null_mut();
        }
    }
}
