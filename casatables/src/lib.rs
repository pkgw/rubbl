// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

#[macro_use] extern crate error_chain;
extern crate rubbl_core;
extern crate rubbl_casatables_impl;

use rubbl_core::Complex;
use std::fmt::Display;
use std::path::Path;

#[macro_use] pub mod errors; // most come first to provide macros for other modules
use errors::{Error, ErrorKind, Result};

mod glue;

/// OMG. Strings were incredibly painful.
///
/// One important piece of context: `casacore::String` is a subclass of C++'s
/// `std::string`. Rust strings can contain interior NUL bytes. Fortunately,
/// `std::string` can as well, so we don't need to worry about the C string
/// convention.
///
/// My understanding is that C++'s `std::string` always allocates its own
/// buffer. So we can't try to be clever about lifetimes and borrowing: every
/// time we bridge to C++ there's going to be a copy.
///
/// Then I ran into problems essentially because of the following bindgen
/// problem: https://github.com/rust-lang-nursery/rust-bindgen/issues/778 . On
/// Linux small classes, such as String, have special ABI conventions, and
/// bindgen does not represent them properly to Rust at the moment (Sep 2017).
/// The String class is a victim of this problem, which led to completely
/// bizarre failures in my code when the small-string optimization was kicking
/// in. It seems that if we only interact with the C++ through pointers and
/// references to Strings, things remain OK.
///
/// Finally, as best I understand it, we need to manually ensure that the C++
/// destructor for the String class is run. I have done this with a little
/// trick off of StackExchange.

impl glue::GlueString {
    fn from_rust(s: &str) -> Self {
        unsafe {
            let mut cs = ::std::mem::zeroed::<glue::GlueString>();
            glue::string_init(&mut cs, s.as_ptr() as _, s.len() as u64);
            cs
        }
    }

    fn to_rust(&self) -> String {
        let mut ptr: *const ::std::os::raw::c_void = 0 as _;
        let mut n_bytes: u64 = 0;

        let buf = unsafe {
            glue::string_get_buf(self, &mut ptr, &mut n_bytes);
            ::std::slice::from_raw_parts(ptr as *const u8, n_bytes as usize)
        };

        String::from_utf8_lossy(buf).into_owned()
    }
}

impl Drop for glue::GlueString {
    fn drop(&mut self) {
        unsafe { glue::string_deinit(self) };
    }
}

// Exceptions

impl glue::ExcInfo {
    fn as_error(&self) -> Error {
        let c_str = unsafe { ::std::ffi::CStr::from_ptr(self.message.as_ptr()) };

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


/// A type that can be translated into a CASA table data type.
pub trait CasaDataType: Clone + Display + PartialEq + Sized {
    const DATA_TYPE: glue::GlueDataType;

    #[cfg(test)]
    fn test_casa_data_size() {
        assert_eq!(std::mem::size_of::<Self>() as i32, Self::DATA_TYPE.element_size());
    }
}


impl CasaDataType for bool {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpBool;
}

impl CasaDataType for i8 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpChar;
}

impl CasaDataType for u8 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpUChar;
}

impl CasaDataType for i16 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpShort;
}

impl CasaDataType for u16 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpUShort;
}

impl CasaDataType for i32 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpInt;
}

impl CasaDataType for u32 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpUInt;
}

impl CasaDataType for i64 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpInt64;
}

impl CasaDataType for f32 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpFloat;
}

impl CasaDataType for f64 {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpDouble;
}

impl CasaDataType for Complex<f32> {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpComplex;
}

impl CasaDataType for Complex<f64> {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpDComplex;
}

impl CasaDataType for String {
    const DATA_TYPE: glue::GlueDataType = glue::GlueDataType::TpString;
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


// Tables

pub struct Table {
    handle: *mut glue::GlueTable,
    exc_info: glue::ExcInfo,
}

impl Table {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let spath = match path.as_ref().to_str() {
            Some(s) => s,
            None => return Err("table paths must be representable as UTF-8 strings".into()),
        };
        let cpath = glue::GlueString::from_rust(spath);
        let mut exc_info = unsafe { ::std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_alloc_and_open(&cpath, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Table {
            handle: handle,
            exc_info: exc_info,
        })
    }

    pub fn n_rows(&self) -> usize {
        unsafe { glue::table_n_rows(self.handle) as usize }
    }

    pub fn n_columns(&self) -> usize {
        unsafe { glue::table_n_columns(self.handle) as usize }
    }

    pub fn column_names(&mut self) -> Result<Vec<String>> {
        let n_cols = self.n_columns();
        let mut cnames: Vec<glue::GlueString> = Vec::with_capacity(n_cols);

        for _ in 0..n_cols {
            cnames.push(glue::GlueString::from_rust(""));
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

        unsafe { cnames.set_len(n_cols); }

        Ok(cnames.iter().map(|cstr| cstr.to_rust()).collect())
    }

    pub fn get_col_desc(&mut self, col_name: &str) -> Result<ColumnDescription> {
        let ccol_name = glue::GlueString::from_rust(col_name);
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
                v.push(*d as u32);
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

    pub fn get_col_as_vec<T: CasaDataType>(&mut self, col_name: &str) -> Result<Vec<T>> {
        let ccol_name = glue::GlueString::from_rust(col_name);
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

        let mut result = Vec::with_capacity(n_rows as usize);

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

        Ok(result)
    }

    pub fn deep_copy_no_rows(&mut self, dest_path: &str) -> Result<()> {
        let cdest_path = glue::GlueString::from_rust(dest_path);

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
    shape: Option<Vec<u32>>,
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

    pub fn shape(&self) -> Option<&[u32]> {
        self.shape.as_ref().map(|v| &v[..])
    }
}


#[cfg(test)]
mod tests {
    use super::glue;

    #[test]
    fn check_string_size() {
        let cpp_size = unsafe { glue::string_check_size() } as usize;
        let rust_size = ::std::mem::size_of::<glue::GlueString>();
        assert_eq!(cpp_size, rust_size);
    }
}
