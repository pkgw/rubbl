// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

General helpers for numerics.

*/

use errors::{ErrorKind, Result};
use ndarray::{IntoDimension, Ix0, Ix1, Ix2, Ix3, Ix4};


/// Adapt a slice representing an array shape into an `ndarray::Dimension` type.
///
/// In `ndarray` array dimensionalities are statically typed, but it is often
/// the case that we are reading arrays from data files where the array
/// dimensionality may not match the expectations of the compiled code. This
/// trait makes it possible to convert a runtime-flexible array shape into one
/// of the compile-time types … if the two dimensionalities are in fact the
/// same.
pub trait DimFromShapeSlice<T> : Sized {
    /// Try to create the implementing type from the specified array shape,
    /// specified as a slice.
    ///
    /// Returns an Err with an ErrorKind of DimensionMismatch if the slice
    /// size does not match the expected dimensionality.
    fn from_shape_slice(shape: &[T]) -> Result<Self>;
}

impl DimFromShapeSlice<u64> for Ix0 {
    fn from_shape_slice(shape: &[u64]) -> Result<Self> {
        if shape.len() == 0 {
            Ok([].into_dimension())
        } else {
            Err(ErrorKind::DimensionMismatch(0, shape.len()).into())
        }
    }
}

impl DimFromShapeSlice<u64> for Ix1 {
    fn from_shape_slice(shape: &[u64]) -> Result<Self> {
        if shape.len() == 1 {
            Ok([shape[0] as usize].into_dimension())
        } else {
            Err(ErrorKind::DimensionMismatch(1, shape.len()).into())
        }
    }
}

impl DimFromShapeSlice<u64> for Ix2 {
    fn from_shape_slice(shape: &[u64]) -> Result<Self> {
        if shape.len() == 2 {
            Ok([shape[0] as usize, shape[1] as usize].into_dimension())
        } else {
            Err(ErrorKind::DimensionMismatch(2, shape.len()).into())
        }
    }
}

impl DimFromShapeSlice<u64> for Ix3 {
    fn from_shape_slice(shape: &[u64]) -> Result<Self> {
        if shape.len() == 3 {
            Ok([shape[0] as usize, shape[1] as usize, shape[2] as usize].into_dimension())
        } else {
            Err(ErrorKind::DimensionMismatch(3, shape.len()).into())
        }
    }
}

impl DimFromShapeSlice<u64> for Ix4 {
    fn from_shape_slice(shape: &[u64]) -> Result<Self> {
        if shape.len() == 4 {
            Ok([shape[0] as usize, shape[1] as usize, shape[2] as usize, shape[3] as usize].into_dimension())
        } else {
            Err(ErrorKind::DimensionMismatch(4, shape.len()).into())
        }
    }
}
