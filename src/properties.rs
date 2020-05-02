use std::convert::TryFrom;
use std::ptr;

use ngt_sys as sys;
use num_enum::TryFromPrimitive;
use scopeguard::defer;

use crate::error::{make_err, Result};

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum ObjectType {
    Uint8 = 1,
    Float = 2,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum DistanceType {
    L1 = 1,
    L2 = 2,
    Angle = 3,
    Hamming = 4,
    Cosine = 5,
    NormalizedAngle = 6,
    NormalizedCosine = 7,
}

#[derive(Debug, Clone)]
pub struct Properties {
    pub(crate) dimension: i32,
    pub(crate) creation_edge_size: i16,
    pub(crate) search_edge_size: i16,
    pub(crate) object_type: ObjectType,
    pub(crate) distance_type: DistanceType,
    pub(crate) raw_prop: sys::NGTProperty,
}

impl Properties {
    pub fn new(dimension: usize) -> Result<Self> {
        let dimension = i32::try_from(dimension)?;
        let creation_edge_size = 10;
        let search_edge_size = 40;
        let object_type = ObjectType::Float;
        let distance_type = DistanceType::L2;

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
            let object_type = ObjectType::try_from(object_type)?;

            let distance_type = sys::ngt_get_property_distance_type(raw_prop, ebuf);
            if distance_type < 0 {
                Err(make_err(ebuf))?
            }
            let distance_type = DistanceType::try_from(distance_type)?;

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

    pub fn object_type(mut self, object_type: ObjectType) -> Result<Self> {
        self.object_type = object_type;
        unsafe { Self::set_object_type(self.raw_prop, object_type)? };
        Ok(self)
    }

    unsafe fn set_object_type(raw_prop: sys::NGTProperty, object_type: ObjectType) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match object_type {
            ObjectType::Uint8 => {
                if !sys::ngt_set_property_object_type_integer(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            ObjectType::Float => {
                if !sys::ngt_set_property_object_type_float(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }

    pub fn distance_type(mut self, distance_type: DistanceType) -> Result<Self> {
        self.distance_type = distance_type;
        unsafe { Self::set_distance_type(self.raw_prop, distance_type)? };
        Ok(self)
    }

    unsafe fn set_distance_type(
        raw_prop: sys::NGTProperty,
        distance_type: DistanceType,
    ) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match distance_type {
            DistanceType::L1 => {
                if !sys::ngt_set_property_distance_type_l1(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::L2 => {
                if !sys::ngt_set_property_distance_type_l2(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::Angle => {
                if !sys::ngt_set_property_distance_type_angle(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::Hamming => {
                if !sys::ngt_set_property_distance_type_hamming(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::Cosine => {
                if !sys::ngt_set_property_distance_type_cosine(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::NormalizedAngle => {
                if !sys::ngt_set_property_distance_type_normalized_angle(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            DistanceType::NormalizedCosine => {
                if !sys::ngt_set_property_distance_type_normalized_cosine(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }
}

impl Drop for Properties {
    fn drop(&mut self) {
        if !self.raw_prop.is_null() {
            unsafe { sys::ngt_destroy_property(self.raw_prop) };
            self.raw_prop = ptr::null_mut();
        }
    }
}
