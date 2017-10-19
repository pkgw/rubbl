// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD-format data sets.

 */

#[macro_use] extern crate error_chain;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};


#[macro_use] pub mod errors; // must come first to provide macros for other modules
pub mod visdata;

use errors::Result;

/// The maximum length of the name of a dataset "item", in bytes.
pub const MAX_ITEM_NAME_LENGTH: usize = 8;


//pub trait IoBackend {
//    type Error: std::error::Error;
//    type Item: Read + Write;
//}


#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Binary = 0,
    Int8 = 1,
    Int32 = 2,
    Int16 = 3,
    Float32 = 4,
    Float64 = 5,
    Text = 6,
    Complex64 = 7,
    Int64 = 8,
}

impl Type {
    pub fn abbrev_char(&self) -> char {
        match self {
            &Type::Binary => '?',
            &Type::Int8 => 'b',
            &Type::Int16 => 'j',
            &Type::Int32 => 'i',
            &Type::Int64 => 'l',
            &Type::Float32 => 'r',
            &Type::Float64 => 'd',
            &Type::Complex64 => 'c',
            &Type::Text => 'a',
        }
    }

    pub fn size(&self) -> usize {
        match self {
            &Type::Binary => 1,
            &Type::Int8 => 1,
            &Type::Int16 => 2,
            &Type::Int32 => 4,
            &Type::Int64 => 8,
            &Type::Float32 => 4,
            &Type::Float64 => 8,
            &Type::Complex64 => 8,
            &Type::Text => 1,
        }
    }

    pub fn alignment(&self) -> u8 {
        match self {
            &Type::Binary => 1,
            &Type::Int8 => 1,
            &Type::Int16 => 2,
            &Type::Int32 => 4,
            &Type::Int64 => 8,
            &Type::Float32 => 4,
            &Type::Float64 => 8,
            &Type::Complex64 => 4, // this is the only surprising one
            &Type::Text => 1,
        }
    }
}


/// This trait marks that the given type maps onto an "elementary" type
/// defined in the MIRIAD data format. It is implemented by u8, i8, i16, i32,
/// i64, f32, and f64; string values are not included because of the
/// encoding/decoding issues that pop up.
pub trait MiriadElementaryType {
    /// The particular MIRIAD `Type` to which this Rust type maps.
    const TYPE: Type;
}

impl MiriadElementaryType for u8 {
    const TYPE: Type = Type::Binary;
}

impl MiriadElementaryType for i8 {
    const TYPE: Type = Type::Int8;
}

impl MiriadElementaryType for i16 {
    const TYPE: Type = Type::Int16;
}

impl MiriadElementaryType for i32 {
    const TYPE: Type = Type::Int32;
}

impl MiriadElementaryType for i64 {
    const TYPE: Type = Type::Int64;
}

impl MiriadElementaryType for f32 {
    const TYPE: Type = Type::Float32;
}

impl MiriadElementaryType for f64 {
    const TYPE: Type = Type::Float64;
}


/// This trait marks that the given type maps on to a vector type
/// defined in the MIRIAD data format.

pub trait MiriadVectorType: Sized {
    /// The particular MIRIAD `Type` to which this Rust type maps.
    const TYPE: Type;

    /// Each chunk is guaranteed to be a size that is a multiple of
    /// `TYPE.size()`.
    fn from_miriad_byte_chunks<'a, T: Iterator<Item = Result<&'a [u8]>>>(chunks: T) -> Result<Self>;

    fn from_miriad_bytes(buf: &[u8]) -> Result<Self> {
        Self::from_miriad_byte_chunks(std::iter::once(Ok(buf)))
    }
}

impl MiriadVectorType for Vec<u8> {
    const TYPE: Type = Type::Binary;

    fn from_miriad_byte_chunks<'a, T: Iterator<Item = Result<&'a [u8]>>>(chunks: T) -> Result<Self> {
        let mut val = Self::new();

        for maybe_chunk in chunks {
            val.extend_from_slice(maybe_chunk?);
        }

        Ok(val)
    }
}

impl MiriadVectorType for Vec<i8> {
    const TYPE: Type = Type::Int8;

    fn from_miriad_byte_chunks<'a, T: Iterator<Item = Result<&'a [u8]>>>(chunks: T) -> Result<Self> {
        let mut val = Self::new();

        for maybe_chunk in chunks {
            let bytes = maybe_chunk?;
            // XXX There Must Be A Better Way™
            let signeds = unsafe { std::mem::transmute::<&[u8], &[i8]>(bytes) };
            val.extend_from_slice(signeds);
        }

        Ok(val)
    }
}

//impl MiriadVectorType for Vec<i16> {
//    const TYPE: Type = Type::Int16;
//}
//
//impl MiriadVectorType for Vec<i32> {
//    const TYPE: Type = Type::Int32;
//}
//
//impl MiriadVectorType for Vec<i64> {
//    const TYPE: Type = Type::Int64;
//}
//
//impl MiriadVectorType for Vec<f32> {
//    const TYPE: Type = Type::Float32;
//}
//
//impl MiriadVectorType for Vec<f64> {
//    const TYPE: Type = Type::Float64;
//}
//
//impl MiriadVectorType for String {
//    const TYPE: Type = Type::Text;
//}

// XXX complex64


pub struct ItemInfo {
    pub name: String,
    pub is_large: bool,
    pub ty: Type,
    pub n_vals: usize,
}


#[repr(C)]
struct HeaderItem {
    /// The name of this item, encoded in UTF-8 with no trailing NUL. Classic
    /// MIRIAD is, of course, completely unaware of UTF-8, but it seems like a
    /// sensible extension.
    pub name: [u8; 15],

    /// The length of the item including the necessary padding for alignment.
    pub aligned_len: u8,
}


#[repr(C)]
struct SmallItem {
    pub name: String,
    pub ty: Type,
    pub nvals: u8,
    pub values: [u8; 64],
}

impl SmallItem {
    pub fn new<S: ToString>(name: S, ty: Type, nvals: u8) -> Self {
        SmallItem {
            name: name.to_string(),
            ty: ty,
            nvals: nvals,
            values: [0; 64],
        }
    }
}


pub struct DataSet {
    path: PathBuf,
    small_items: HashMap<String, SmallItem>
}


impl DataSet {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut ds = DataSet {
            path: path.as_ref().into(),
            small_items: HashMap::new(),
        };

        let mut header = ds.open_file_lowlevel("header")?;
        let mut rec: HeaderItem = HeaderItem { name: [0; 15], aligned_len: 0 };

        loop {
            // XXX There Must Be A Better Way™
            let r = {
                let rec_as_bytes: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(
                        &mut rec as *mut HeaderItem as *mut u8,
                        std::mem::size_of::<HeaderItem>()
                    )
                };
                header.read_exact(rec_as_bytes)
            };

            if let Err(e) = r {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    break;
                }

                return Err(e.into());
            }

            let name = std::str::from_utf8(&rec.name[..])?;
            println!("NAME: {}  LEN; {}", name, rec.aligned_len);
            header.seek(io::SeekFrom::Current(rec.aligned_len as i64))?;

            // TODO: could/should warn if a redundant item is encountered
            ds.small_items.insert(name.to_owned(), SmallItem::new(name, Type::Binary, 0));
        }

        Ok(ds)
    }


    fn open_file_lowlevel(&self, name: &str) -> Result<File> {
        // Note: not checking for length, validity, etc.
        let mut p = self.path.clone();
        p.push(name);
        Ok(File::open(p)?)
    }


    //pub fn read_whole_item<T: MiriadVectorType>(&self, name: &str) -> Result<T> {
    //}
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_header_item_size() {
        assert_eq!(::std::mem::size_of::<HeaderItem>(), HEADER_RECORD_SIZE);
    }
}
