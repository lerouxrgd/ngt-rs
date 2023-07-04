use std::marker::PhantomData;
use std::ptr;

use half::f16;
use ngt_sys as sys;
use num_enum::TryFromPrimitive;
use scopeguard::defer;

use crate::error::{make_err, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum QgObject {
    Uint8 = 1,
    Float = 2,
    Float16 = 3,
}

mod private {
    pub trait Sealed {}
}

pub trait QgObjectType: private::Sealed {
    fn as_obj() -> QgObject;
}

impl private::Sealed for f32 {}
impl QgObjectType for f32 {
    fn as_obj() -> QgObject {
        QgObject::Float
    }
}

impl private::Sealed for u8 {}
impl QgObjectType for u8 {
    fn as_obj() -> QgObject {
        QgObject::Uint8
    }
}

impl private::Sealed for f16 {}
impl QgObjectType for f16 {
    fn as_obj() -> QgObject {
        QgObject::Float16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(i32)]
pub enum QgDistance {
    L2 = 1,
    Cosine = 4,
}

#[derive(Debug)]
pub struct QgProperties<T> {
    pub(crate) dimension: i32,
    pub(crate) creation_edge_size: i16,
    pub(crate) search_edge_size: i16,
    pub(crate) object_type: QgObject,
    pub(crate) distance_type: QgDistance,
    pub(crate) raw_prop: sys::NGTProperty,
    _marker: PhantomData<T>,
}

unsafe impl<T> Send for QgProperties<T> {}
unsafe impl<T> Sync for QgProperties<T> {}

impl<T> QgProperties<T>
where
    T: QgObjectType,
{
    pub fn dimension(dimension: usize) -> Result<Self> {
        let dimension = i32::try_from(dimension)?;
        let creation_edge_size = 10;
        let search_edge_size = 40;
        let object_type = T::as_obj();
        let distance_type = QgDistance::L2;

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
                _marker: PhantomData,
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
                _marker: PhantomData,
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
            let object_type = QgObject::try_from(object_type)?;

            let distance_type = sys::ngt_get_property_distance_type(raw_prop, ebuf);
            if distance_type < 0 {
                Err(make_err(ebuf))?
            }
            let distance_type = QgDistance::try_from(distance_type)?;

            Ok(Self {
                dimension,
                creation_edge_size,
                search_edge_size,
                object_type,
                distance_type,
                raw_prop,
                _marker: PhantomData,
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

    unsafe fn set_object_type(raw_prop: sys::NGTProperty, object_type: QgObject) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match object_type {
            QgObject::Uint8 => {
                if !sys::ngt_set_property_object_type_integer(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            QgObject::Float => {
                if !sys::ngt_set_property_object_type_float(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            QgObject::Float16 => {
                if !sys::ngt_set_property_object_type_float16(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }

    pub fn distance_type(mut self, distance_type: QgDistance) -> Result<Self> {
        self.distance_type = distance_type;
        unsafe { Self::set_distance_type(self.raw_prop, distance_type)? };
        Ok(self)
    }

    unsafe fn set_distance_type(
        raw_prop: sys::NGTProperty,
        distance_type: QgDistance,
    ) -> Result<()> {
        let ebuf = sys::ngt_create_error_object();
        defer! { sys::ngt_destroy_error_object(ebuf); }

        match distance_type {
            QgDistance::L2 => {
                if !sys::ngt_set_property_distance_type_l2(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
            QgDistance::Cosine => {
                if !sys::ngt_set_property_distance_type_cosine(raw_prop, ebuf) {
                    Err(make_err(ebuf))?
                }
            }
        }

        Ok(())
    }
}

impl<T> Drop for QgProperties<T> {
    fn drop(&mut self) {
        if !self.raw_prop.is_null() {
            unsafe { sys::ngt_destroy_property(self.raw_prop) };
            self.raw_prop = ptr::null_mut();
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct QgQuantizationParams {
    pub dimension_of_subvector: f32,
    pub max_number_of_edges: u64,
}

impl Default for QgQuantizationParams {
    fn default() -> Self {
        Self {
            dimension_of_subvector: 0.0,
            max_number_of_edges: 128,
        }
    }
}

impl QgQuantizationParams {
    pub(crate) fn into_raw(self) -> sys::NGTQGQuantizationParameters {
        sys::NGTQGQuantizationParameters {
            dimension_of_subvector: self.dimension_of_subvector,
            max_number_of_edges: self.max_number_of_edges,
        }
    }
}
