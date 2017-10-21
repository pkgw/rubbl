// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD-format data sets.

 */

extern crate byteorder;
#[macro_use] extern crate error_chain;
extern crate openat;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use std::collections::HashMap;
use std::io;
use std::io::prelude::*;


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
    pub fn try_from(type_code: i32) -> Result<Self> {
        // Kind of gross ...
        match type_code {
            0 => Ok(Type::Binary),
            1 => Ok(Type::Int8),
            3 => Ok(Type::Int16),
            2 => Ok(Type::Int32),
            8 => Ok(Type::Int64),
            4 => Ok(Type::Float32),
            5 => Ok(Type::Float64),
            7 => Ok(Type::Complex64),
            6 => Ok(Type::Text),
            _ => err_msg!("illegal MIRIAD type code {}", type_code),
        }
    }

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

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match self {
            &Type::Binary => "binary",
            &Type::Int8 => "int8",
            &Type::Int16 => "int16",
            &Type::Int32 => "int32",
            &Type::Int64 => "int64",
            &Type::Float32 => "float32",
            &Type::Float64 => "float64",
            &Type::Complex64 => "complex64",
            &Type::Text => "text",
        })
    }
}


/// This trait marks that the given type maps onto a type defined in the
/// MIRIAD data format.
pub trait MiriadMappedType: Sized {
    /// The particular MIRIAD `Type` to which this Rust type maps.
    const TYPE: Type;

    fn vec_from_miriad_reader<R: Read>(stream: R) -> Result<Vec<Self>>;

    fn vec_from_miriad_bytes(buf: &[u8]) -> Result<Vec<Self>> {
        Self::vec_from_miriad_reader(std::io::Cursor::new(buf))
    }

    fn scalar_from_miriad_reader<R: Read>(stream: R) -> Result<Self> {
        let vec = Self::vec_from_miriad_reader(stream)?;

        if vec.len() != 1 {
            return err_msg!("expected scalar value but got {}-element vector", vec.len());
        }

        Ok(vec.into_iter().next().unwrap())
    }

    fn scalar_from_miriad_bytes(buf: &[u8]) -> Result<Self> {
        Self::scalar_from_miriad_reader(std::io::Cursor::new(buf))
    }
}

impl MiriadMappedType for u8 {
    const TYPE: Type = Type::Binary;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(val)
    }
}

impl MiriadMappedType for i8 {
    const TYPE: Type = Type::Int8;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(unsafe { std::mem::transmute::<Vec<u8>, Vec<i8>>(val) }) // yeehaw!
    }
}

//impl MiriadMappedType for i16 {
//    const TYPE: Type = Type::Int16;
//}
//
//impl MiriadMappedType for i32 {
//    const TYPE: Type = Type::Int32;
//}

impl MiriadMappedType for i64 {
    const TYPE: Type = Type::Int64;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();

        loop {
            // XXX won't barf if the stream only has, e.g., 3 bytes
            match stream.read_i64::<BigEndian>() {
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }

                    return Err(e.into());
                },
                Ok(x) => { val.push(x); }
            }
        }

        Ok(val)
    }
}

impl MiriadMappedType for String {
    const TYPE: Type = Type::Text;

    /// As a special hack, this only ever returns a 1-element vector.
    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = String::new();
        stream.read_to_string(&mut val)?;
        Ok(vec!(val))
    }
}

//impl MiriadMappedType for f32 {
//    const TYPE: Type = Type::Float32;
//}
//
//impl MiriadMappedType for f64 {
//    const TYPE: Type = Type::Float64;
//}

// XXX complex64


pub struct ItemInfo<'a> {
    pub name: &'a str,
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


struct SmallItem {
    pub name: String,
    pub ty: Type,
    pub data: Vec<u8>,
}

impl SmallItem {
    pub fn new<S: ToString>(name: S, ty: Type, data: Vec<u8>) -> Self {
        SmallItem {
            name: name.to_string(),
            ty: ty,
            data: data,
        }
    }

    pub fn n_vals(&self) -> usize {
        if self.ty == Type::Text {
            1
        } else {
            self.data.len() / self.ty.size()
        }
    }

    pub fn is_scalar(&self) -> bool {
        self.n_vals() == 1
    }

    pub fn as_info<'a>(&'a self) -> ItemInfo<'a> {
        ItemInfo {
            name: &self.name,
            is_large: false,
            ty: self.ty,
            n_vals: self.n_vals(),
        }
    }
}


pub struct DataSet {
    dir: openat::Dir,
    small_items: HashMap<String, SmallItem>
}


impl DataSet {
    pub fn open<P: openat::AsPath>(path: P) -> Result<Self> {
        let mut ds = DataSet {
            dir: openat::Dir::open(path)?,
            small_items: HashMap::new(),
        };

        let mut header = ds.dir.open_file("header")?;
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

            // If we pass the trailing NULs to from_utf8, they will silently
            // be included at the end of the `name` String.
            let mut name_len = 0;

            while rec.name[name_len] != 0 {
                name_len += 1;
            }

            let name = std::str::from_utf8(&rec.name[..name_len])?;
            // TODO: validate "len": must be between 5 and 64

            let (ty, data) =  if rec.aligned_len == 0 {
                (Type::Binary, Vec::new())
            } else {
                let type_code = header.read_i32::<BigEndian>()?;
                // TODO: warn and press on if conversion fails
                let mut ty = Type::try_from(type_code)?;

                // The "Text" type is internal-only; textual header items are
                // expressed as arrays of int8's.
                if ty == Type::Int8 && rec.aligned_len > 5 {
                    ty = Type::Text;
                }

                // The header-writing code aligns based on the type sizes, not
                // the type alignment values.

                let align = std::cmp::max(4, ty.size());
                let mut align_buf = [0u8; 8];
                header.read_exact(&mut align_buf[..align - 4])?;

                let n_bytes = rec.aligned_len as usize - align;

                if n_bytes % ty.size() != 0 {
                    // TODO: warn and press on
                    return err_msg!("illegal array size {} for type {:?}", n_bytes, ty);
                }

                let mut data = Vec::with_capacity(n_bytes);
                unsafe { data.set_len(n_bytes); } // better way?
                header.read_exact(&mut data[..])?;

                (ty, data)
            };

            // TODO: could/should warn if a redundant item is encountered
            ds.small_items.insert(name.to_owned(), SmallItem::new(name, ty, data));
        }

        Ok(ds)
    }

    pub fn item_names<'a>(&'a self) -> DataSetItemNamesIterator<'a> {
        DataSetItemNamesIterator::new(self)
    }

    pub fn item_info<'a>(&'a self, item_name: &str) -> ItemInfo<'a> {
        if let Some(small_item) = self.small_items.get(item_name) {
            return small_item.as_info();
        }

        panic!("NYI");
    }

    pub fn read_scalar_item<T: MiriadMappedType>(&self, name: &str) -> Result<T> {
        // TODO: upcasting
        if let Some(small_item) = self.small_items.get(name) {
            if small_item.ty != T::TYPE {
                return err_msg!("expected type {} but got {}", T::TYPE, small_item.ty);
            }

            return T::scalar_from_miriad_bytes(&small_item.data[..]);
        }

        panic!("NYI");
    }

    pub fn read_vector_item<T: MiriadMappedType>(&self, name: &str) -> Result<Vec<T>> {
        // TODO: upcasting
        if let Some(small_item) = self.small_items.get(name) {
            if small_item.ty != T::TYPE {
                return err_msg!("expected type {} but got {}", T::TYPE, small_item.ty);
            }

            return T::vec_from_miriad_bytes(&small_item.data[..]);
        }

        panic!("NYI");
    }
}


/// This helper struct stores state when iterating over the item names
/// provided by a MIRIAD data set.
pub struct DataSetItemNamesIterator<'a> {
    dset: &'a DataSet,
    small_names_iter: Option<std::collections::hash_map::Keys<'a, String, SmallItem>>
}

impl<'a> DataSetItemNamesIterator<'a> {
    pub fn new(dset: &'a DataSet) -> Self {
        DataSetItemNamesIterator {
            dset: dset,
            small_names_iter: Some(dset.small_items.keys())
        }
    }
}

impl<'a> Iterator for DataSetItemNamesIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut sni) = self.small_names_iter {
            if let Some(k) = sni.next() {
                return Some(k);
            }
        }

        self.small_names_iter = None;

        None
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_header_item_size() {
        assert_eq!(::std::mem::size_of::<HeaderItem>(), HEADER_RECORD_SIZE);
    }
}
