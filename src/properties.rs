use std::convert::TryFrom;
use std::ptr;

use ngt_sys as sys;
use scopeguard::defer;

use crate::{make_err, Error};

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum ObjectType {
    Uint8 = 1,
    Float = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum DistanceType {
    L1,
    L2,
    Angle,
    Hamming,
    Cosine,
    NormalizedAngle,
    NormalizedCosine,
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
    pub fn new(dimension: usize) -> Result<Self, Error> {
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

    unsafe fn set_dimension(raw_prop: sys::NGTProperty, dimension: i32) -> Result<(), Error> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_dimension(raw_prop, dimension, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn creation_edge_size(mut self, size: usize) -> Result<Self, Error> {
        let size = i16::try_from(size)?;
        self.creation_edge_size = size;
        unsafe { Self::set_creation_edge_size(self.raw_prop, size)? };
        Ok(self)
    }

    unsafe fn set_creation_edge_size(raw_prop: sys::NGTProperty, size: i16) -> Result<(), Error> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_edge_size_for_creation(raw_prop, size, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn search_edge_size(mut self, size: usize) -> Result<Self, Error> {
        let size = i16::try_from(size)?;
        self.search_edge_size = size;
        unsafe { Self::set_search_edge_size(self.raw_prop, size)? };
        Ok(self)
    }

    unsafe fn set_search_edge_size(raw_prop: sys::NGTProperty, size: i16) -> Result<(), Error> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        if !sys::ngt_set_property_edge_size_for_search(raw_prop, size, ebuf) {
            Err(make_err(ebuf))?
        }

        Ok(())
    }

    pub fn object_type(mut self, object_type: ObjectType) -> Result<Self, Error> {
        self.object_type = object_type;
        unsafe { Self::set_object_type(self.raw_prop, object_type)? };
        Ok(self)
    }

    unsafe fn set_object_type(
        raw_prop: sys::NGTProperty,
        object_type: ObjectType,
    ) -> Result<(), Error> {
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

    pub fn distance_type(mut self, distance_type: DistanceType) -> Result<Self, Error> {
        self.distance_type = distance_type;
        unsafe { Self::set_distance_type(self.raw_prop, distance_type)? };
        Ok(self)
    }

    unsafe fn set_distance_type(
        raw_prop: sys::NGTProperty,
        distance_type: DistanceType,
    ) -> Result<(), Error> {
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
