use std::convert::TryFrom;
use std::ptr;

use ngt_sys as sys;
use num_enum::TryFromPrimitive;
use scopeguard::defer;

use crate::error::{make_err, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum NgtObject {
    Uint8 = 1,
    Float = 2,
    Float16 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum NgtDistance {
    L1 = 0,
    L2 = 1,
    Angle = 2,
    Hamming = 3,
    Cosine = 4,
    NormalizedAngle = 5,
    NormalizedCosine = 6,
    Jaccard = 7,
    SparseJaccard = 8,
    NormalizedL2 = 9,
    Poincare = 100,
    Lorentz = 101,
}

#[derive(Debug)]
pub struct NgtProperties {
    pub(crate) dimension: i32,
    pub(crate) creation_edge_size: i16,
    pub(crate) search_edge_size: i16,
    pub(crate) object_type: NgtObject,
    pub(crate) distance_type: NgtDistance,
    pub(crate) raw_prop: sys::NGTProperty,
}

unsafe impl Send for NgtProperties {}
unsafe impl Sync for NgtProperties {}

impl NgtProperties {
    pub fn dimension(dimension: usize) -> Result<Self> {
        let dimension = i32::try_from(dimension)?;
        let creation_edge_size = 10;
        let search_edge_size = 40;
        let object_type = NgtObject::Float;
        let distance_type = NgtDistance::L2;

        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let raw_prop = sys::ngt_create_property(ebuf);
            if raw_prop.is_null() {
                Err(make_err(ebuf))?
            }

            Self::set_dimension(raw_prop, dimension)?;
            Self::set_creation_edge_size(raw_prop, creation_edge_size)?;
            Self::set_search_edge_size(raw_prop, search_edge_size)?;
            Self::set_object_type(raw_prop, object_type)?;
            Self::set_distance_type(raw_prop, distance_type)?;

            Ok(Self {
                dimension,
                creation_edge_size,
                search_edge_size,
                object_type,
                distance_type,
                raw_prop,
            })
        }
    }

    pub fn try_clone(&self) -> Result<Self> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let raw_prop = sys::ngt_create_property(ebuf);
            if raw_prop.is_null() {
                Err(make_err(ebuf))?
            }

            Self::set_dimension(raw_prop, self.dimension)?;
            Self::set_creation_edge_size(raw_prop, self.creation_edge_size)?;
            Self::set_search_edge_size(raw_prop, self.search_edge_size)?;
            Self::set_object_type(raw_prop, self.object_type)?;
            Self::set_distance_type(raw_prop, self.distance_type)?;

            Ok(Self {
                dimension: self.dimension,
                creation_edge_size: self.creation_edge_size,
                search_edge_size: self.search_edge_size,
                object_type: self.object_type,
                distance_type: self.distance_type,
                raw_prop,
            })
        }
    }

    pub(crate) fn from(index: sys::NGTIndex) -> Result<Self> {
        unsafe {
            let ebuf = sys::ngt_create_error_object();
            defer! { sys::ngt_destroy_error_object(ebuf); }

            let raw_prop = sys::ngt_create_property(ebuf);
            if raw_prop.is_null() {
                Err(make_err(ebuf))?
            }

            if !sys::ngt_get_property(index, raw_prop, ebuf) {
                Err(make_err(ebuf))?
            }

            let dimension = sys::ngt_get_property_dimension(raw_prop, ebuf);
            if dimension < 0 {
                Err(make_err(ebuf))?
            }

            let creation_edge_size = sys::ngt_get_property_edge_size_for_creation(raw_prop, ebuf);
            if creation_edge_size < 0 {
                Err(make_err(ebuf))?
            }

            let search_edge_size = sys::ngt_get_property_edge_size_for_search(raw_prop, ebuf);
            if search_edge_size < 0 {
                Err(make_err(ebuf))?
            }

            let object_type = sys::ngt_get_property_object_type(raw_prop, ebuf);
            if object_type < 0 {
                Err(make_err(ebuf))?
            }
            let object_type = NgtObject::try_from(object_type)?;

            let distance_type = sys::ngt_get_property_distance_type(raw_prop, ebuf);
            if distance_type < 0 {
                Err(make_err(ebuf))?
            }
            let distance_type = NgtDistance::try_from(distance_type)?;

            Ok(Self {
                dimension,
                creation_edge_size,
                search_edge_size,
                object_type,
                distance_type,
                raw_prop,
            })
        }
    }

    unsafe fn set_dimension(raw_prop: sys::NGTProperty, dimension: i32) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_dimension(raw_prop, dimension, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn creation_edge_size(mut self, size: usize) -> Result<Self> {
        let size = i16::try_from(size)?;
        self.creation_edge_size = size;
        unsafe { Self::set_creation_edge_size(self.raw_prop, size)? };
        Ok(self)
    }

    unsafe fn set_creation_edge_size(raw_prop: sys::NGTProperty, size: i16) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_edge_size_for_creation(raw_prop, size, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn search_edge_size(mut self, size: usize) -> Result<Self> {
        let size = i16::try_from(size)?;
        self.search_edge_size = size;
        unsafe { Self::set_search_edge_size(self.raw_prop, size)? };
        Ok(self)
    }

    unsafe fn set_search_edge_size(raw_prop: sys::NGTProperty, size: i16) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_edge_size_for_search(raw_prop, size, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn object_type(mut self, object_type: NgtObject) -> Result<Self> {
        self.object_type = object_type;
        unsafe { Self::set_object_type(self.raw_prop, object_type)? };
        Ok(self)
    }

    unsafe fn set_object_type(raw_prop: sys::NGTProperty, object_type: NgtObject) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match object_type {
            NgtObject::Uint8 => {
                if !sys::ngt_set_property_object_type_integer(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtObject::Float => {
                if !sys::ngt_set_property_object_type_float(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtObject::Float16 => {
                if !sys::ngt_set_property_object_type_float16(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }

    pub fn distance_type(mut self, distance_type: NgtDistance) -> Result<Self> {
        self.distance_type = distance_type;
        unsafe { Self::set_distance_type(self.raw_prop, distance_type)? };
        Ok(self)
    }

    unsafe fn set_distance_type(
        raw_prop: sys::NGTProperty,
        distance_type: NgtDistance,
    ) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match distance_type {
            NgtDistance::L1 => {
                if !sys::ngt_set_property_distance_type_l1(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::L2 => {
                if !sys::ngt_set_property_distance_type_l2(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Angle => {
                if !sys::ngt_set_property_distance_type_angle(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Hamming => {
                if !sys::ngt_set_property_distance_type_hamming(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Cosine => {
                if !sys::ngt_set_property_distance_type_cosine(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::NormalizedAngle => {
                if !sys::ngt_set_property_distance_type_normalized_angle(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::NormalizedCosine => {
                if !sys::ngt_set_property_distance_type_normalized_cosine(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Jaccard => {
                if !sys::ngt_set_property_distance_type_jaccard(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::SparseJaccard => {
                if !sys::ngt_set_property_distance_type_sparse_jaccard(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::NormalizedL2 => {
                if !sys::ngt_set_property_distance_type_normalized_l2(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Poincare => {
                if !sys::ngt_set_property_distance_type_poincare(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            NgtDistance::Lorentz => {
                if !sys::ngt_set_property_distance_type_lorentz(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }
}

impl Drop for NgtProperties {
    fn drop(&mut self) {
        if !self.raw_prop.is_null() {
            unsafe { sys::ngt_destroy_property(self.raw_prop) };
            self.raw_prop = ptr::null_mut();
        }
    }
}
