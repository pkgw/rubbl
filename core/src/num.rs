// Copyright 2017-2020 Peter Williams
// Licensed under the MIT License.

//! General helpers for numerics.

use ndarray::{IntoDimension, Ix0, Ix1, Ix2, Ix3, Ix4, Ix5, Ix6};
use thiserror::Error;

/// An error type used when two arrays should have the same dimensionality,
/// but do not.
#[derive(Error, Debug)]
#[error("Expected a {expected}-dimensional array, but got one with {actual} dimensions")]
pub struct DimensionMismatchError {
    /// The number of dimensions that the array was expected to have.
    pub expected: usize,

    /// The number of dimensions that the array actually had.
    pub actual: usize,
}

/// Adapt a slice representing an array shape into an `ndarray::Dimension` type.
///
/// In `ndarray` array dimensionalities are statically typed, but it is often
/// the case that we are reading arrays from data files where the array
/// dimensionality may not match the expectations of the compiled code. This
/// trait makes it possible to convert a runtime-flexible array shape into one
/// of the compile-time types … if the two dimensionalities are in fact the
/// same.
pub trait DimFromShapeSlice<T>: Sized {
    /// Try to create the implementing type from the specified array shape,
    /// specified as a slice.
    ///
    /// Returns an Err with an ErrorKind of DimensionMismatch if the slice
    /// size does not match the expected dimensionality.
    fn from_shape_slice(shape: &[T]) -> Result<Self, DimensionMismatchError>;
}

macro_rules! impl_dim_from_shape_slice {
    ($dimtype:ty; $ndim:expr; $($numbers:expr);*) => {
        impl DimFromShapeSlice<u64> for $dimtype {
            fn from_shape_slice(shape: &[u64]) -> Result<Self, DimensionMismatchError> {
                if shape.len() == $ndim {
                    Ok([$(shape[$numbers] as usize),*].into_dimension())
                } else {
                    Err(DimensionMismatchError { expected: $ndim, actual: shape.len() })
                }
            }
        }

        impl DimFromShapeSlice<usize> for $dimtype {
            fn from_shape_slice(shape: &[usize]) -> Result<Self, DimensionMismatchError> {
                if shape.len() == $ndim {
                    Ok([$(shape[$numbers] as usize),*].into_dimension())
                } else {
                    Err(DimensionMismatchError { expected: $ndim, actual: shape.len() })
                }
            }
        }
    }
}

impl_dim_from_shape_slice! { Ix0; 0; }
impl_dim_from_shape_slice! { Ix1; 1; 0 }
impl_dim_from_shape_slice! { Ix2; 2; 0;1 }
impl_dim_from_shape_slice! { Ix3; 3; 0;1;2 }
impl_dim_from_shape_slice! { Ix4; 4; 0;1;2;3 }
impl_dim_from_shape_slice! { Ix5; 5; 0;1;2;3;4 }
impl_dim_from_shape_slice! { Ix6; 6; 0;1;2;3;4;5 }
