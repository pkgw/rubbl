// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

extern crate ndarray;
#[macro_use] extern crate error_chain;
extern crate rubbl_core;
extern crate rubbl_casatables_impl;

use ndarray::Dimension;
use rubbl_core::{Array, Complex};
use rubbl_core::errors::ErrorKind as CoreErrorKind;
use rubbl_core::num::DimFromShapeSlice;
use std::fmt;
use std::path::Path;

#[macro_use] pub mod errors; // most come first to provide macros for other modules
use errors::{Error, ErrorKind, Result};

mod glue;

// Exceptions

impl glue::ExcInfo {
    fn as_error(&self) -> Error {
        let c_str = unsafe { std::ffi::CStr::from_ptr(self.message.as_ptr()) };

        let msg = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => "[un-translatable C++ exception]",
        };

        ErrorKind::CasacoreException(msg.to_owned()).into()
    }

    fn as_err<T>(&self) -> Result<T> {
        Err(self.as_error())
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
        f.pad(match self {
            &glue::GlueDataType::TpBool => "bool",
            &glue::GlueDataType::TpChar => "i8",
            &glue::GlueDataType::TpUChar => "u8",
            &glue::GlueDataType::TpShort => "i16",
            &glue::GlueDataType::TpUShort => "u16",
            &glue::GlueDataType::TpInt => "i32",
            &glue::GlueDataType::TpUInt => "u32",
            &glue::GlueDataType::TpFloat => "f32",
            &glue::GlueDataType::TpDouble => "f64",
            &glue::GlueDataType::TpComplex => "c32",
            &glue::GlueDataType::TpDComplex => "c64",
            &glue::GlueDataType::TpString => "string",
            &glue::GlueDataType::TpTable => "table",
            &glue::GlueDataType::TpArrayBool => "arr<bool>",
            &glue::GlueDataType::TpArrayChar => "arr<i8>",
            &glue::GlueDataType::TpArrayUChar => "arr<u8>",
            &glue::GlueDataType::TpArrayShort => "arr<i16>",
            &glue::GlueDataType::TpArrayUShort => "arr<u16>",
            &glue::GlueDataType::TpArrayInt => "arr<i32>",
            &glue::GlueDataType::TpArrayUInt => "arr<u32>",
            &glue::GlueDataType::TpArrayFloat => "arr<f32>",
            &glue::GlueDataType::TpArrayDouble => "arr<f64>",
            &glue::GlueDataType::TpArrayComplex => "arr<c32>",
            &glue::GlueDataType::TpArrayDComplex => "arr<c64>",
            &glue::GlueDataType::TpArrayString => "arr<string>",
            &glue::GlueDataType::TpRecord => "record",
            &glue::GlueDataType::TpOther => "other",
            &glue::GlueDataType::TpQuantity => "quantity",
            &glue::GlueDataType::TpArrayQuantity => "arr<quantity>",
            &glue::GlueDataType::TpInt64 => "i64",
            &glue::GlueDataType::TpArrayInt64 => "arr<i64>",
        })
    }
}

/// A type that can be translated into a CASA table data type.
pub trait CasaDataType: Clone + PartialEq + Sized {
    const DATA_TYPE: glue::GlueDataType;

    #[cfg(test)]
    fn test_casa_data_size() {
        assert_eq!(std::mem::size_of::<Self>() as i32, Self::DATA_TYPE.element_size());
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

    /// A hack that lets us properly special-case string vectors
    #[doc(hidden)]
    fn casatables_stringvec_pass_through_out(_s: &Self) -> Vec<glue::StringBridge> {
        unreachable!();
    }

    /// Defaut behavior: fill the dest with a zero shape, i.e. report that we're a scalar.
    #[doc(hidden)]
    fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
        shape_dest.truncate(0);
    }

    #[doc(hidden)]
    fn casatables_alloc(shape: &[u64]) -> Result<Self>;

    #[doc(hidden)]
    fn casatables_as_buf(&self) -> *const () {
        self as *const Self as _
    }

    #[doc(hidden)]
    fn casatables_as_mut_buf(&mut self) -> *mut () {
        self as *mut Self as _
    }
}


/// A type that maps to one of CASA's scalar data types.
pub trait CasaScalarData: CasaDataType {
    const VECTOR_TYPE: glue::GlueDataType;
}

macro_rules! impl_scalar_data_type {
    ($rust_type:ty, $casa_scalar_type:ident, $casa_vector_type:ident, $default:expr) => {
        impl CasaDataType for $rust_type {
            const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::$casa_scalar_type;

            fn casatables_alloc(_shape: &[u64]) -> Result<Self> {
                Ok($default)
            }
        }

        impl CasaScalarData for $rust_type {
            const VECTOR_TYPE: glue::GlueDataType = glue::GlueDataType::$casa_vector_type;
        }
    }
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

    fn casatables_alloc(_shape: &[u64]) -> Result<Self> {
        Ok("".to_owned())
    }

    fn casatables_as_buf(&self) -> *const () {
        panic!("disallowed for string values")
    }

    fn casatables_as_mut_buf(&mut self) -> *mut () {
        panic!("disallowed for string values")
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

            fn casatables_alloc(shape: &[u64]) -> Result<Self> {
                if shape.len() != 1 {
                    Err(CoreErrorKind::DimensionMismatch(1, shape.len()).into())
                } else {
                    let mut rv = Vec::with_capacity(shape[0] as usize);
                    unsafe { rv.set_len(shape[0] as usize); }
                    Ok(rv)
                }
            }

            fn casatables_put_shape(&self, shape_dest: &mut Vec<u64>) {
                shape_dest.truncate(0);
                shape_dest.push(self.len() as u64);
            }

            fn casatables_as_buf(&self) -> *const () {
                self.as_ptr() as _
            }

            fn casatables_as_mut_buf(&mut self) -> *mut () {
                self.as_mut_ptr() as _
            }
        }
    }
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

    fn casatables_alloc(shape: &[u64]) -> Result<Self> {
        if shape.len() != 1 {
            Err(CoreErrorKind::DimensionMismatch(1, shape.len()).into())
        } else {
            let mut rv = Vec::with_capacity(shape[0] as usize);
            unsafe { rv.set_len(shape[0] as usize); }
            Ok(rv)
        }
    }

    fn casatables_stringvec_pass_through_out(svec: &Self) -> Vec<glue::StringBridge> {
        svec.iter().map(|s| glue::StringBridge::from_rust(s)).collect()
    }

    fn casatables_as_buf(&self) -> *const () {
        self.as_ptr() as _
    }

    fn casatables_as_mut_buf(&mut self) -> *mut () {
        self.as_mut_ptr() as _
    }
}


// Blanket implementation of n-dimensional array mappings.
impl<I: CasaScalarData + Copy, D: Dimension + DimFromShapeSlice<u64>> CasaDataType for Array<I, D> {
    const DATA_TYPE: glue::GlueDataType = I::VECTOR_TYPE;

    fn casatables_alloc(shape: &[u64]) -> Result<Self> {
        Ok(unsafe { Self::uninitialized(D::from_shape_slice(shape)?) })
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

    fn casatables_as_mut_buf(&mut self) -> *mut () {
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
    fn zeroed() -> Self {
        Self {
            data: 0 as _,
            n_bytes: 0,
        }
    }

    fn from_rust(s: &str) -> Self {
        Self {
            data: s.as_ptr() as _,
            n_bytes: s.len() as std::os::raw::c_ulong,
        }
    }

    fn to_rust(&self) -> String {
        let buf = unsafe {
            std::slice::from_raw_parts(self.data as *const u8, self.n_bytes as usize)
        };

        String::from_utf8_lossy(buf).into_owned()
    }
}


// Tables

pub struct Table {
    handle: *mut glue::GlueTable,
    exc_info: glue::ExcInfo,
}

pub enum TableOpenMode {
    Read = 1,
    ReadWrite = 2,
    Create = 3
}

impl Table {
    pub fn open<P: AsRef<Path>>(path: P, mode: TableOpenMode) -> Result<Self> {
        let spath = match path.as_ref().to_str() {
            Some(s) => s,
            None => return Err("table paths must be representable as UTF-8 strings".into()),
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

        Ok(Table {
            handle: handle,
            exc_info: exc_info,
        })
    }

    pub fn n_rows(&self) -> u64 {
        unsafe { glue::table_n_rows(self.handle) as u64 }
    }

    pub fn n_columns(&self) -> usize {
        unsafe { glue::table_n_columns(self.handle) as usize }
    }

    pub fn column_names(&mut self) -> Result<Vec<String>> {
        let n_cols = self.n_columns();
        let mut cnames: Vec<glue::StringBridge> = Vec::with_capacity(n_cols);

        for _ in 0..n_cols {
            cnames.push(glue::StringBridge::zeroed());
        }

        let rv = unsafe {
            glue::table_get_column_names(
                self.handle,
                cnames.as_mut_ptr(),
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(cnames.iter().map(|cstr| cstr.to_rust()).collect())
    }

    pub fn remove_column(&mut self, col_name: &str) -> Result<()> {
        let ccol_name = glue::StringBridge::from_rust(col_name);

        let rv = unsafe {
            glue::table_remove_column(
                self.handle,
                &ccol_name,
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    pub fn table_keyword_names(&mut self) -> Result<Vec<String>> {
        // Oh man. So, the C++ code behind this functionality reports back a
        // sequence of casa::String (<=> std::string) objects, but they are
        // only temporary, so for each item we need to make a copy of its
        // contents right then and there. Therefore we need a callback. It
        // took me a long time to get this right but I think it's working now.
        // We need an "extern C fn" callback that the C code can call
        // explicitly, which then hands off to the actual Rust closure of
        // interest.

        unsafe extern "C" fn casatables_cb_table_keyword_names<F>(
            name: *const glue::StringBridge,
            dtype: glue::GlueDataType,
            ctxt: *mut std::os::raw::c_void
        )
            where F: FnMut(String, glue::GlueDataType)
        {
            let f: &mut F = &mut *(ctxt as *mut F);
            f((&*name).to_rust(), dtype)
        }

        // I think we need this wrapper to give us a way to name the type `F`
        // of our closure.

        unsafe fn invoke<F>(handle: *mut glue::GlueTable, exc_info: &mut glue::ExcInfo,
                            mut f: F) -> std::os::raw::c_int
            where F: FnMut(String, glue::GlueDataType)
        {
            glue::table_get_keyword_info(
                handle,
                Some(casatables_cb_table_keyword_names::<F>),
                &mut f as *mut _ as *mut std::os::raw::c_void,
                exc_info
            )
        }

        // Here's where we actually do stuff.

        let mut result = Vec::new();

        let rv = unsafe { invoke(self.handle, &mut self.exc_info, |name, dtype| {
            if dtype == glue::GlueDataType::TpTable {
                result.push(name);
            }
        }) };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(result)
    }

    pub fn get_col_desc(&mut self, col_name: &str) -> Result<ColumnDescription> {
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
                &mut self.exc_info
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
                v.push(*d as u64);
            }

            Some(v)
        };

        Ok(ColumnDescription {
            name: col_name.to_owned(),
            data_type: data_type,
            is_scalar: is_scalar != 0,
            is_fixed_shape: is_fixed_shape != 0,
            shape: shape,
        })
    }

    pub fn get_col_as_vec<T: CasaScalarData>(&mut self, col_name: &str) -> Result<Vec<T>> {
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
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if is_scalar == 0 || is_fixed_shape == 0 || n_dim != 0 {
            return Err(ErrorKind::NotScalarColumn.into());
        }

        if data_type != T::DATA_TYPE {
            return Err(ErrorKind::UnexpectedCasaType(data_type).into());
        }

        let mut result = Vec::<T>::with_capacity(n_rows as usize);

        if data_type != glue::GlueDataType::TpString {
            let rv = unsafe {
                glue::table_get_scalar_column_data(
                    self.handle,
                    &ccol_name,
                    result.as_mut_ptr() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe { result.set_len(n_rows as usize); }
        } else {
            // We are not given ownership of the String objects that are
            // returned, so we must std::mem::forget() them. Empirically, we
            // have to initialize our string structures.
            let mut glue_strings = Vec::<glue::StringBridge>::with_capacity(n_rows as usize);

            for _ in 0..n_rows as usize {
                glue_strings.push(glue::StringBridge::from_rust(""));
            }

            let rv = unsafe {
                glue::table_get_scalar_column_data(
                    self.handle,
                    &ccol_name,
                    glue_strings.as_mut_ptr() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            for cstr in glue_strings.into_iter() {
                result.push(T::casatables_string_pass_through(cstr.to_rust()));
                std::mem::forget(cstr);
            }
        };

        Ok(result)
    }

    pub fn get_cell<T: CasaDataType>(&mut self, col_name: &str, row: u64) -> Result<T> {
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
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(ErrorKind::UnexpectedCasaType(data_type).into());
        }

        let result = if data_type != glue::GlueDataType::TpString {
            let mut result = T::casatables_alloc(&dims[..n_dim as usize])?;

            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    result.casatables_as_mut_buf() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            result
        } else {
            // We are not given ownership of the String object that is
            // returned, so we must std::mem::forget() it.
            let mut glue_string = glue::StringBridge::from_rust("");

            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    &mut glue_string as *mut glue::StringBridge as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            let result = T::casatables_string_pass_through(glue_string.to_rust());
            std::mem::forget(glue_string);
            result
        };

        Ok(result)
    }

    /// This function discards shape information but won't accept scalars.
    pub fn get_cell_as_vec<T: CasaScalarData>(&mut self, col_name: &str, row: u64) -> Result<Vec<T>> {
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
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(ErrorKind::UnexpectedCasaType(data_type).into());
        }

        let n_items = dims[..n_dim as usize].iter().fold(1usize, |p, n| p * (*n as usize));

        let mut result = Vec::<T>::with_capacity(n_items);

        if data_type != glue::GlueDataType::TpString {
            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    result.as_mut_ptr() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            unsafe { result.set_len(n_items as usize); }
        } else {
            // We are not given ownership of the String objects that are
            // returned, so we must std::mem::forget() them.
            let mut glue_strings = Vec::<glue::StringBridge>::with_capacity(n_items as usize);

            for _ in 0..n_items as usize {
                glue_strings.push(glue::StringBridge::from_rust(""));
            }

            let rv = unsafe {
                glue::table_get_cell(
                    self.handle,
                    &ccol_name,
                    row,
                    glue_strings.as_mut_ptr() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            for cstr in glue_strings.into_iter() {
                result.push(T::casatables_string_pass_through(cstr.to_rust()));
                std::mem::forget(cstr);
            }
        }

        Ok(result)
    }

    pub fn put_cell<T: CasaDataType>(&mut self, col_name: &str, row: u64, value: &T) -> Result<()> {
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
                    &mut self.exc_info
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
                    &mut self.exc_info
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
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }


    pub fn add_rows(&mut self, n_rows: usize) -> Result<()> {
        if unsafe { glue::table_add_rows(self.handle, n_rows as u64, &mut self.exc_info) != 0 } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }


    pub fn get_row_writer(&mut self) -> Result<TableRow> {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_row_alloc(self.handle, 0, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(TableRow {
            handle: handle,
            exc_info: exc_info,
        })
    }


    pub fn for_each_row<F>(&mut self, mut func: F) -> Result<()>
        where F: FnMut(&mut TableRow) -> Result<()> {
        let mut exc_info = unsafe { std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_row_alloc(self.handle, 1, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        let mut row = TableRow {
            handle: handle,
            exc_info: exc_info,
        };

        for row_number in 0..self.n_rows() {
            if unsafe { glue::table_row_read(row.handle, row_number as u64, &mut row.exc_info) } != 0 {
                return row.exc_info.as_err();
            }

            func(&mut row)?;
        }

        Ok(())
    }

    pub fn copy_rows_to(&mut self, dest: &mut Table) -> Result<()> {
        if unsafe { glue::table_copy_rows(self.handle, dest.handle, &mut self.exc_info) != 0 } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }


    pub fn deep_copy_no_rows(&mut self, dest_path: &str) -> Result<()> {
        let cdest_path = glue::StringBridge::from_rust(dest_path);

        if unsafe { glue::table_deep_copy_no_rows(self.handle, &cdest_path, &mut self.exc_info) != 0 } {
            self.exc_info.as_err()
        } else {
            Ok(())
        }
    }
}


impl Drop for Table {
    fn drop(&mut self) {
        // FIXME: not sure if this function can actually produce useful
        // exceptions anyway, but we can't do anything if it does!
        unsafe { glue::table_close_and_free(self.handle, &mut self.exc_info) }
    }
}


pub struct ColumnDescription {
    name: String,
    data_type: glue::GlueDataType,
    is_scalar: bool,
    is_fixed_shape: bool,
    shape: Option<Vec<u64>>,
}

impl ColumnDescription {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn data_type(&self) -> glue::GlueDataType {
        self.data_type
    }

    pub fn is_scalar(&self) -> bool {
        self.is_scalar
    }

    pub fn is_fixed_shape(&self) -> bool {
        self.is_fixed_shape
    }

    pub fn shape(&self) -> Option<&[u64]> {
        self.shape.as_ref().map(|v| &v[..])
    }
}


// Table Row handles

pub struct TableRow {
    handle: *mut glue::GlueTableRow,
    exc_info: glue::ExcInfo,
}

impl TableRow {
    pub fn get_cell<T: CasaDataType>(&mut self, col_name: &str) -> Result<T> {
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
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        if data_type != T::DATA_TYPE {
            return Err(ErrorKind::UnexpectedCasaType(data_type).into());
        }

        let result = if data_type != glue::GlueDataType::TpString {
            let mut result = T::casatables_alloc(&dims[..n_dim as usize])?;

            let rv = unsafe {
                glue::table_row_get_cell(
                    self.handle,
                    &ccol_name,
                    result.casatables_as_mut_buf() as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            result
        } else {
            // We are not given ownership of the String object that is
            // returned, so we must std::mem::forget() it.
            let mut glue_string = glue::StringBridge::from_rust("");

            let rv = unsafe {
                glue::table_row_get_cell(
                    self.handle,
                    &ccol_name,
                    &mut glue_string as *mut glue::StringBridge as _,
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }

            let result = T::casatables_string_pass_through(glue_string.to_rust());
            std::mem::forget(glue_string);
            result
        };

        Ok(result)
    }

    pub fn put_cell<T: CasaDataType>(&mut self, col_name: &str, value: &T) -> Result<()> {
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
                    &mut self.exc_info
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
                    &mut self.exc_info
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
                    &mut self.exc_info
                )
            };

            if rv != 0 {
                return self.exc_info.as_err();
            }
        }

        Ok(())
    }

    pub fn copy_and_put(&mut self, dest: &mut TableRow, row_number: u64) -> Result<()> {
        let rv = unsafe {
            glue::table_row_copy_and_put(
                self.handle,
                row_number,
                dest.handle,
                &mut self.exc_info
            )
        };

        if rv != 0 {
            return self.exc_info.as_err();
        }

        Ok(())
    }

    pub fn put(&mut self, row_number: u64) -> Result<()> {
        let rv = unsafe {
            glue::table_row_write(
                self.handle,
                row_number,
                &mut self.exc_info
            )
        };

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
        unsafe { glue::table_row_free(self.handle, &mut self.exc_info); }
    }
}
