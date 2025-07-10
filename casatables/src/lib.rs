// Copyright 2017-2021 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

//! I/O on [CASA] table data sets.
//!
//! [CASA]: https://casa.nrao.edu/
//!
//! This crate provides I/O access to the CASA table data format. This format is
//! commonly used for storing radio astronomical visibility data in the
//! [Measurement Set][MS] data model, but is not limited to that particular
//! application — it is a generic data format for tabular scientific data. This
//! crate provides only lower-level I/O interfaces and not higher-level
//! abstractions for dealing with the specific semantics of Measurement Set
//! data.
//!
//! [MS]: https://casa.nrao.edu/Memos/229.html
//!
//! Because the on-disk representation of the CASA table format is quite complex
//! and essentially undocumented, this crate’s implementation relies on wrapping
//! a substantial quantity of C++ code from the [casacore] project. The goal is
//! to provide access to the data format in a way that is completely safe and as
//! idiomatic as possible, given the limitations imposed by the architecture of
//! the underlying C++ code.
//!
//! [casacore]: http://casacore.github.io/casacore/
//!
//! The entry point to this crate is typically the [`Table`] struct that
//! represents a handle to a CASA table data set.

#![deny(missing_docs)]

use ndarray::{ArrayBase, Dimension};
use rubbl_core::num::{DimFromShapeSlice, DimensionMismatchError};
use std::{
    fmt::{self, Debug},
    mem::MaybeUninit as StdMaybeUninit,
    path::Path,
};
use thiserror::Error;

pub use rubbl_core::{Array, Complex, CowArray};

#[allow(missing_docs)]
mod glue;
pub use glue::{GlueDataType, TableDescCreateMode};

// Exceptions

/// An error type used when the wrapped "casacore" C++ code raises an
/// exception.
#[derive(Error, Debug)]
#[error("{0}")]
pub struct CasacoreError(String);

impl glue::ExcInfo {
    fn as_error(&self) -> CasacoreError {
        let c_str = unsafe { std::ffi::CStr::from_ptr(self.message.as_ptr()) };

        let msg = c_str.to_str().unwrap_or("[un-translatable C++ exception]");

        CasacoreError(msg.to_owned())
    }

    fn as_err<T, E>(&self) -> Result<T, E>
    where
        E: From<CasacoreError>,
    {
        Err(self.as_error().into())
    }
}

// Data types

impl glue::GlueDataType {
    /// Return the number of bytes per element of this data type.
    ///
    /// Returns -1 for types that do not have fixed sizes, which includes
    /// strings. `TpX` and `TpArrayX` both return the same value.
    pub fn element_size(&self) -> i32 {
        unsafe { glue::data_type_get_element_size(*self) as i32 }
    }
}

impl fmt::Display for glue::GlueDataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(match *self {
            glue::GlueDataType::TpBool => "bool",
            glue::GlueDataType::TpChar => "i8",
            glue::GlueDataType::TpUChar => "u8",
            glue::GlueDataType::TpShort => "i16",
            glue::GlueDataType::TpUShort => "u16",
            glue::GlueDataType::TpInt => "i32",
            glue::GlueDataType::TpUInt => "u32",
            glue::GlueDataType::TpFloat => "f32",
            glue::GlueDataType::TpDouble => "f64",
            glue::GlueDataType::TpComplex => "c32",
            glue::GlueDataType::TpDComplex => "c64",
            glue::GlueDataType::TpString => "string",
            glue::GlueDataType::TpTable => "table",
            glue::GlueDataType::TpArrayBool => "arr<bool>",
            glue::GlueDataType::TpArrayChar => "arr<i8>",
            glue::GlueDataType::TpArrayUChar => "arr<u8>",
            glue::GlueDataType::TpArrayShort => "arr<i16>",
            glue::GlueDataType::TpArrayUShort => "arr<u16>",
            glue::GlueDataType::TpArrayInt => "arr<i32>",
            glue::GlueDataType::TpArrayUInt => "arr<u32>",
            glue::GlueDataType::TpArrayFloat => "arr<f32>",
            glue::GlueDataType::TpArrayDouble => "arr<f64>",
            glue::GlueDataType::TpArrayComplex => "arr<c32>",
            glue::GlueDataType::TpArrayDComplex => "arr<c64>",
            glue::GlueDataType::TpArrayString => "arr<string>",
            glue::GlueDataType::TpRecord => "record",
            glue::GlueDataType::TpOther => "other",
            glue::GlueDataType::TpQuantity => "quantity",
            glue::GlueDataType::TpArrayQuantity => "arr<quantity>",
            glue::GlueDataType::TpInt64 => "i64",
            glue::GlueDataType::TpArrayInt64 => "arr<i64>",
        })
    }
}

/// A type that can be translated into a CASA table data type.
///
/// You should never need to implement this trait yourself, because this crate
/// provides definitions for all types supported by the underlying CASA table
/// libraries.
pub trait CasaDataType: Clone + PartialEq + Sized {
    /// The CASA data type identifier associated with this Rust type.
    const DATA_TYPE: glue::GlueDataType;

    #[cfg(test)]
    /// Testing assertion.
    fn test_casa_data_size() {
        assert_eq!(
            std::mem::size_of::<Self>() as i32,
            Self::DATA_TYPE.element_size()
        );
    }

    /// A hack that lets us properly special-case strings as scalar types.
    #[doc(hidden)]
    fn casatables_string_pass_through(_s: String) -> Self {
        unreachable!();
    }

    /// A hack that lets us properly special-case strings as scalar types.
    #[doc(hidden)]
    fn casatables_string_pass_through_out(_s: &Self) -> String {
        unreachable!();
    }

    /// A hack that lets us properly special-case string vectors.
    #[doc(hidden)]
    fn casatables_stringvec_pass_through(_s: Vec<String>) -> Self {
        unreachable!();
    }

    /// A hack that lets us properly special-case string vectors
    #[doc(hidden)]
    fn casatables_stringvec_pass_through_out(_s: &Self) -> Vec<glue::StringBridge> {
        unreachable!();
    }

    /// A hack that lets us properly special-case table records.
    #[doc(hidden)]
    fn casatables_tablerec_pass_through(_r: TableRecord) -> Self {
        unreachable!();
    }

    /// A hack that lets us properly special-case table records
    #[doc(hidden)]
    fn casatables_tablerec_pass_through_out(_r: &Self) -> TableRecord {
        unreachable!();
    }

    /// Default behavior: fill the dest with a zero shape, i.e. report that we're a scalar.
    #[doc(hidden)]
    fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
        shape_dest.truncate(0);
    }

    #[doc(hidden)]
    // The corresponding type for a "MaybeUninit" version of this datatype.
    type MaybeUninit: UninitedCasaData;

    #[doc(hidden)]
    // Allocate an uninitialized buffer for this datatype.
    fn casatables_alloc(shape: &[u64]) -> Result<Self::MaybeUninit, TableError>;

    #[doc(hidden)]
    // Indicate that we can now assume that this value is fully initialized.
    unsafe fn casatables_assume_init(buf: Self::MaybeUninit) -> Self;

    #[doc(hidden)]
    fn casatables_as_buf(&self) -> *const () {
        self as *const Self as _
    }
}

#[doc(hidden)]
// This trait needs to be public because it is referenced in CasaDataType, but
// it is not something that crate consumers should ever need to use or even know
// about.
pub trait UninitedCasaData {
    #[doc(hidden)]
    // Get a mut-ptr reference to this buffer suitable for passing into the CASA
    // APIs.
    fn casatables_uninit_as_mut_ptr(&mut self) -> *mut ();
}

/// A type that maps to one of CASA's scalar data types.
pub trait CasaScalarData: CasaDataType {
    /// The CASA data type value associated with the vector form of this scalar
    /// data type.
    const VECTOR_TYPE: glue::GlueDataType;
}

macro_rules! impl_scalar_data_type {
    ($rust_type:ty, $casa_scalar_type:ident, $casa_vector_type:ident, $default:expr) => {
        impl CasaDataType for $rust_type {
            const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::$casa_scalar_type;

            type MaybeUninit = StdMaybeUninit<Self>;

            fn casatables_alloc(_shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
                Ok(StdMaybeUninit::new($default))
            }

            unsafe fn casatables_assume_init(buf: Self::MaybeUninit) -> Self {
                buf.assume_init()
            }
        }

        impl UninitedCasaData for StdMaybeUninit<$rust_type> {
            fn casatables_uninit_as_mut_ptr(&mut self) -> *mut () {
                self.as_mut_ptr() as _
            }
        }

        impl CasaScalarData for $rust_type {
            const VECTOR_TYPE: glue::GlueDataType = glue::GlueDataType::$casa_vector_type;
        }
    };
}

impl_scalar_data_type! { bool, TpBool, TpArrayBool, false }
impl_scalar_data_type! { i8, TpChar, TpArrayChar, 0 }
impl_scalar_data_type! { u8, TpUChar, TpArrayUChar, 0 }
impl_scalar_data_type! { i16, TpShort, TpArrayShort, 0 }
impl_scalar_data_type! { u16, TpUShort, TpArrayUShort, 0 }
impl_scalar_data_type! { i32, TpInt, TpArrayInt, 0 }
impl_scalar_data_type! { u32, TpUInt, TpArrayUInt, 0 }
impl_scalar_data_type! { i64, TpInt64, TpArrayInt64, 0 }
impl_scalar_data_type! { f32, TpFloat, TpArrayFloat, 0. }
impl_scalar_data_type! { f64, TpDouble, TpArrayDouble, 0. }
impl_scalar_data_type! { Complex<f32>, TpComplex, TpArrayComplex, Complex::new(0., 0.) }
impl_scalar_data_type! { Complex<f64>, TpDComplex, TpArrayDComplex, Complex::new(0., 0.) }

impl CasaDataType for String {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpString;

    fn casatables_string_pass_through(s: String) -> Self {
        s
    }

    fn casatables_string_pass_through_out(s: &Self) -> String {
        s.to_owned()
    }

    // Strings must be special-cased in the API functions and cannot be handled
    // as buffers that can be converted to pointers. When it comes to this
    // associated type, this is exactly what the "never" type is for.
    type MaybeUninit = never::Never;

    fn casatables_alloc(_shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
        unreachable!()
    }

    unsafe fn casatables_assume_init(_buf: Self::MaybeUninit) -> Self {
        unreachable!()
    }

    fn casatables_as_buf(&self) -> *const () {
        panic!("disallowed for string values")
    }
}

impl UninitedCasaData for never::Never {
    fn casatables_uninit_as_mut_ptr(&mut self) -> *mut () {
        unreachable!()
    }
}

impl CasaScalarData for String {
    const VECTOR_TYPE: glue::GlueDataType = glue::GlueDataType::TpArrayString;
}

// Vec<T> mappings. Unfortunately trait specialization is not yet stable, so
// we have to implement each type separately because Strings need special
// handling.

macro_rules! impl_vec_data_type {
    ($rust_type:ty, $casa_type:ident) => {
        impl CasaDataType for Vec<$rust_type> {
            const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::$casa_type;

            type MaybeUninit = Vec<StdMaybeUninit<$rust_type>>;

            fn casatables_alloc(shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
                if shape.len() != 1 {
                    Err(DimensionMismatchError {
                        expected: 1,
                        actual: shape.len(),
                    }
                    .into())
                } else {
                    let mut rv = Vec::with_capacity(shape[0] as usize);
                    unsafe {
                        rv.set_len(shape[0] as usize);
                    }
                    Ok(rv)
                }
            }

            unsafe fn casatables_assume_init(buf: Self::MaybeUninit) -> Self {
                // This appears to be the recommended way to do this ...
                std::mem::transmute::<_, Vec<$rust_type>>(buf)
            }

            fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
                shape_dest.truncate(0);
                shape_dest.push(self.len() as u64);
            }

            fn casatables_as_buf(&self) -> *const () {
                self.as_ptr() as _
            }
        }

        impl UninitedCasaData for Vec<StdMaybeUninit<$rust_type>> {
            fn casatables_uninit_as_mut_ptr(&mut self) -> *mut () {
                self.as_mut_ptr() as _
            }
        }
    };
}

impl_vec_data_type! { bool, TpArrayBool }
impl_vec_data_type! { i8, TpArrayChar }
impl_vec_data_type! { u8, TpArrayUChar }
impl_vec_data_type! { i16, TpArrayShort }
impl_vec_data_type! { u16, TpArrayUShort }
impl_vec_data_type! { i32, TpArrayInt }
impl_vec_data_type! { u32, TpArrayUInt }
impl_vec_data_type! { i64, TpArrayInt64 }
impl_vec_data_type! { f32, TpArrayFloat }
impl_vec_data_type! { f64, TpArrayDouble }
impl_vec_data_type! { Complex<f32>, TpArrayComplex }
impl_vec_data_type! { Complex<f64>, TpArrayDComplex }

impl CasaDataType for Vec<String> {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpArrayString;

    // As with scalar strings, we must never use the alloc/assume_init API for
    // this datatype, so:
    type MaybeUninit = never::Never;

    fn casatables_alloc(_shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
        unreachable!()
    }

    unsafe fn casatables_assume_init(_buf: Self::MaybeUninit) -> Self {
        unreachable!()
    }

    fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
        shape_dest.truncate(0);
        shape_dest.push(self.len() as u64);
    }

    fn casatables_stringvec_pass_through(s: Vec<String>) -> Self {
        s
    }

    fn casatables_stringvec_pass_through_out(svec: &Self) -> Vec<glue::StringBridge> {
        svec.iter()
            .map(|s| glue::StringBridge::from_rust(s))
            .collect()
    }

    fn casatables_as_buf(&self) -> *const () {
        self.as_ptr() as _
    }
}

// Blanket implementation of n-dimensional array mappings.
impl<I: CasaScalarData + Copy, D: Dimension + DimFromShapeSlice<u64>> CasaDataType for Array<I, D> {
    const DATA_TYPE: glue::GlueDataType = I::VECTOR_TYPE;

    type MaybeUninit = Array<StdMaybeUninit<I>, D>;

    fn casatables_alloc(shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
        Ok(Self::uninit(D::from_shape_slice(shape)?))
    }

    unsafe fn casatables_assume_init(buf: Self::MaybeUninit) -> Self {
        buf.assume_init()
    }

    fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
        shape_dest.truncate(0);
        for s in self.shape() {
            shape_dest.push(*s as u64);
        }
    }

    fn casatables_as_buf(&self) -> *const () {
        self.as_ptr() as _
    }
}

impl<I: ndarray::RawDataMut, D: Dimension> UninitedCasaData for ArrayBase<I, D> {
    fn casatables_uninit_as_mut_ptr(&mut self) -> *mut () {
        self.as_mut_ptr() as _
    }
}

#[cfg(test)]
mod data_type_tests {
    use super::*;

    #[test]
    fn sizes() {
        bool::test_casa_data_size();
        i8::test_casa_data_size();
        u8::test_casa_data_size();
        i16::test_casa_data_size();
        u16::test_casa_data_size();
        i32::test_casa_data_size();
        u32::test_casa_data_size();
        i64::test_casa_data_size();
        f32::test_casa_data_size();
        f64::test_casa_data_size();
        Complex::<f32>::test_casa_data_size();
        Complex::<f64>::test_casa_data_size();
    }
}

// String bridge

impl glue::StringBridge {
    fn from_rust(s: &str) -> Self {
        Self {
            data: s.as_ptr() as _,
            n_bytes: s.len() as std::os::raw::c_ulong,
        }
    }

    // This function should only be called inside a callback from the C++ code.
    // Otherwise, it is essentially impossible to ensure that the data pointer
    // is valid and that its contents are uncorrupted. (The only time you can be
    // sure of that is if your C++ string points to data owned by a data
    // structure whose lifetime is long compared to the Rust code, which is far
    // from generically true.)
    #[allow(clippy::wrong_self_convention)]
    fn to_rust(&self) -> String {
        let buf =
            unsafe { std::slice::from_raw_parts(self.data as *const u8, self.n_bytes as usize) };

        String::from_utf8_lossy(buf).into_owned()
    }
}

// The only truly safe way to get C++ strings into Rust is with a callback --
// otherwise, it is extremely hard to be sure that the string's buffer will be
// valid after the C++ stack frames have exited, and even harder to be sure that
// they'll remain valid. The first part of the requisite dance is an `extern "C"
// fn` callback that the C++ code can call safely.

unsafe extern "C" fn casatables_string_bridge_cb<F>(
    name: *const glue::StringBridge,
    ctxt: *mut std::os::raw::c_void,
) where
    F: FnMut(String),
{
    let f: &mut F = &mut *(ctxt as *mut F);
    f((*name).to_rust())
}

unsafe extern "C" fn casatables_keyword_info_cb<F>(
    name: *const glue::StringBridge,
    dtype: glue::GlueDataType,
    ctxt: *mut std::os::raw::c_void,
) where
    F: FnMut(String, glue::GlueDataType),
{
    let f: &mut F = &mut *(ctxt as *mut F);
    f((*name).to_rust(), dtype)
}

unsafe extern "C" fn casatables_keyword_repr_cb<F>(
    name: *const glue::StringBridge,
    dtype: glue::GlueDataType,
    repr: *const glue::StringBridge,
    ctxt: *mut std::os::raw::c_void,
) where
    F: FnMut(String, glue::GlueDataType, String),
{
    let f: &mut F = &mut *(ctxt as *mut F);
    f((*name).to_rust(), dtype, (*repr).to_rust())
}

// The next part: wrappers that allow us to invoke the various callback-having
// functions with Rust closures. The main point to having these functions is
// basically to be able to assign a name to the function type `F`.

unsafe fn invoke_table_get_column_names<F>(
    handle: *mut glue::GlueTable,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_get_column_names(
        handle,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_keyword_info<F>(
    handle: *mut glue::GlueTable,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String, glue::GlueDataType),
{
    glue::table_get_keyword_info(
        handle,
        Some(casatables_keyword_info_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_column_keyword_info<F>(
    handle: *mut glue::GlueTable,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String, glue::GlueDataType),
{
    glue::table_get_column_keyword_info(
        handle,
        ccol_name,
        Some(casatables_keyword_info_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_scalar_column_data_string<F>(
    handle: *mut glue::GlueTable,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_get_scalar_column_data_string(
        handle,
        ccol_name,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_cell_string<F>(
    handle: *mut glue::GlueTable,
    ccol_name: &glue::StringBridge,
    row: u64,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_get_cell_string(
        handle,
        ccol_name,
        row,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_file_name<F>(
    handle: *mut glue::GlueTable,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_get_file_name(
        handle,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_get_cell_string_array<F>(
    handle: *mut glue::GlueTable,
    ccol_name: &glue::StringBridge,
    row: u64,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_get_cell_string_array(
        handle,
        ccol_name,
        row,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_tablerec_get_keyword_info<F>(
    handle: *mut glue::GlueTableRecord,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String, glue::GlueDataType),
{
    glue::tablerec_get_keyword_info(
        handle,
        Some(casatables_keyword_info_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_tablerec_get_keyword_repr<F>(
    handle: *mut glue::GlueTableRecord,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String, glue::GlueDataType, String),
{
    glue::tablerec_get_keyword_repr(
        handle,
        Some(casatables_keyword_repr_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_tablerec_get_field_string<F>(
    handle: *mut glue::GlueTableRecord,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::tablerec_get_field_string(
        handle,
        ccol_name,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_tablerec_get_field_string_array<F>(
    handle: *mut glue::GlueTableRecord,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::tablerec_get_field_string_array(
        handle,
        ccol_name,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_row_get_cell_string<F>(
    handle: *mut glue::GlueTableRow,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_row_get_cell_string(
        handle,
        ccol_name,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

unsafe fn invoke_table_row_get_cell_string_array<F>(
    handle: *mut glue::GlueTableRow,
    ccol_name: &glue::StringBridge,
    exc_info: &mut glue::ExcInfo,
    mut f: F,
) -> std::os::raw::c_int
where
    F: FnMut(String),
{
    glue::table_row_get_cell_string_array(
        handle,
        ccol_name,
        Some(casatables_string_bridge_cb::<F>),
        &mut f as *mut _ as *mut std::os::raw::c_void,
        exc_info,
    )
}

/// Information about the structure of a CASA table.
///
/// From the casacore documentation: "A TableDesc object contains the
/// description, or structure, of a table. This description is required for the
/// creation of a new table. Descriptions are subsequently associated with every
/// table and embedded in them.""
///
/// # Examples
///
/// Create a description of a table named "TYPE", with a scalar string column
/// named "A" with comment "string", a column of unsigned integer arrays of no
/// fixed size named "B" with comment "uint array", and a column of double
/// precision complex number arrays of shape `[4]` named "C" with comment "fixed
/// complex vector"
///
/// ```rust
/// use rubbl_casatables::{GlueDataType, TableDescCreateMode, TableDesc};
///
/// let mut table_desc = TableDesc::new("TYPE", TableDescCreateMode::TDM_SCRATCH).unwrap();
/// table_desc
///     .add_scalar_column(GlueDataType::TpString, "A", Some("string"), false, false).unwrap();
/// table_desc
///     .add_array_column(GlueDataType::TpUInt, "B", Some("uint array"), None, false, false).unwrap();
/// table_desc
///     .add_array_column(GlueDataType::TpDComplex, "C", Some("fixed complex vector"), Some(&[4]), false, false).unwrap();
/// ```
pub struct TableDesc {
    handle: *mut glue::GlueTableDesc,
    exc_info: glue::ExcInfo,
}

impl TableDesc {
    /// Create a new TableDesc.
    ///
    /// `name` - The name of the table description. From casacore:
    ///     This name can be seen as the table type in the same way as a
    ///     class name is the data type of an object.
    ///
    /// `mode` - The mode in which to create the table description.
    ///     For compatibility with casacore, multiple options are provided,
    ///     however you most likely want to go with Scratch, as this avoids
    ///     writing a .tabdsc file to disk.
    ///
    pub fn new(name: &str, mode: glue::TableDescCreateMode) -> Result<Self, TableError> {
        let cname = glue::StringBridge::from_rust(name);
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::tabledesc_create(&cname, mode, &mut exc_info) };

        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(TableDesc { handle, exc_info })
    }

    /// Add a scalar column to the TableDesc
    pub fn add_scalar_column(
        &mut self,
        data_type: glue::GlueDataType,
        col_name: &str,
        comment: Option<&str>,
        direct: bool,
        undefined: bool,
    ) -> Result<(), TableError> {
        let cname = glue::StringBridge::from_rust(col_name);
        let comment = comment.unwrap_or_default();
        let ccomment = glue::StringBridge::from_rust(comment);
        let rv = unsafe {
            glue::tabledesc_add_scalar_column(
                self.handle,
                data_type,
                &cname,
                &ccomment,
                direct,
                undefined,
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Add an array column to the TableDesc
    ///
    /// If dimensions (`dims`) are provided, then the column has fixed dimensions,
    /// other wise the column is not fixed.
    pub fn add_array_column(
        &mut self,
        data_type: glue::GlueDataType,
        col_name: &str,
        comment: Option<&str>,
        dims: Option<&[u64]>,
        direct: bool,
        undefined: bool,
    ) -> Result<(), TableError> {
        let cname = glue::StringBridge::from_rust(col_name);
        let comment = comment.unwrap_or_default();
        let ccomment = glue::StringBridge::from_rust(comment);
        let rv = unsafe {
            if let Some(dims_) = dims {
                glue::tabledesc_add_fixed_array_column(
                    self.handle,
                    data_type,
                    &cname,
                    &ccomment,
                    dims_.len() as u64,
                    dims_.as_ptr(),
                    direct,
                    undefined,
                    &mut self.exc_info,
                )
            } else {
                glue::tabledesc_add_array_column(
                    self.handle,
                    data_type,
                    &cname,
                    &ccomment,
                    direct,
                    undefined,
                    &mut self.exc_info,
                )
            }
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Set the number of dimensions of a column
    pub fn set_ndims(&mut self, col_name: &str, ndims: u64) -> Result<(), TableError> {
        let cname = glue::StringBridge::from_rust(col_name);
        let rv =
            unsafe { glue::tabledesc_set_ndims(self.handle, &cname, ndims, &mut self.exc_info) };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Return a copy of the keyword TableRecord
    pub fn get_keyword_record(&mut self) -> Result<TableRecord, CasacoreError> {
        let handle = unsafe { glue::tabledesc_get_keywords(self.handle, &mut self.exc_info) };

        if handle.is_null() {
            return self.exc_info.as_err();
        }

        TableRecord::copy_handle(unsafe { &*handle })
    }

    /// Return a copy of the keyword TableRecord for a given column.
    pub fn get_column_keyword_record(
        &mut self,
        col_name: &str,
    ) -> Result<TableRecord, CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let handle = unsafe {
            glue::tabledesc_get_column_keywords(self.handle, &ccol_name, &mut self.exc_info)
        };

        if handle.is_null() {
            return self.exc_info.as_err();
        }

        TableRecord::copy_handle(unsafe { &*handle })
    }

    /// Add a "keyword" to be associated with a particular column in this table
    /// description.
    ///
    /// `col_name` - The name of the affected column.
    ///
    /// `kw_name` - The name of the keyword to apply to the column.
    ///
    /// `value` - The value to associate with the keyword.
    pub fn put_column_keyword<T: CasaDataType>(
        &mut self,
        col_name: &str,
        kw_name: &str,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let ckw_name = glue::StringBridge::from_rust(kw_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::tabledesc_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::tabledesc_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::tabledesc_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }
}

// Tables

/// A CASA data table.
pub struct Table {
    handle: *mut glue::GlueTable,
    exc_info: glue::ExcInfo,
}

/// Modes in which a casacore table can be opened.
///
pub enum TableOpenMode {
    /// Open the table for read-only access.
    Read = 1,

    /// Open the table for read-write access.
    ReadWrite = 2,

    /// Create a new table.
    Create = 3,
}

/// Modes in which a casacore table can be created.
///
/// ## Note
///
/// Casacore allows for an additional mode, `Scratch` which it describes as "new
/// table, which gets marked for delete".
///
/// The use case for this was unclear, but if you have a use for this mode,
/// consider opening an issue in rubbl.
///
/// For more details about the discussion of this mode, see [this GitHub
/// comment][x].
///
/// [x]: https://github.com/pkgw/rubbl/pull/160#discussion_r707433551
///
pub enum TableCreateMode {
    /// Create a new table.
    ///
    /// **To check:** if the table already exists, do we delete it or open
    /// it as-is or what?
    New = 1,

    /// Create a new table, raising an error if it already exists.
    NewNoReplace = 2,
}

/// An error type used when the expected data type was not found.
///
/// The first element of the tuple is the expected data type, and the second
/// element is the one that was actually encountered.
#[derive(Error, Debug)]
#[error("Expected data with the storage type {0}, but found {1}")]
pub struct UnexpectedDataTypeError(glue::GlueDataType, glue::GlueDataType);

/// An error type capturing all potential problems when interfacing with
/// [`Table`]s.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum TableError {
    /// Table paths must be representable as UTF-8 strings.
    #[error("table paths must be representable as UTF-8 strings")]
    InvalidUtf8,

    /// Expected a scalar, but got a vector.
    #[error("Expected a column with a scalar data type, but found a vector of {0}")]
    NotScalarColumnError(glue::GlueDataType),

    /// Received a different type from what was expected.
    #[error(transparent)]
    UnexpectedDataType(#[from] UnexpectedDataTypeError),

    /// Generic casacore C++ exception.
    #[error(transparent)]
    Casacore(#[from] CasacoreError),

    /// An error type used when two arrays should have the same dimensionality,
    /// but do not.
    #[error(transparent)]
    DimensionMismatch(#[from] DimensionMismatchError),

    /// An error type for when a casatables user wants to propagate a simple
    /// message up the call chain.
    ///
    /// This variant can be useful in functions like [`Table::for_each_row`],
    /// where you might have some custom logic leading to an error that isn't
    /// easily describable using one of the more specific variants.
    #[error("{0}")]
    UserMessage(String),
}

/// A Rust wrapper for a casacore table.
///
/// For details on the casacore table concepts as expressed in the underlying
/// C++ implementation, see the
/// [documentation](https://casacore.github.io/casacore/group__Tables__module.html#details).
impl Table {
    /// Create a new casacore table.
    ///
    /// To open an existing table, use [`Table::open`].
    ///
    /// Read more about the casacore tables C++ interface
    /// [here](https://casacore.github.io/casacore/group__Tables__module.html#details).
    ///
    /// # Examples
    ///
    /// Creating a table:
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use tempfile::tempdir;
    /// use rubbl_casatables::{
    ///     ColumnDescription, GlueDataType, Table,
    ///     TableCreateMode, TableDesc, TableDescCreateMode,
    /// };
    ///
    /// // tempdir is only necessary to avoid writing to disk each time this example is run
    /// let tmp_dir = tempdir().unwrap();
    /// let table_path = tmp_dir.path().join("test.ms");
    ///
    /// // First create a table description for our base table.
    /// // Use TDM_SCRATCH to avoid writing a `.tabdsc` file to disk.
    /// let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
    ///
    /// // Define the columns in your table description:
    /// table_desc.add_array_column(
    ///     GlueDataType::TpDouble,                           // the data type
    ///     "UVW",                                            // the column name
    ///     Some("Vector with uvw coordinates (in meters)"),  // an optional comment about the column
    ///     Some(&[3]),                                       // a required vector shape for the volumn
    ///     true,                                             // "direct": whether data are stored in the table
    ///     false,                                            // Whether some scalar values are treated as undefined
    /// ).unwrap();
    ///
    /// // Create your new table with 0 rows.
    /// Table::new(&table_path, table_desc, 0, TableCreateMode::New).unwrap();
    /// ```
    pub fn new<P: AsRef<Path>>(
        path: P,
        table_desc: TableDesc,
        n_rows: usize,
        mode: TableCreateMode,
    ) -> Result<Self, TableError> {
        let spath = match path.as_ref().to_str() {
            Some(s) => s,
            None => {
                return Err(TableError::InvalidUtf8);
            }
        };

        let cpath = glue::StringBridge::from_rust(spath);
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let cmode = match mode {
            TableCreateMode::New => glue::TableCreateMode::TCM_NEW,
            TableCreateMode::NewNoReplace => glue::TableCreateMode::TCM_NEW_NO_REPLACE,
            // TableCreateMode::Scratch => glue::TableCreateMode::TCM_SCRATCH,
        };

        let handle = unsafe {
            glue::table_create(
                &cpath,
                table_desc.handle,
                n_rows as u64,
                cmode,
                &mut exc_info,
            )
        };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Table { handle, exc_info })
    }

    /// Open an existing casacore table.
    ///
    /// To create a table, use [`Table::new`]. Do not use
    /// [`TableOpenMode::Create`] with this function — it will not do what you
    /// want.
    ///
    /// Read more about the casacore tables C++ interface
    /// [here](https://casacore.github.io/casacore/group__Tables__module.html#details).
    ///
    /// # Examples
    ///
    /// Creating a table, writing a cell, opening it and reading the cell.
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use tempfile::tempdir;
    /// use rubbl_casatables::{ColumnDescription, GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode, TableOpenMode};
    ///
    /// // tempdir is only necessary to avoid writing to disk each time this example is run
    /// let tmp_dir = tempdir().unwrap();
    /// let table_path = tmp_dir.path().join("test.ms");
    ///
    /// // First create a table description for our base table.
    /// // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
    /// let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
    ///
    /// // Define the columns in your table description:
    /// table_desc.add_array_column(
    ///     GlueDataType::TpDouble,                           // the data type
    ///     "UVW",                                            // the column name
    ///     Some("Vector with uvw coordinates (in meters)"),  // an optional comment about the column
    ///     Some(&[3]),                                       // a required vector shape for the volumn
    ///     true,                                             // "direct": whether data are stored in the table
    ///     false,                                            // Whether some scalar values are treated as undefined
    /// ).unwrap();
    ///
    /// // Create the new table with 1 row.
    /// let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
    ///
    /// // Write to the first row in the uvw column
    /// let cell_value: Vec<f64> = vec![1.0, 2.0, 3.0];
    /// table.put_cell("UVW", 0, &cell_value).unwrap();
    ///
    /// // This writes the table to disk and closes the file pointer:
    /// drop(table);
    ///
    /// // Now open the table:
    /// let mut table = Table::open(&table_path, TableOpenMode::ReadWrite).unwrap();
    ///
    /// // ... and extract the cell value we wrote earlier:
    /// let extracted_cell_value: Vec<f64> = table.get_cell_as_vec("UVW", 0).unwrap();
    /// assert_eq!(cell_value, extracted_cell_value);
    /// ```
    ///
    /// # Errors
    ///
    /// Can raise [`CasacoreError`] if there was an issue invoking casacore.
    pub fn open<P: AsRef<Path>>(path: P, mode: TableOpenMode) -> Result<Self, TableError> {
        let spath = match path.as_ref().to_str() {
            Some(s) => s,
            None => return Err(TableError::InvalidUtf8),
        };
        let cpath = glue::StringBridge::from_rust(spath);
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let cmode = match mode {
            TableOpenMode::Read => glue::TableOpenMode::TOM_OPEN_READONLY,
            TableOpenMode::ReadWrite => glue::TableOpenMode::TOM_OPEN_RW,
            TableOpenMode::Create => glue::TableOpenMode::TOM_CREATE,
        };

        let handle = unsafe { glue::table_alloc_and_open(&cpath, cmode, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Table { handle, exc_info })
    }

    /// Get the number of rows in the table.
    pub fn n_rows(&self) -> u64 {
        unsafe { glue::table_n_rows(self.handle) as u64 }
    }

    /// Get the number of columns in the table.
    pub fn n_columns(&self) -> usize {
        unsafe { glue::table_n_columns(self.handle) as usize }
    }

    /// Get the filesystem path associated with the table.
    ///
    /// This function should only fail of the underlying C++ throws an
    /// exception, which *should* basically never happen for this operation.
    pub fn file_name(&self) -> Result<String, CasacoreError> {
        // this is to allow filename access without &mut.
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };
        let mut result: String = "".into();
        let rv = unsafe {
            invoke_table_get_file_name(self.handle, &mut exc_info, |file_name| {
                result.push_str(&file_name);
            }) as usize
        };

        if rv != 0 {
            return exc_info.as_err();
        }

        Ok(result)
    }

    /// Get a vector containing all of the column names in the table.
    ///
    /// # Errors
    ///
    /// Can raise [`CasacoreError`] if there was an issue invoking casacore
    pub fn column_names(&mut self) -> Result<Vec<String>, CasacoreError> {
        let n_cols = self.n_columns();
        let mut cnames = Vec::with_capacity(n_cols);

        let rv = unsafe {
            invoke_table_get_column_names(self.handle, &mut self.exc_info, |name| {
                cnames.push(name);
            })
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(cnames)
    }

    /// Remove a column from the table.
    ///
    /// # Errors
    ///
    /// Can raise [`CasacoreError`] if there was an issue invoking casacore.
    /// **To check:** this probably returns an error if the named column was not
    /// present?
    pub fn remove_column(&mut self, col_name: &str) -> Result<(), CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);

        let rv = unsafe { glue::table_remove_column(self.handle, &ccol_name, &mut self.exc_info) };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Add a scalar column to the table.
    ///
    /// `col_name` - column name, must be unique
    ///
    /// `comment` - optional string field describing how to use the column.
    ///
    /// `direct` - Whether the underyling data are stored directly in the table.
    ///     This should almost always be `true`.
    ///
    /// # Errors
    ///
    /// Can raise [`CasacoreError`] if there was an issue invoking casacore
    pub fn add_scalar_column(
        &mut self,
        data_type: glue::GlueDataType,
        col_name: &str,
        comment: Option<&str>,
        direct: bool,
        undefined: bool,
    ) -> Result<(), CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let comment = comment.unwrap_or_default();
        let ccomment = glue::StringBridge::from_rust(comment);

        let rv = unsafe {
            glue::table_add_scalar_column(
                self.handle,
                data_type,
                &ccol_name,
                &ccomment,
                direct,
                undefined,
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Add an array column to the Table
    ///
    /// If dimensions (`dims`) are provided, then the column has fixed dimensions,
    /// other wise the column is not fixed.
    pub fn add_array_column(
        &mut self,
        data_type: glue::GlueDataType,
        col_name: &str,
        comment: Option<&str>,
        dims: Option<&[u64]>,
        direct: bool,
        undefined: bool,
    ) -> Result<(), TableError> {
        let cname = glue::StringBridge::from_rust(col_name);
        let comment = comment.unwrap_or_default();
        let ccomment = glue::StringBridge::from_rust(comment);
        let rv = unsafe {
            if let Some(dims_) = dims {
                glue::table_add_fixed_array_column(
                    self.handle,
                    data_type,
                    &cname,
                    &ccomment,
                    dims_.len() as u64,
                    dims_.as_ptr(),
                    direct,
                    undefined,
                    &mut self.exc_info,
                )
            } else {
                glue::table_add_array_column(
                    self.handle,
                    data_type,
                    &cname,
                    &ccomment,
                    direct,
                    undefined,
                    &mut self.exc_info,
                )
            }
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Return all of the names of keywords whose values are type `TpTable`.
    pub fn table_keyword_names(&mut self) -> Result<Vec<String>, CasacoreError> {
        let mut result = Vec::new();

        let rv = unsafe {
            invoke_table_get_keyword_info(self.handle, &mut self.exc_info, |name, dtype| {
                if dtype == glue::GlueDataType::TpTable {
                    result.push(name);
                }
            })
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(result)
    }

    /// Return all of the names of keywords for a given column
    pub fn column_keyword_names(&mut self, col_name: &str) -> Result<Vec<String>, CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut result = Vec::new();

        let rv = unsafe {
            invoke_table_get_column_keyword_info(
                self.handle,
                &ccol_name,
                &mut self.exc_info,
                |name, _dtype| {
                    result.push(name);
                },
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(result)
    }

    /// Define a keyword of type `TpTable` in this table
    pub fn put_table_keyword(&mut self, kw_name: &str, table: Table) -> Result<(), CasacoreError> {
        let ckw_name = glue::StringBridge::from_rust(kw_name);
        let shape = Vec::new();
        let rv = unsafe {
            glue::table_put_keyword(
                self.handle,
                &ckw_name,
                glue::GlueDataType::TpTable,
                shape.len() as _,
                shape.as_ptr(),
                table.handle as _,
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    // TODO: dedup from TableDesc::put_keyword
    /// Add a "keyword" to be associated with the table.
    ///
    /// A keyword is essentially a name-value pair, where the associated value
    /// need not be a simple data type: it can be a record or even a sub-table.
    ///
    /// See also [`Self::put_column_keyword`].
    pub fn put_keyword<T: CasaDataType>(
        &mut self,
        kw_name: &str,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let ckw_name = glue::StringBridge::from_rust(kw_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::table_put_keyword(
                    self.handle,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::table_put_keyword(
                    self.handle,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::table_put_keyword(
                    self.handle,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }

    // TODO: dedup from TableDesc::put_column_keyword
    /// Add a "keyword" to be associated with a particular column of the table.
    ///
    /// A keyword is essentially a name-value pair, where the associated value
    /// need not be a simple data type: it can be a record or even a sub-table.
    ///
    /// See also [`Self::put_keyword`] for table-level keywords.
    pub fn put_column_keyword<T: CasaDataType>(
        &mut self,
        col_name: &str,
        kw_name: &str,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let ckw_name = glue::StringBridge::from_rust(kw_name);
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::table_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::table_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::table_put_column_keyword(
                    self.handle,
                    &ccol_name,
                    &ckw_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }

    /// Return a TableRecord containing all keyword / value pairs for this
    /// table.
    pub fn get_keyword_record(&mut self) -> Result<TableRecord, CasacoreError> {
        let handle = unsafe { glue::table_get_keywords(self.handle, &mut self.exc_info) };

        if handle.is_null() {
            return self.exc_info.as_err();
        }

        TableRecord::copy_handle(unsafe { &*handle })
    }

    /// Return a TableRecord containing all keyword / value pairs for the named
    /// column.
    pub fn get_column_keyword_record(
        &mut self,
        col_name: &str,
    ) -> Result<TableRecord, CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let handle =
            unsafe { glue::table_get_column_keywords(self.handle, &ccol_name, &mut self.exc_info) };

        if handle.is_null() {
            return self.exc_info.as_err();
        }

        TableRecord::copy_handle(unsafe { &*handle })
    }

    /// Get the description of the named column.
    ///
    /// The returned [`ColumnDescription`] handle provides access to column's
    /// data type, required vector shape, keywords, and other such metadata.
    pub fn get_col_desc(&mut self, col_name: &str) -> Result<ColumnDescription, CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut n_rows = 0;
        let mut data_type = glue::GlueDataType::TpOther;
        let mut is_scalar = 0;
        let mut is_fixed_shape = 0;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::table_get_column_info(
                self.handle,
                &ccol_name,
                &mut n_rows,
                &mut data_type,
                &mut is_scalar,
                &mut is_fixed_shape,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        let shape = if is_fixed_shape == 0 || n_dim < 0 {
            None
        } else {
            let mut v = Vec::new();

            for d in &dims[..n_dim as usize] {
                v.push(*d);
            }

            Some(v)
        };

        let keywords = self.get_column_keyword_record(col_name).unwrap();

        Ok(ColumnDescription {
            name: col_name.to_owned(),
            data_type,
            is_scalar: is_scalar != 0,
            is_fixed_shape: is_fixed_shape != 0,
            shape,
            keywords,
        })
    }

    /// Get all of the data in a column as one vector.
    ///
    /// The underlying data type of the column must be scalar. Use this function
    /// wisely since some CASA tables may contain millions of rows.
    pub fn get_col_as_vec<T: CasaScalarData>(
        &mut self,
        col_name: &str,
    ) -> Result<Vec<T>, TableError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut n_rows = 0;
        let mut data_type = glue::GlueDataType::TpOther;
        let mut is_scalar = 0;
        let mut is_fixed_shape = 0;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::table_get_column_info(
                self.handle,
                &ccol_name,
                &mut n_rows,
                &mut data_type,
                &mut is_scalar,
                &mut is_fixed_shape,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if is_scalar == 0 || is_fixed_shape == 0 || n_dim != 0 {
            return Err(TableError::NotScalarColumnError(data_type));
        }

        if data_type != T::DATA_TYPE {
            return Err(UnexpectedDataTypeError(T::DATA_TYPE, data_type).into());
        }

        let mut result = Vec::<T>::with_capacity(n_rows as usize);

        if data_type != glue::GlueDataType::TpString {
            let rv = unsafe {
                glue::table_get_scalar_column_data(
                    self.handle,
                    &ccol_name,
                    result.as_mut_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe {
                result.set_len(n_rows as usize);
            }
        } else {
            let rv = unsafe {
                invoke_table_get_scalar_column_data_string(
                    self.handle,
                    &ccol_name,
                    &mut self.exc_info,
                    |v| {
                        result.push(T::casatables_string_pass_through(v));
                    },
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        };

        Ok(result)
    }

    /// Get the value of one cell of the table.
    pub fn get_cell<T: CasaDataType>(&mut self, col_name: &str, row: u64) -> Result<T, TableError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut data_type = glue::GlueDataType::TpOther;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::table_get_cell_info(
                self.handle,
                &ccol_name,
                row,
                &mut data_type,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(UnexpectedDataTypeError(T::DATA_TYPE, data_type).into());
        }

        let result = if data_type != glue::GlueDataType::TpString {
            let mut result = T::casatables_alloc(&dims[..n_dim as usize])?;

            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    result.casatables_uninit_as_mut_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe { T::casatables_assume_init(result) }
        } else {
            let mut value = None;

            let rv = unsafe {
                invoke_table_get_cell_string(
                    self.handle,
                    &ccol_name,
                    row,
                    &mut self.exc_info,
                    |v| {
                        value = Some(v);
                    },
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_string_pass_through(value.unwrap())
        };

        Ok(result)
    }

    /// Get the contents of one cell of the table as a simple Rust vector.
    ///
    /// This function discards shape information and won't accept scalars.
    pub fn get_cell_as_vec<T: CasaScalarData>(
        &mut self,
        col_name: &str,
        row: u64,
    ) -> Result<Vec<T>, TableError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut data_type = glue::GlueDataType::TpOther;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::table_get_cell_info(
                self.handle,
                &ccol_name,
                row,
                &mut data_type,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(UnexpectedDataTypeError(T::DATA_TYPE, data_type).into());
        }

        let n_items = dims[..n_dim as usize]
            .iter()
            .fold(1usize, |p, n| p * (*n as usize));

        let mut result = Vec::<T>::with_capacity(n_items);

        if data_type != glue::GlueDataType::TpString {
            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    result.as_mut_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe {
                result.set_len(n_items);
            }
        } else {
            let rv = unsafe {
                invoke_table_get_cell_string_array(
                    self.handle,
                    &ccol_name,
                    row,
                    &mut self.exc_info,
                    |v| {
                        result.push(T::casatables_string_pass_through(v));
                    },
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(result)
    }

    /// Put a value for one cell of the table.
    pub fn put_cell<T: CasaDataType>(
        &mut self,
        col_name: &str,
        row: u64,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::table_put_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::table_put_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::table_put_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }

    /// Add additional, empty rows to the table.
    pub fn add_rows(&mut self, n_rows: usize) -> Result<(), CasacoreError> {
        if unsafe { glue::table_add_rows(self.handle, n_rows as u64, &mut self.exc_info) != 0 } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }

    fn get_row_handle(&mut self, is_read_only: bool) -> Result<TableRow, CasacoreError> {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };
        let ro_flag = if is_read_only { 1 } else { 0 };

        let handle = unsafe { glue::table_row_alloc(self.handle, ro_flag, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(TableRow { handle, exc_info })
    }

    /// Get an object for read-only access to individual rows of the table.
    ///
    /// The row reader can be used to iterate through the rows of the table
    /// efficiently.
    ///
    /// See also [`Self::get_row_writer`].
    pub fn get_row_reader(&mut self) -> Result<TableRow, CasacoreError> {
        self.get_row_handle(true)
    }

    /// Get an object for read-write access to individual rows of the table.
    ///
    /// The row writer can be used to iterate through the rows of the table
    /// efficiently.
    ///
    /// See also [`Self::get_row_reader`].
    pub fn get_row_writer(&mut self) -> Result<TableRow, CasacoreError> {
        self.get_row_handle(false)
    }

    /// Populate a [`TableRow`] accessor object with data from the specified
    /// row.
    pub fn read_row(&mut self, row: &mut TableRow, row_number: u64) -> Result<(), TableError> {
        if unsafe { glue::table_row_read(row.handle, row_number, &mut row.exc_info) } != 0 {
            return row.exc_info.as_err();
        }

        Ok(())
    }

    /// Perform `func` on each row of the table.
    pub fn for_each_row<F>(&mut self, mut func: F) -> Result<(), TableError>
    where
        F: FnMut(&mut TableRow) -> Result<(), TableError>,
    {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_row_alloc(self.handle, 1, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        let mut row = TableRow { handle, exc_info };

        for row_number in 0..self.n_rows() {
            if unsafe { glue::table_row_read(row.handle, row_number, &mut row.exc_info) } != 0 {
                return row.exc_info.as_err();
            }

            func(&mut row)?;
        }

        Ok(())
    }

    /// Perform `func` on each row in the range `row_range`.
    pub fn for_each_row_in_range<F>(
        &mut self,
        row_range: std::ops::Range<u64>,
        mut func: F,
    ) -> Result<(), TableError>
    where
        F: FnMut(&mut TableRow) -> Result<(), TableError>,
    {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_row_alloc(self.handle, 1, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        let mut row = TableRow { handle, exc_info };

        for row_number in row_range {
            if unsafe { glue::table_row_read(row.handle, row_number, &mut row.exc_info) } != 0 {
                return row.exc_info.as_err();
            }

            func(&mut row)?;
        }

        Ok(())
    }

    /// Perform `func` on each row indicated by `rows`.
    pub fn for_each_specific_row<F>(&mut self, rows: &[u64], mut func: F) -> Result<(), TableError>
    where
        F: FnMut(&mut TableRow) -> Result<(), TableError>,
    {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_row_alloc(self.handle, 1, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        let mut row = TableRow { handle, exc_info };

        for &row_number in rows {
            if unsafe { glue::table_row_read(row.handle, row_number, &mut row.exc_info) } != 0 {
                return row.exc_info.as_err();
            }

            func(&mut row)?;
        }

        Ok(())
    }

    /// Copy all rows from this table to another table.
    pub fn copy_rows_to(&mut self, dest: &mut Table) -> Result<(), CasacoreError> {
        if unsafe { glue::table_copy_rows(self.handle, dest.handle, &mut self.exc_info) != 0 } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }

    /// Copy the "description" of this table to a new filesystem path, without
    /// copying any of the actual data contents.
    pub fn deep_copy_no_rows(&mut self, dest_path: &str) -> Result<(), CasacoreError> {
        let cdest_path = glue::StringBridge::from_rust(dest_path);

        if unsafe {
            glue::table_deep_copy_no_rows(self.handle, &cdest_path, &mut self.exc_info) != 0
        } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }
}

impl Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Table")
            .field("handle", &self.handle)
            .field("name", &self.file_name().unwrap())
            .finish()
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        // FIXME: not sure if this function can actually produce useful
        // exceptions anyway, but we can't do anything if it does!
        unsafe { glue::table_close_and_free(self.handle, &mut self.exc_info) }
    }
}

/// Information describing the properties of a particular column of a table.
#[derive(PartialEq, Eq, Debug)]
pub struct ColumnDescription {
    name: String,
    data_type: glue::GlueDataType,
    is_scalar: bool,
    is_fixed_shape: bool,
    shape: Option<Vec<u64>>,
    keywords: TableRecord,
}

impl ColumnDescription {
    /// Get the name of the column.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the underlying CASA data type of the column.
    pub fn data_type(&self) -> glue::GlueDataType {
        self.data_type
    }

    /// Get whether the column is scalar-valued, as opposed to vector-valued.
    pub fn is_scalar(&self) -> bool {
        self.is_scalar
    }

    /// Get whether the column is vector-valued with a fixed array shape for
    /// each cell.
    pub fn is_fixed_shape(&self) -> bool {
        self.is_fixed_shape
    }

    /// If the column is vector-valued with a fixed shape for each cell, get the
    /// required shape.
    pub fn shape(&self) -> Option<&[u64]> {
        self.shape.as_ref().map(|v| &v[..])
    }
}

// Table Row handles

/// A type for examining individual rows of a CASA table.
pub struct TableRow {
    handle: *mut glue::GlueTableRow,
    exc_info: glue::ExcInfo,
}

impl TableRow {
    /// Get the value of a specific cell in this row.
    pub fn get_cell<T: CasaDataType>(&mut self, col_name: &str) -> Result<T, TableError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut data_type = glue::GlueDataType::TpOther;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::table_row_get_cell_info(
                self.handle,
                &ccol_name,
                &mut data_type,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(UnexpectedDataTypeError(T::DATA_TYPE, data_type).into());
        }

        let result = if data_type == glue::GlueDataType::TpString {
            let mut value = None;

            let rv = unsafe {
                invoke_table_row_get_cell_string(self.handle, &ccol_name, &mut self.exc_info, |v| {
                    value = Some(v);
                })
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_string_pass_through(value.unwrap())
        } else if data_type == glue::GlueDataType::TpArrayString {
            let mut result = Vec::new();

            let rv = unsafe {
                invoke_table_row_get_cell_string_array(
                    self.handle,
                    &ccol_name,
                    &mut self.exc_info,
                    |v| {
                        result.push(v);
                    },
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_stringvec_pass_through(result)
        } else {
            let mut result = T::casatables_alloc(&dims[..n_dim as usize])?;

            let rv = unsafe {
                glue::table_row_get_cell(
                    self.handle,
                    &ccol_name,
                    result.casatables_uninit_as_mut_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe { T::casatables_assume_init(result) }
        };

        Ok(result)
    }

    /// Write a new value for a specific cell in this row.
    ///
    /// Note: I am not sure if this function actually works! `Table.put_cell`
    /// does work. Investigation required.
    pub fn put_cell<T: CasaDataType>(
        &mut self,
        col_name: &str,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::table_row_put_cell(
                    self.handle,
                    &ccol_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::table_row_put_cell(
                    self.handle,
                    &ccol_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::table_row_put_cell(
                    self.handle,
                    &ccol_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }

    /// Copy the data associated with this row accessor to another one, and then
    /// write the data to the specifed row of that other accessor's associated
    /// table.
    pub fn copy_and_put(
        &mut self,
        dest: &mut TableRow,
        row_number: u64,
    ) -> Result<(), CasacoreError> {
        let rv = unsafe {
            glue::table_row_copy_and_put(self.handle, row_number, dest.handle, &mut self.exc_info)
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    /// Write the data currently associated with this row accessor to the
    /// specified row of its associated table.
    pub fn put(&mut self, row_number: u64) -> Result<(), CasacoreError> {
        let rv = unsafe { glue::table_row_write(self.handle, row_number, &mut self.exc_info) };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }
}

impl Drop for TableRow {
    fn drop(&mut self) {
        // FIXME: not sure if this function can actually produce useful
        // exceptions anyway, but we can't do anything if it does!
        unsafe {
            glue::table_row_free(self.handle, &mut self.exc_info);
        }
    }
}

/// A dictionary-like data structured that can be stored in a CASA table.
///
/// FIXME: currently, to avoid potential double-frees, TableRecords are only
/// created standalone, or as copies of TableRecords owned by something else.
///
/// Ideally, there would be a lot of utility in having rewritable TableRecords,
/// however this would require keeping track of whether a TableRecord is owned,
/// and not freeing owned TableRecords.
///
/// Further discussion [here][x].
///
/// [x]: https://github.com/pkgw/rubbl/pull/181#issuecomment-968493738
#[derive(Clone)]
pub struct TableRecord {
    handle: *mut glue::GlueTableRecord,
    exc_info: glue::ExcInfo,
}

impl TableRecord {
    /// Create a new, empty [`TableRecord`].
    ///
    /// This function will return an error if the underlying C++ code raises an
    /// exception. This *should* only happen if there is a memory allocation
    /// error.
    pub fn new() -> Result<Self, TableError> {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::tablerec_create(&mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Self { handle, exc_info })
    }

    fn copy_handle(other_handle: &glue::GlueTableRecord) -> Result<Self, CasacoreError> {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::tablerec_copy(other_handle, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Self { handle, exc_info })
    }

    /// Return all of the keyword names in the record.
    pub fn keyword_names(&mut self) -> Result<Vec<String>, CasacoreError> {
        let mut result = Vec::new();

        let rv = unsafe {
            invoke_tablerec_get_keyword_info(self.handle, &mut self.exc_info, |name, _| {
                result.push(name);
            })
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(result)
    }

    /// Get the record contents as a vector of field names, datatypes, and
    /// string representations.
    pub fn keyword_names_types_reprs(
        &mut self,
    ) -> Result<Vec<(String, GlueDataType, String)>, CasacoreError> {
        let mut result = Vec::new();

        let rv = unsafe {
            invoke_tablerec_get_keyword_repr(
                self.handle,
                &mut self.exc_info,
                |name, type_, repr| {
                    result.push((name, type_, repr));
                },
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(result)
    }

    /// Get the value of a particular field of this record.
    pub fn get_field<T: CasaDataType>(&mut self, col_name: &str) -> Result<T, TableError> {
        let ccol_name = glue::StringBridge::from_rust(col_name);
        let mut data_type = glue::GlueDataType::TpOther;
        let mut n_dim = 0;
        let mut dims = [0; 8];

        let rv = unsafe {
            glue::tablerec_get_field_info(
                self.handle,
                &ccol_name,
                &mut data_type,
                &mut n_dim,
                dims.as_mut_ptr(),
                &mut self.exc_info,
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(UnexpectedDataTypeError(T::DATA_TYPE, data_type).into());
        }

        let result = if data_type == glue::GlueDataType::TpString {
            let mut value = None;

            let rv = unsafe {
                invoke_tablerec_get_field_string(self.handle, &ccol_name, &mut self.exc_info, |v| {
                    value = Some(v);
                })
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_string_pass_through(value.unwrap())
        } else if data_type == glue::GlueDataType::TpArrayString {
            let mut result = Vec::new();

            let rv = unsafe {
                invoke_tablerec_get_field_string_array(
                    self.handle,
                    &ccol_name,
                    &mut self.exc_info,
                    |v| {
                        result.push(v);
                    },
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_stringvec_pass_through(result)
        } else if data_type == glue::GlueDataType::TpRecord {
            let result = TableRecord::new().unwrap();
            let rv = unsafe {
                glue::tablerec_get_field_subrecord(
                    self.handle,
                    &ccol_name,
                    result.handle,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            T::casatables_tablerec_pass_through(result)
        } else {
            let mut result = T::casatables_alloc(&dims[..n_dim as usize])?;

            let rv = unsafe {
                glue::tablerec_get_field(
                    self.handle,
                    &ccol_name,
                    result.casatables_uninit_as_mut_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe { T::casatables_assume_init(result) }
        };

        Ok(result)
    }

    /// Set the value of a particular field of this record.
    pub fn put_field<T: CasaDataType>(
        &mut self,
        field_name: &str,
        value: &T,
    ) -> Result<(), CasacoreError> {
        let cfield_name = glue::StringBridge::from_rust(field_name);
        let mut shape = Vec::new();

        value.casatables_put_shape(&mut shape);

        if T::DATA_TYPE == glue::GlueDataType::TpString {
            let as_string = T::casatables_string_pass_through_out(value);
            let glue_string = glue::StringBridge::from_rust(&as_string);

            let rv = unsafe {
                glue::tablerec_put_field(
                    self.handle,
                    &cfield_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    &glue_string as *const glue::StringBridge as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else if T::DATA_TYPE == glue::GlueDataType::TpArrayString {
            let glue_strings = T::casatables_stringvec_pass_through_out(value);

            let rv = unsafe {
                glue::tablerec_put_field(
                    self.handle,
                    &cfield_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    glue_strings.as_ptr() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        } else {
            let rv = unsafe {
                glue::tablerec_put_field(
                    self.handle,
                    &cfield_name,
                    T::DATA_TYPE,
                    shape.len() as u64,
                    shape.as_ptr(),
                    value.casatables_as_buf() as _,
                    &mut self.exc_info,
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }
}

impl PartialEq for TableRecord {
    fn eq(&self, other: &Self) -> bool {
        unsafe { glue::tablerec_eq(self.handle, other.handle) }
    }
}

impl Eq for TableRecord {}

impl Debug for TableRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // This is to avoid mutably borrowing self
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };
        write!(f, "TableRecord {{ ").unwrap();
        unsafe {
            invoke_tablerec_get_keyword_repr(self.handle, &mut exc_info, |name, type_, repr| {
                write!(f, "{name}:{type_} = {repr}, ").unwrap();
            })
        };
        write!(f, " }}")
    }
}

impl CasaDataType for TableRecord {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpRecord;

    // Quasi-hack: In this framework, the "maybe uninit" form of the data is
    // never truly uninitialized; it's still a valid, allocated record structure
    // pointer. It just hasn't been populated yet. So it's OK for this to be a
    // passthrough type. In a certain sense it would be a bit more appropriate
    // to special-case this type in all of the generic functions as appropriate,
    // but we still are passing through a pointer, so we can reuse the generic
    // machinery.
    type MaybeUninit = Self;

    fn casatables_alloc(_shape: &[u64]) -> Result<Self::MaybeUninit, TableError> {
        TableRecord::new()
    }

    unsafe fn casatables_assume_init(buf: Self::MaybeUninit) -> Self {
        buf
    }

    fn casatables_as_buf(&self) -> *const () {
        self.handle as _
    }

    fn casatables_tablerec_pass_through(r: TableRecord) -> Self {
        r
    }

    fn casatables_tablerec_pass_through_out(s: &Self) -> TableRecord {
        s.to_owned()
    }
}

impl UninitedCasaData for TableRecord {
    fn casatables_uninit_as_mut_ptr(&mut self) -> *mut () {
        self.handle as _
    }
}

impl Drop for TableRecord {
    /// Free the casacore::TableRecord handle
    fn drop(&mut self) {
        // TODO: we want to free standalone tablerecords, but not those which
        // are associated with a table, because they are freed automatically
        // with the table. Not sure the best way to do this.

        // FIXME: not sure if this function can actually produce useful
        // exceptions anyway, but we can't do anything if it does!
        unsafe {
            glue::tablerec_free(self.handle, &mut self.exc_info);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;

    use super::*;
    use crate::glue::{GlueDataType, TableDescCreateMode};
    use ndarray::array;
    use tempfile::tempdir;

    #[allow(non_camel_case_types)]
    type c64 = Complex<f64>;

    #[test]
    fn table_create_with_scalar_desc() {
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");

        let col_name = "test_uint";

        let mut table_desc = TableDesc::new("TEST", TableDescCreateMode::TDM_SCRATCH).unwrap();
        table_desc
            .add_scalar_column(GlueDataType::TpUInt, col_name, None, false, false)
            .unwrap();

        let mut table = Table::new(table_path, table_desc, 123, TableCreateMode::New).unwrap();

        assert_eq!(table.n_rows(), 123);
        assert_eq!(table.n_columns(), 1);

        let column_info = table.get_col_desc(col_name).unwrap();
        assert_eq!(column_info.data_type(), GlueDataType::TpUInt);
        assert_eq!(column_info.name(), col_name);
        assert!(column_info.is_scalar());
    }

    #[test]
    fn table_create_with_scalar_string_desc() {
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");

        let col_name = "test_string";

        let mut table_desc = TableDesc::new("TEST", TableDescCreateMode::TDM_SCRATCH).unwrap();
        table_desc
            .add_scalar_column(GlueDataType::TpString, col_name, None, false, false)
            .unwrap();

        let mut table = Table::new(table_path, table_desc, 123, TableCreateMode::New).unwrap();

        assert_eq!(table.n_rows(), 123);
        assert_eq!(table.n_columns(), 1);

        let column_info = table.get_col_desc(col_name).unwrap();
        assert_eq!(column_info.data_type(), GlueDataType::TpString);
        assert_eq!(column_info.name(), col_name);
        assert!(column_info.is_scalar());
    }

    #[test]
    fn table_create_no_replace() {
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");

        // touch the file
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(table_path.clone())
            .unwrap();

        let col_name = "test_string";

        let mut table_desc = TableDesc::new("TEST", TableDescCreateMode::TDM_SCRATCH).unwrap();
        table_desc
            .add_scalar_column(GlueDataType::TpString, col_name, None, false, false)
            .unwrap();

        // NewNoReplace should fail if table exists.
        assert!(matches!(
            Table::new(table_path, table_desc, 123, TableCreateMode::NewNoReplace),
            Err(TableError::Casacore { .. })
        ));
    }

    #[test]
    fn table_create_with_fixed_string_array_desc() {
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");

        let col_name = "test_string_fixed";

        let mut table_desc = TableDesc::new("TEST", TableDescCreateMode::TDM_SCRATCH).unwrap();
        table_desc
            .add_array_column(
                GlueDataType::TpString,
                col_name,
                None,
                Some(&[1, 2, 3]),
                false,
                false,
            )
            .unwrap();

        let mut table = Table::new(table_path, table_desc, 123, TableCreateMode::New).unwrap();

        assert_eq!(table.n_rows(), 123);
        assert_eq!(table.n_columns(), 1);

        let column_info = table.get_col_desc(col_name).unwrap();
        assert_eq!(column_info.data_type(), GlueDataType::TpString);
        assert_eq!(column_info.name(), col_name);
        assert!(!column_info.is_scalar());
        assert!(column_info.is_fixed_shape());
        assert_eq!(column_info.shape(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn table_create_with_variable_string_array_desc() {
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");

        let col_name = "test_string_var";

        let mut table_desc = TableDesc::new("TEST", TableDescCreateMode::TDM_SCRATCH).unwrap();
        table_desc
            .add_array_column(GlueDataType::TpString, col_name, None, None, false, false)
            .unwrap();

        let mut table = Table::new(table_path, table_desc, 123, TableCreateMode::New).unwrap();

        assert_eq!(table.n_rows(), 123);
        assert_eq!(table.n_columns(), 1);

        let column_info = table.get_col_desc(col_name).unwrap();
        assert_eq!(column_info.data_type(), GlueDataType::TpString);
        assert_eq!(column_info.name(), col_name);
        assert!(!column_info.is_scalar());
        assert!(!column_info.is_fixed_shape());
        assert_eq!(column_info.shape(), None);
    }

    #[test]
    fn table_create_write_open_read_cell() {
        // tempdir is only necessary to avoid writing to disk each time this example is run
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        table_desc
            .add_array_column(
                GlueDataType::TpDouble,
                "UVW",
                Some("Vector with uvw coordinates (in meters)"),
                Some(&[3]),
                true,
                false,
            )
            .unwrap();
        // Create your new table with 1 rows
        let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
        // write to the first row in the uvw column
        let cell_value: Vec<f64> = vec![1.0, 2.0, 3.0];
        table.put_cell("UVW", 0, &cell_value).unwrap();
        // This writes the table to disk and closes the file pointer.
        drop(table);

        // now open the table again for reading cells
        let mut table = Table::open(&table_path, TableOpenMode::Read).unwrap();
        // and extract the cell value we wrote earlier
        let extracted_cell_value: Vec<f64> = table.get_cell_as_vec("UVW", 0).unwrap();
        assert_eq!(cell_value, extracted_cell_value);
    }

    #[test]
    fn table_add_scalar_column() {
        // tempdir is only necessary to avoid writing to disk each time this example is run
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        table_desc
            .add_array_column(
                GlueDataType::TpDouble,
                "UVW",
                Some("Vector with uvw coordinates (in meters)"),
                Some(&[3]),
                true,
                false,
            )
            .unwrap();
        // Create your new table with 1 rows
        let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
        // write to the first row in the uvw column
        let cell_value: Vec<f64> = vec![1.0, 2.0, 3.0];
        table.put_cell("UVW", 0, &cell_value).unwrap();
        // This writes the table to disk and closes the file pointer.
        drop(table);

        // now open the table again for adding the column
        let mut table = Table::open(&table_path, TableOpenMode::ReadWrite).unwrap();
        table
            .add_scalar_column(
                GlueDataType::TpInt,
                "second",
                Some("comment2"),
                false,
                false,
            )
            .unwrap();

        let col_names = table.column_names().unwrap();
        assert!(col_names.len() == 2);
    }

    #[test]
    fn table_add_array_column() {
        // tempdir is only necessary to avoid writing to disk each time this example is run
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        table_desc
            .add_array_column(
                GlueDataType::TpDouble,
                "UVW",
                Some("Vector with uvw coordinates (in meters)"),
                Some(&[3]),
                true,
                false,
            )
            .unwrap();
        // Create your new table with 1 rows
        let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
        // write to the first row in the uvw column
        let cell_value: Vec<f64> = vec![1.0, 2.0, 3.0];
        table.put_cell("UVW", 0, &cell_value).unwrap();
        // This writes the table to disk and closes the file pointer.
        drop(table);

        // now open the table again for adding the column
        let mut table = Table::open(&table_path, TableOpenMode::ReadWrite).unwrap();
        table
            .add_array_column(
                GlueDataType::TpDComplex,
                "second",
                Some("comment2"),
                Some(&[4]),
                false,
                false,
            )
            .unwrap();
        let cell_value: Vec<Complex<f64>> = vec![
            c64::new(1.0, 2.0),
            c64::new(3.0, 4.0),
            c64::new(5.0, 6.0),
            c64::new(7.0, 8.0),
        ];
        table.put_cell("second", 0, &cell_value).unwrap();

        let col_names = table.column_names().unwrap();
        assert!(col_names.len() == 2);
    }

    #[test]
    fn table_add_multi_dimensional_array_column() {
        // tempdir is only necessary to avoid writing to disk each time this example is run
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        table_desc
            .add_array_column(
                GlueDataType::TpDouble,
                "UVW",
                Some("Vector with uvw coordinates (in meters)"),
                Some(&[3]),
                true,
                false,
            )
            .unwrap();
        // Create your new table with 1 rows
        let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
        // write to the first row in the uvw column
        let cell_value: Vec<f64> = vec![1.0, 2.0, 3.0];
        table.put_cell("UVW", 0, &cell_value).unwrap();
        // This writes the table to disk and closes the file pointer.
        drop(table);

        // now open the table again for adding the column
        let mut table = Table::open(&table_path, TableOpenMode::ReadWrite).unwrap();
        let data_shape = [2, 4, 1];
        table
            .add_array_column(
                GlueDataType::TpDComplex,
                "DATA",
                None,
                Some(&data_shape),
                false,
                false,
            )
            .unwrap();

        let cell_value = array![
            [
                [c64::new(1.0, 2.0)],
                [c64::new(-1.0, -2.0)],
                [c64::new(3.0, 4.0)],
                [c64::new(-3.0, -4.0)]
            ],
            [
                [c64::new(5.0, 6.0)],
                [c64::new(-5.0, -6.0)],
                [c64::new(7.0, 8.0)],
                [c64::new(-7.0, -8.0)]
            ]
        ];
        table.put_cell("DATA", 0, &cell_value).unwrap();

        drop(table);

        let mut table = Table::open(&table_path, TableOpenMode::Read).unwrap();
        let uvw_tabledesc = table.get_col_desc("UVW").unwrap();
        let data_tabledesc = table.get_col_desc("DATA").unwrap();
        assert_eq!(uvw_tabledesc.data_type(), GlueDataType::TpDouble);
        assert_eq!(data_tabledesc.data_type(), GlueDataType::TpDComplex);
        assert!(uvw_tabledesc.is_fixed_shape());
        assert!(data_tabledesc.is_fixed_shape());
        assert!(!uvw_tabledesc.is_scalar());
        assert!(!data_tabledesc.is_scalar());
        assert_eq!(data_tabledesc.shape().unwrap(), data_shape);
        assert_eq!(uvw_tabledesc.shape().unwrap(), &[3]);
    }

    #[test]
    fn table_add_ndim_1_string_array_column() {
        // tempdir is only necessary to avoid writing to disk each time this example is run
        let tmp_dir = tempdir().unwrap();
        let table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        table_desc
            .add_array_column(
                GlueDataType::TpString,
                "APP_PARAMS",
                Some("Application parameters"),
                None,
                false,
                false,
            )
            .unwrap();
        table_desc.set_ndims("APP_PARAMS", 1).unwrap();
        // Create your new table with 1 rows
        let mut table = Table::new(&table_path, table_desc, 1, TableCreateMode::New).unwrap();
        // write to the first row in the uvw column
        let cell_value: Vec<String> = vec!["app params".to_string()];
        table.put_cell("APP_PARAMS", 0, &cell_value).unwrap();
        // This writes the table to disk and closes the file pointer.
        drop(table);

        let mut table = Table::open(&table_path, TableOpenMode::Read).unwrap();
        let aparams_tabledesc = table.get_col_desc("APP_PARAMS").unwrap();
        assert_eq!(aparams_tabledesc.data_type(), GlueDataType::TpString);
        assert_eq!(aparams_tabledesc.shape(), None);
        assert!(!aparams_tabledesc.is_fixed_shape());
        assert!(!aparams_tabledesc.is_scalar());

        let cell_value_read: Vec<String> = table.get_cell_as_vec("APP_PARAMS", 0).unwrap();
        assert_eq!(cell_value_read, cell_value);
    }

    #[test]
    pub fn table_put_table_keyword() {
        let tmp_dir = tempdir().unwrap();
        let root_table_path = tmp_dir.path().join("test.ms");
        let sub_table_path = root_table_path.join("SUB");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let mut root_table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        // Define the columns in your table description
        root_table_desc
            .add_array_column(
                GlueDataType::TpDouble,
                "UVW",
                Some("Vector with uvw coordinates (in meters)"),
                Some(&[3]),
                true,
                false,
            )
            .unwrap();
        // Create your new table with 1 rows
        let mut root_table =
            Table::new(&root_table_path, root_table_desc, 1, TableCreateMode::New).unwrap();

        let mut sub_table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        sub_table_desc
            .add_scalar_column(GlueDataType::TpInt, "int", None, true, false)
            .unwrap();

        let sub_table =
            Table::new(&sub_table_path, sub_table_desc, 1, TableCreateMode::New).unwrap();
        root_table.put_table_keyword("SUB", sub_table).unwrap();

        assert_eq!(root_table.table_keyword_names().unwrap(), ["SUB"]);
    }

    #[test]
    pub fn tabledesc_put_frequency_meas_desc() {
        let tmp_dir = tempdir().unwrap();
        let root_table_path = tmp_dir.path().join("test.ms");
        let source_table_path = root_table_path.join("SOURCE");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let root_table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        Table::new(&root_table_path, root_table_desc, 1, TableCreateMode::New).unwrap();

        let mut source_table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        source_table_desc
            .add_array_column(
                glue::GlueDataType::TpDouble,
                "REST_FREQUENCY",
                None,
                None,
                false,
                false,
            )
            .unwrap();

        source_table_desc
            .put_column_keyword("REST_FREQUENCY", "QuantumUnits", &vec!["Hz".to_string()])
            .unwrap();

        let mut meas_info = TableRecord::new().unwrap();
        meas_info
            .put_field("type", &"frequency".to_string())
            .unwrap();
        meas_info.put_field("Ref", &"LSRK".to_string()).unwrap();

        source_table_desc
            .put_column_keyword("REST_FREQUENCY", "MEASINFO", &meas_info)
            .unwrap();

        let mut source_table = Table::new(
            &source_table_path,
            source_table_desc,
            1,
            TableCreateMode::New,
        )
        .unwrap();

        let mut result_tablerec = source_table
            .get_column_keyword_record("REST_FREQUENCY")
            .unwrap();
        let keywords = result_tablerec.keyword_names().unwrap();
        assert_eq!(keywords, vec!["QuantumUnits", "MEASINFO"]);
        let col_keywords = source_table.column_keyword_names("REST_FREQUENCY").unwrap();
        assert_eq!(keywords, col_keywords);

        let units: Vec<String> = result_tablerec.get_field("QuantumUnits").unwrap();
        assert_eq!(units, ["Hz"]);

        let mut result_meas_info: TableRecord = result_tablerec.get_field("MEASINFO").unwrap();

        let result_meas_info_keywords = result_meas_info.keyword_names().unwrap();
        assert_eq!(result_meas_info_keywords, vec!["type", "Ref"]);

        let meas_type: String = result_meas_info.get_field("type").unwrap();
        assert_eq!(meas_type, "frequency");
        let meas_ref: String = result_meas_info.get_field("Ref").unwrap();
        assert_eq!(meas_ref, "LSRK");
    }

    #[test]
    pub fn tablerec_equality() {
        let mut rec1 = TableRecord::new().unwrap();
        rec1.put_field("field1", &"value1".to_string()).unwrap();
        let mut rec2 = TableRecord::new().unwrap();
        rec2.put_field("field1", &"value1".to_string()).unwrap();
        assert_eq!(rec1, rec2);

        rec2.put_field("field2", &"value2".to_string()).unwrap();
        assert_ne!(rec1, rec2);
    }

    #[test]
    pub fn table_debug() {
        let tmp_dir = tempdir().unwrap();
        let root_table_path = tmp_dir.path().join("test.ms");
        // First create a table description for our base table.
        // Use TDM_SCRATCH to avoid writing the .tabdsc to disk.
        let root_table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();
        let root_table =
            Table::new(&root_table_path, root_table_desc, 1, TableCreateMode::New).unwrap();

        assert_eq!(
            root_table.file_name().unwrap(),
            root_table_path.to_str().unwrap()
        );

        let table_debug = format!("{root_table:?}");

        assert!(table_debug.contains(root_table_path.to_str().unwrap()));
    }
}
