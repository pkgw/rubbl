// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD-format data sets.

 */

extern crate byteorder;
extern crate openat;
#[macro_use] extern crate rubbl_core;

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use rubbl_core::io::EofReadExactExt;
use rubbl_core::Complex;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;


pub mod mask;
pub mod visdata;

use rubbl_core::errors::{Error, ErrorKind, Result};

/// The maximum length of the name of a dataset "item", in bytes.
pub const MAX_ITEM_NAME_LENGTH: usize = 8;



#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
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
    pub fn try_from_i32(type_code: i32) -> Result<Self> {
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

    /// This function takes a &str as an argument since working with
    /// individual characters is usually a hassle.
    pub fn try_from_abbrev(abbrev: &str) -> Result<Self> {
        // Ditto.
        match abbrev {
            "b" => Ok(Type::Int8),
            "j" => Ok(Type::Int16),
            "i" => Ok(Type::Int32),
            "l" => Ok(Type::Int64),
            "r" => Ok(Type::Float32),
            "d" => Ok(Type::Float64),
            "c" => Ok(Type::Complex64),
            "a" => Ok(Type::Text),
            _ => err_msg!("illegal MIRIAD type abbreviation {}", abbrev),
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

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>);
}

impl MiriadMappedType for u8 {
    const TYPE: Type = Type::Binary;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(buf.len(), 0);
        vec.copy_from_slice(buf);
    }
}

impl MiriadMappedType for i8 {
    const TYPE: Type = Type::Int8;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(unsafe { std::mem::transmute::<Vec<u8>, Vec<i8>>(val) }) // yeehaw!
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(buf.len(), 0);
        vec.copy_from_slice(unsafe { std::mem::transmute::<&[u8], &[i8]>(buf) });
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

        while let Some(n) = stream.eof_read_be_i64()? {
            val.push(n);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(8) {
            vec.push(BigEndian::read_i64(chunk));
        }
    }
}


impl MiriadMappedType for Complex<f32> {
    const TYPE: Type = Type::Complex64;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>> {
        let mut val = Vec::new();

        while let Some(x) = stream.eof_read_be_c64()? {
            val.push(x);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(8) {
            let real = BigEndian::read_f32(&chunk[..4]);
            let imag = BigEndian::read_f32(&chunk[4..]);
            vec.push(Complex::new(real, imag));
        }
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

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(1, String::new());
        vec[0] = String::from_utf8_lossy(buf).into_owned();
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


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ItemStorage {
    Small(Vec<u8>),
    Large(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct InternalItemInfo {
    pub ty: Type,
    pub storage: ItemStorage,
}

impl InternalItemInfo {
    pub fn new_small(ty: Type, data: Vec<u8>) -> Self {
        InternalItemInfo {
            ty: ty,
            storage: ItemStorage::Small(data),
        }
    }

    pub fn new_large(dir: &mut openat::Dir, name: &str) -> Result<Self> {
        let mut f = dir.open_file(name)?;
        let mut size_offset = 4;
        let mut type_buf = [0u8; 4];

        if let Err(e) = f.read_exact(&mut type_buf) {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                // We will assume that an (e.g.) 3-byte file is text or binary
                size_offset = 0;
                for b in type_buf.iter_mut() {
                    *b = 0;
                }
            } else {
                return Err(e.into());
            }
        }

        let type_code = BigEndian::read_i32(&type_buf);

        let ty = match Type::try_from_i32(type_code) {
            Ok(t) => t,
            Err(_) => {
                // This is probably a text file, but might be something we
                // don't understand. We test for ASCII printability which
                // might not be quite right.

                if type_buf.iter().all(|c| *c >= 0x20 && *c <= 0x7e) {
                    size_offset = 0;
                    Type::Text
                } else {
                    Type::Binary
                }
            }
        };

        let data_size = f.metadata()?.len() - size_offset;

        if data_size % ty.size() as u64 != 0 {
            return err_msg!("non-integral number of elements in {}", name);
        }

        Ok(InternalItemInfo {
            ty: ty,
            storage: ItemStorage::Large((data_size / ty.size() as u64) as usize),
        })
    }

    pub fn n_vals(&self) -> usize {
        if self.ty == Type::Text {
            1
        } else {
            match self.storage {
                ItemStorage::Small(ref data) => data.len() / self.ty.size(),
                ItemStorage::Large(n) => n,
            }
        }
    }
}


#[derive(Debug)]
pub struct Item<'a> {
    dset: &'a DataSet,
    name: &'a str,
    info: &'a InternalItemInfo,
}

impl<'a> Item<'a> {
    pub fn name(&self) -> &str {
        self.name
    }


    pub fn is_large(&self) -> bool {
        match self.info.storage {
            ItemStorage::Small(_) => false,
            ItemStorage::Large(_) => true,
        }
    }


    pub fn type_(&self) -> Type {
        self.info.ty
    }


    pub fn n_vals(&self) -> usize {
        self.info.n_vals()
    }


    pub fn read_vector<T: MiriadMappedType>(&self) -> Result<Vec<T>> {
        // TODO: upcasting
        if T::TYPE != self.info.ty {
            return err_msg!("expected variable of type {}, but found {}", T::TYPE, self.info.ty);
        }

        match self.info.storage {
            ItemStorage::Small(ref data) => T::vec_from_miriad_bytes(data),
            ItemStorage::Large(_) => {
                let mut f = self.dset.dir.open_file(self.name)?;

                if self.info.ty != Type::Text {
                    let align = std::cmp::max(4, self.info.ty.alignment()) as usize;
                    let mut align_buf = [0u8; 8];
                    f.read_exact(&mut align_buf[..align])?;
                }

                T::vec_from_miriad_reader(f)
            },
        }
    }


    pub fn read_scalar<T: MiriadMappedType>(&self) -> Result<T> {
        let vec = self.read_vector()?;

        if vec.len() != 1 {
            return err_msg!("expected scalar value for {} but got {}-element vector",
                            self.name, vec.len());
        }

        Ok(vec.into_iter().next().unwrap())
    }


    pub fn into_lines(self) -> Result<io::Lines<io::BufReader<fs::File>>> {
        if self.info.ty != Type::Text {
            return err_msg!("cannot read lines of non-text item {}", self.name);
        }

        if let ItemStorage::Small(_) = self.info.storage {
            return err_msg!("cannot read lines of small text item {}", self.name);
        }

        // Text items don't need any alignment futzing so we don't have to
        // skip initial bytes.
        Ok(io::BufReader::new(self.dset.dir.open_file(self.name)?).lines())
    }


    pub fn into_byte_stream(self) -> Result<io::BufReader<fs::File>> {
        if let ItemStorage::Small(_) = self.info.storage {
            // We *could* do this, but for coding simplicity we only allow it
            // for large items.
            return err_msg!("cannot turn small item {} into byte stream", self.name);
        }

        let f = self.dset.dir.open_file(self.name)?;
        let mut br = io::BufReader::new(f);

        if self.info.ty != Type::Text {
            let align = std::cmp::max(4, self.info.ty.alignment()) as usize;
            let mut align_buf = [0u8; 8];
            br.read_exact(&mut align_buf[..align])?;
        }

        Ok(br)
    }
}


#[derive(Debug)]
pub struct DataSet {
    dir: openat::Dir,
    items: HashMap<String, InternalItemInfo>,
    large_items_scanned: bool,
}


#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
struct HeaderItem {
    /// The name of this item, encoded in UTF-8 with no trailing NUL. Classic
    /// MIRIAD is, of course, completely unaware of UTF-8, but it seems like a
    /// sensible extension.
    pub name: [u8; 15],

    /// The length of the item including the necessary padding for alignment.
    pub aligned_len: u8,
}


impl DataSet {
    pub fn open<P: openat::AsPath>(path: P) -> Result<Self> {
        let mut ds = DataSet {
            dir: openat::Dir::open(path)?,
            items: HashMap::new(),
            large_items_scanned: false,
        };

        // Parse the header

        let mut header = io::BufReader::new(ds.dir.open_file("header")?);
        let mut rec = HeaderItem { name: [0; 15], aligned_len: 0 };
        let mut offset = 0;

        loop {
            // Read data directly into `rec`. As far as I can tell this is the
            // least-bad way to do this:
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

            offset += std::mem::size_of::<HeaderItem>();

            // If we pass the trailing NULs to from_utf8, they will silently
            // be included at the end of the `name` String.
            let mut name_len = 0;

            while rec.name[name_len] != 0 {
                name_len += 1;
            }

            let name = std::str::from_utf8(&rec.name[..name_len])?;
            // TODO: validate "len": must be between 5 and 64

            let (ty, data) = if rec.aligned_len == 0 {
                (Type::Binary, Vec::new())
            } else {
                let type_code = header.read_i32::<BigEndian>()?;
                // TODO: warn and press on if conversion fails
                let mut ty = Type::try_from_i32(type_code)?;

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

            offset += rec.aligned_len as usize;

            // TODO: could/should warn if a redundant item is encountered
            ds.items.insert(name.to_owned(), InternalItemInfo::new_small(ty, data));

            // Maintain alignment.
            let misalignment = offset % std::mem::size_of::<HeaderItem>();

            if misalignment != 0 {
                let mut align_buf = [0u8; 16];
                let n_to_read = 16 - misalignment;

                if !header.eof_read_exact(&mut align_buf[..n_to_read])? {
                    break;
                }

                offset += n_to_read;
            }
        }

        // All done

        Ok(ds)
    }


    fn scan_large_items(&mut self) -> Result<()> {
        for maybe_item in self.dir.list_dir(".")? {
            let item = maybe_item?;

            if let Some(s) = item.file_name().to_str() {
                if s == "header" {
                    continue;
                }

                if s.starts_with(".") {
                    continue;
                }

                // TODO: could/should warn if a large item shadowing a small
                // item is encountered.
                let iii = InternalItemInfo::new_large(&mut self.dir, s)?;
                self.items.insert(s.to_owned(), iii);
            }
        }

        self.large_items_scanned = true;
        Ok(())
    }


    pub fn item_names<'a>(&'a mut self) -> Result<DataSetItemNamesIterator<'a>> {
        if !self.large_items_scanned {
            self.scan_large_items()?;
            self.large_items_scanned = true;
        }

        Ok(DataSetItemNamesIterator::new(self))
    }


    pub fn items<'a>(&'a mut self) -> Result<DataSetItemsIterator<'a>> {
        if !self.large_items_scanned {
            self.scan_large_items()?;
            self.large_items_scanned = true;
        }

        Ok(DataSetItemsIterator::new(self))
    }


    /// Get a handle to an item in this data set.
    ///
    /// I feel like there should be a better way to do this, but right now the
    /// reference to *item_name* needs to have a lifetime compatible with the
    /// reference to the dataset itself.
    pub fn get<'a>(&'a mut self, item_name: &'a str) -> Result<Option<Item<'a>>> {
        // The HashMap access approach I use here feels awkward to me but it's
        // the only way I can get the lifetimes to work out.

        if !self.items.contains_key(item_name) {
            // Assume it's an as-yet-unprobed large item on the filesystem.
            let iii = match InternalItemInfo::new_large(&mut self.dir, item_name) {
                Ok(iii) => iii,
                Err(Error(ErrorKind::Io(ioe), _)) => {
                    if ioe.kind() == io::ErrorKind::NotFound {
                        // No such item. Don't bother to cache negative results.
                        return Ok(None);
                    }
                    return Err(ioe.into());
                },
                Err(e) => {
                    return Err(e);
                },
            };
            self.items.insert(item_name.to_owned(), iii);
        }

        Ok(Some(Item {
            dset: self,
            name: item_name,
            info: self.items.get(item_name).unwrap(),
        }))
    }


    pub fn open_uv(&mut self) -> Result<visdata::Reader> {
        visdata::Reader::create(self)
    }
}


/// This helper struct stores state when iterating over the item names
/// provided by a MIRIAD data set.
#[derive(Debug)]
pub struct DataSetItemNamesIterator<'a> {
    inner: std::collections::hash_map::Keys<'a, String, InternalItemInfo>,
}

impl<'a> DataSetItemNamesIterator<'a> {
    pub fn new(dset: &'a DataSet) -> Self {
        DataSetItemNamesIterator {
            inner: dset.items.keys(),
        }
    }
}

impl<'a> Iterator for DataSetItemNamesIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|s| s.as_ref())
    }
}


/// This helper struct stores state when iterating over the items inside a
/// MIRIAD data set.
#[derive(Debug)]
pub struct DataSetItemsIterator<'a> {
    dset: &'a DataSet,
    inner: std::collections::hash_map::Iter<'a, String, InternalItemInfo>,
}

impl<'a> DataSetItemsIterator<'a> {
    pub fn new(dset: &'a DataSet) -> Self {
        DataSetItemsIterator {
            dset: dset,
            inner: dset.items.iter(),
        }
    }
}

impl<'a> Iterator for DataSetItemsIterator<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|kv| Item {
            dset: self.dset,
            name: kv.0,
            info: kv.1,
        })
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
