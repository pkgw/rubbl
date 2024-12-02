// Copyright 2017-2020 Peter Williams
// Licensed under the MIT License.

//! Access to MIRIAD-format data sets.

use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use rubbl_core::io::{AligningReader, AligningWriter, EofReadExactExt};
use rubbl_core::Complex;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::prelude::*;
use thiserror::Error;

pub mod mask;
pub mod visdata;

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

/// An error type for when a MIRIAD file is malformed.
#[derive(Error, Debug)]
pub enum MiriadFormatError {
    #[error("{0}")]
    Generic(String),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

impl Type {
    pub fn try_from_i32(type_code: i32) -> Result<Self, MiriadFormatError> {
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
            _ => Err(MiriadFormatError::Generic(format!(
                "illegal MIRIAD type code {type_code}"
            ))),
        }
    }

    /// This function takes a &str as an argument since working with
    /// individual characters is usually a hassle.
    pub fn try_from_abbrev(abbrev: &str) -> Result<Self, MiriadFormatError> {
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
            _ => Err(MiriadFormatError::Generic(format!(
                "illegal MIRIAD type abbreviation {abbrev}"
            ))),
        }
    }

    pub fn abbrev_char(&self) -> char {
        match *self {
            Type::Binary => '?',
            Type::Int8 => 'b',
            Type::Int16 => 'j',
            Type::Int32 => 'i',
            Type::Int64 => 'l',
            Type::Float32 => 'r',
            Type::Float64 => 'd',
            Type::Complex64 => 'c',
            Type::Text => 'a',
        }
    }

    pub fn size(&self) -> usize {
        match *self {
            Type::Binary => 1,
            Type::Int8 => 1,
            Type::Int16 => 2,
            Type::Int32 => 4,
            Type::Int64 => 8,
            Type::Float32 => 4,
            Type::Float64 => 8,
            Type::Complex64 => 8,
            Type::Text => 1,
        }
    }

    pub fn alignment(&self) -> u8 {
        match *self {
            Type::Binary => 1,
            Type::Int8 => 1,
            Type::Int16 => 2,
            Type::Int32 => 4,
            Type::Int64 => 8,
            Type::Float32 => 4,
            Type::Float64 => 8,
            Type::Complex64 => 4, // this is the only surprising one
            Type::Text => 1,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.pad(match *self {
            Type::Binary => "binary",
            Type::Int8 => "int8",
            Type::Int16 => "int16",
            Type::Int32 => "int32",
            Type::Int64 => "int64",
            Type::Float32 => "float32",
            Type::Float64 => "float64",
            Type::Complex64 => "complex64",
            Type::Text => "text",
        })
    }
}

/// This trait marks that the given type maps onto a type defined in the
/// MIRIAD data format.
pub trait MiriadMappedType: Sized {
    /// The particular MIRIAD `Type` to which this Rust type maps.
    const TYPE: Type;

    fn vec_from_miriad_reader<R: Read>(stream: R) -> Result<Vec<Self>, std::io::Error>;

    fn vec_from_miriad_bytes(buf: &[u8]) -> Result<Vec<Self>, std::io::Error> {
        Self::vec_from_miriad_reader(std::io::Cursor::new(buf))
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>);

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>);

    /// This is a hack so we can write type-generic functions that can figure
    /// out how many bytes are in a string.
    fn get_miriad_count(values: &[Self]) -> usize {
        values.len()
    }
}

impl MiriadMappedType for u8 {
    const TYPE: Type = Type::Binary;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(buf.len(), 0);
        vec.copy_from_slice(buf);
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < values.len() {
            let extra = values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(values.len());
        }

        vec.copy_from_slice(values);
    }
}

impl MiriadMappedType for i8 {
    const TYPE: Type = Type::Int8;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();
        stream.read_to_end(&mut val)?;
        Ok(unsafe { std::mem::transmute::<Vec<u8>, Vec<i8>>(val) }) // yeehaw!
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(buf.len(), 0);
        vec.copy_from_slice(unsafe { std::mem::transmute::<&[u8], &[i8]>(buf) });
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < values.len() {
            let extra = values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(values.len());
        }

        vec.copy_from_slice(unsafe { std::mem::transmute::<&[i8], &[u8]>(values) });
    }
}

impl MiriadMappedType for i16 {
    const TYPE: Type = Type::Int16;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(n) = stream.eof_read_be_i16::<std::io::Error>()? {
            val.push(n);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(2) {
            vec.push(BigEndian::read_i16(chunk));
        }
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 2 * values.len() {
            let extra = 2 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(2 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_i16(&mut vec[ofs..ofs + 2], *v);
            ofs += 2;
        }
    }
}

impl MiriadMappedType for i32 {
    const TYPE: Type = Type::Int32;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(n) = stream.eof_read_be_i32::<std::io::Error>()? {
            val.push(n);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(4) {
            vec.push(BigEndian::read_i32(chunk));
        }
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 4 * values.len() {
            let extra = 4 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(4 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_i32(&mut vec[ofs..ofs + 4], *v);
            ofs += 4;
        }
    }
}

impl MiriadMappedType for i64 {
    const TYPE: Type = Type::Int64;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(n) = stream.eof_read_be_i64::<std::io::Error>()? {
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

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 8 * values.len() {
            let extra = 8 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(8 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_i64(&mut vec[ofs..ofs + 8], *v);
            ofs += 8;
        }
    }
}

impl MiriadMappedType for Complex<f32> {
    const TYPE: Type = Type::Complex64;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(x) = stream.eof_read_be_c64::<std::io::Error>()? {
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

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 8 * values.len() {
            let extra = 8 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(8 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_f32(&mut vec[ofs..ofs + 4], v.re);
            ofs += 4;
            BigEndian::write_f32(&mut vec[ofs..ofs + 4], v.im);
            ofs += 4;
        }
    }
}

impl MiriadMappedType for String {
    const TYPE: Type = Type::Text;

    /// As a special hack, this only ever returns a 1-element vector.
    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = String::new();
        stream.read_to_string(&mut val)?;
        Ok(vec![val])
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.resize(1, String::new());
        vec[0] = String::from_utf8_lossy(buf).into_owned();
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        assert_eq!(values.len(), 1);
        let bytes = values[0].as_bytes();

        if vec.capacity() < bytes.len() {
            let extra = bytes.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(bytes.len());
        }

        vec.copy_from_slice(bytes);
    }

    fn get_miriad_count(values: &[Self]) -> usize {
        assert_eq!(values.len(), 1);
        values[0].as_bytes().len()
    }
}

impl MiriadMappedType for f32 {
    const TYPE: Type = Type::Float32;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(x) = stream.eof_read_be_f32::<std::io::Error>()? {
            val.push(x);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(4) {
            vec.push(BigEndian::read_f32(chunk));
        }
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 4 * values.len() {
            let extra = 4 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(4 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_f32(&mut vec[ofs..ofs + 4], *v);
            ofs += 4;
        }
    }
}

impl MiriadMappedType for f64 {
    const TYPE: Type = Type::Float64;

    fn vec_from_miriad_reader<R: Read>(mut stream: R) -> Result<Vec<Self>, std::io::Error> {
        let mut val = Vec::new();

        while let Some(x) = stream.eof_read_be_f64::<std::io::Error>()? {
            val.push(x);
        }

        Ok(val)
    }

    fn decode_buf_into_vec(buf: &[u8], vec: &mut Vec<Self>) {
        vec.clear();

        for chunk in buf.chunks(8) {
            vec.push(BigEndian::read_f64(chunk));
        }
    }

    fn encode_values_into_vec(values: &[Self], vec: &mut Vec<u8>) {
        if vec.capacity() < 8 * values.len() {
            let extra = 8 * values.len() - vec.capacity();
            vec.reserve(extra);
        }

        unsafe {
            vec.set_len(8 * values.len());
        }

        let mut ofs = 0;

        for v in values {
            BigEndian::write_f64(&mut vec[ofs..ofs + 8], *v);
            ofs += 8;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnyMiriadValue {
    Binary(Vec<u8>),
    Int8(Vec<i8>),
    Int16(Vec<i16>),
    Int32(Vec<i32>),
    Int64(Vec<i64>),
    Float32(Vec<f32>),
    Float64(Vec<f64>),
    Complex64(Vec<Complex<f32>>),
    Text(String),
}

impl AnyMiriadValue {
    fn from_type_and_buf(ty: Type, buf: &[u8]) -> Self {
        match ty {
            Type::Binary => {
                let mut vec = Vec::new();
                u8::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Binary(vec)
            }

            Type::Int8 => {
                let mut vec = Vec::new();
                i8::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Int8(vec)
            }

            Type::Int16 => {
                let mut vec = Vec::new();
                i16::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Int16(vec)
            }

            Type::Int32 => {
                let mut vec = Vec::new();
                i32::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Int32(vec)
            }

            Type::Int64 => {
                let mut vec = Vec::new();
                i64::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Int64(vec)
            }

            Type::Float32 => {
                let mut vec = Vec::new();
                f32::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Float32(vec)
            }

            Type::Float64 => {
                let mut vec = Vec::new();
                f64::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Float64(vec)
            }

            Type::Complex64 => {
                let mut vec = Vec::new();
                Complex::<f32>::decode_buf_into_vec(buf, &mut vec);
                AnyMiriadValue::Complex64(vec)
            }

            Type::Text => AnyMiriadValue::Text(String::from_utf8_lossy(buf).into_owned()),
        }
    }
}

impl std::fmt::Display for AnyMiriadValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        fn do_vec<T: std::fmt::Display>(
            f: &mut std::fmt::Formatter,
            vec: &[T],
        ) -> std::fmt::Result {
            if vec.len() == 1 {
                return f.write_fmt(format_args!("{}", vec[0]));
            }

            let mut first = true;

            f.write_str("[")?;

            for item in vec {
                if first {
                    first = false;
                } else {
                    f.write_str(", ")?;
                }

                f.write_fmt(format_args!("{}", item))?;
            }

            f.write_str("]")
        }

        match self {
            AnyMiriadValue::Binary(vec) => do_vec(f, vec),
            AnyMiriadValue::Int8(vec) => do_vec(f, vec),
            AnyMiriadValue::Int16(vec) => do_vec(f, vec),
            AnyMiriadValue::Int32(vec) => do_vec(f, vec),
            AnyMiriadValue::Int64(vec) => do_vec(f, vec),
            AnyMiriadValue::Float32(vec) => do_vec(f, vec),
            AnyMiriadValue::Float64(vec) => do_vec(f, vec),
            AnyMiriadValue::Complex64(vec) => do_vec(f, vec),
            AnyMiriadValue::Text(s) => {
                f.write_str("\"")?;
                f.write_str(s)?;
                f.write_str("\"")
            }
        }
    }
}

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
            ty,
            storage: ItemStorage::Small(data),
        }
    }

    pub fn new_large(dir: &mut openat::Dir, name: &str) -> Result<Self, MiriadFormatError> {
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
            return Err(MiriadFormatError::Generic(format!(
                "non-integral number of elements in {name}"
            )));
        }

        Ok(InternalItemInfo {
            ty,
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

pub type ReadStream = AligningReader<io::BufReader<fs::File>>;
pub type WriteStream = AligningWriter<io::BufWriter<fs::File>>;

#[derive(Debug)]
pub struct Item<'a> {
    dset: &'a DataSet,
    name: &'a str,
    info: &'a InternalItemInfo,
}

impl Item<'_> {
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

    pub fn read_vector<T: MiriadMappedType>(&self) -> Result<Vec<T>, MiriadFormatError> {
        // TODO: upcasting
        if T::TYPE != self.info.ty {
            return Err(MiriadFormatError::Generic(format!(
                "expected variable of type {}, but found {}",
                T::TYPE,
                self.info.ty
            )));
        }

        let v = match self.info.storage {
            ItemStorage::Small(ref data) => T::vec_from_miriad_bytes(data)?,
            ItemStorage::Large(_) => {
                let mut f = self.dset.dir.open_file(self.name)?;

                if self.info.ty != Type::Text {
                    let align = std::cmp::max(4, self.info.ty.alignment()) as usize;
                    let mut align_buf = [0u8; 8];
                    f.read_exact(&mut align_buf[..align])?;
                }

                T::vec_from_miriad_reader(f)?
            }
        };

        Ok(v)
    }

    pub fn read_scalar<T: MiriadMappedType>(&self) -> Result<T, MiriadFormatError> {
        let vec = self.read_vector()?;

        if vec.len() != 1 {
            return Err(MiriadFormatError::Generic(format!(
                "expected scalar value for {} but got {}-element vector",
                self.name,
                vec.len()
            )));
        }

        Ok(vec.into_iter().next().unwrap())
    }

    pub fn into_lines(self) -> Result<io::Lines<io::BufReader<fs::File>>, MiriadFormatError> {
        if self.info.ty != Type::Text {
            return Err(MiriadFormatError::Generic(format!(
                "cannot read lines of non-text item {}",
                self.name
            )));
        }

        if let ItemStorage::Small(_) = self.info.storage {
            return Err(MiriadFormatError::Generic(format!(
                "cannot read lines of small text item {}",
                self.name
            )));
        }

        // Text items don't need any alignment futzing so we don't have to
        // skip initial bytes.
        Ok(io::BufReader::new(self.dset.dir.open_file(self.name)?).lines())
    }

    pub fn into_byte_stream(self) -> Result<ReadStream, MiriadFormatError> {
        if let ItemStorage::Small(_) = self.info.storage {
            // We *could* do this, but for coding simplicity we only allow it
            // for large items.
            return Err(MiriadFormatError::Generic(format!(
                "cannot turn small item {} into byte stream",
                self.name
            )));
        }

        let f = self.dset.dir.open_file(self.name)?;
        let mut ar = AligningReader::new(io::BufReader::new(f));

        if self.info.ty != Type::Text {
            ar.align_to(std::cmp::max(4, self.info.ty.alignment()) as usize)?;
        }

        Ok(ar)
    }
}

#[derive(Debug)]
pub struct DataSet {
    dir: openat::Dir,
    items: HashMap<String, InternalItemInfo>,
    large_items_scanned: bool,
    needs_flush: bool,
}

impl DataSet {
    pub fn open<P: openat::AsPath>(path: P) -> Result<Self, MiriadFormatError> {
        let mut ds = DataSet {
            dir: openat::Dir::open(path)?,
            items: HashMap::new(),
            large_items_scanned: false,
            needs_flush: false,
        };

        // Parse the header

        let mut header = AligningReader::new(io::BufReader::new(ds.dir.open_file("header")?));
        let mut buf = [0u8; 16];

        loop {
            if !header.eof_read_exact::<std::io::Error>(&mut buf)? {
                break; // no more data
            }

            // First 15 bytes are variable name, last byte is "aligned
            // length". For the name, if we pass the trailing NULs to
            // from_utf8, they will silently be included at the end of the
            // `name` String.

            let mut name_len = 0;

            while name_len < 15 && buf[name_len] != 0 {
                name_len += 1;
            }

            let aligned_len = buf[15];

            let name = std::str::from_utf8(&buf[..name_len])?;
            // TODO: validate "len": must be between 5 and 64

            let (ty, data) = if aligned_len == 0 {
                (Type::Binary, Vec::new())
            } else {
                let type_code = header.read_i32::<BigEndian>()?;
                // TODO: warn and press on if conversion fails
                let mut ty = Type::try_from_i32(type_code)?;

                // The "Text" type is internal-only; textual header items are
                // expressed as arrays of int8's.
                if ty == Type::Int8 && aligned_len > 5 {
                    ty = Type::Text;
                }

                // The header-writing code aligns based on the type sizes, not
                // the type alignment values.

                let align = std::cmp::max(4, ty.size());
                header.align_to(align)?;
                let n_bytes = aligned_len as usize - align;

                if n_bytes % ty.size() != 0 {
                    // TODO: warn and press on
                    return Err(MiriadFormatError::Generic(format!(
                        "illegal array size {} for type {:?}",
                        n_bytes, ty
                    )));
                }

                // This initializes the buffer to zeros just to overwrite them
                // with header data, but this is semi-famously necessary; see e.g.
                // https://github.com/rust-lang/rfcs/blob/master/text/2930-read-buf.md#summary
                let mut data = vec![0; n_bytes];
                header.read_exact(&mut data[..])?;

                (ty, data)
            };

            // TODO: could/should warn if a redundant item is encountered
            ds.items
                .insert(name.to_owned(), InternalItemInfo::new_small(ty, data));
            header.align_to(16)?;
        }

        // All done

        Ok(ds)
    }

    fn scan_large_items(&mut self) -> Result<(), MiriadFormatError> {
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

    pub fn item_names(&mut self) -> Result<DataSetItemNamesIterator<'_>, MiriadFormatError> {
        if !self.large_items_scanned {
            self.scan_large_items()?;
            self.large_items_scanned = true;
        }

        Ok(DataSetItemNamesIterator::new(self))
    }

    pub fn items(&mut self) -> Result<DataSetItemsIterator<'_>, MiriadFormatError> {
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
    pub fn get<'a>(
        &'a mut self,
        item_name: &'a str,
    ) -> Result<Option<Item<'a>>, MiriadFormatError> {
        // The HashMap access approach I use here feels awkward to me but it's
        // the only way I can get the lifetimes to work out.

        if !self.items.contains_key(item_name) {
            // Assume it's an as-yet-unprobed large item on the filesystem.
            let iii = match InternalItemInfo::new_large(&mut self.dir, item_name) {
                Ok(iii) => iii,
                Err(e) => match e {
                    MiriadFormatError::IO(ioe) => {
                        if ioe.kind() == io::ErrorKind::NotFound {
                            // No such item. Don't bother to cache negative results.
                            return Ok(None);
                        }
                        return Err(ioe.into());
                    }
                    _ => return Err(e),
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

    pub fn open_uv(&mut self) -> Result<visdata::Decoder, MiriadFormatError> {
        visdata::Decoder::create(self)
    }

    pub fn new_uv_like(
        &mut self,
        template: &visdata::Decoder,
    ) -> Result<visdata::Encoder, MiriadFormatError> {
        visdata::Encoder::new_like(self, template)
    }

    pub fn create_large_item(
        &mut self,
        name: &str,
        ty: Type,
    ) -> Result<WriteStream, MiriadFormatError> {
        if name == "header" {
            return Err(MiriadFormatError::Generic(
                "cannot create an item named \"header\"".to_string(),
            ));
        }

        let name_bytes = name.as_bytes();

        if name_bytes.len() > 8 {
            return Err(MiriadFormatError::Generic(
                "cannot create an item with a name longer than 8 bytes".to_string(),
            ));
        }

        if !name_bytes.is_ascii() {
            return Err(MiriadFormatError::Generic(
                "cannot create an item with a non-ASCII name".to_string(),
            ));
        }

        let mut stream = AligningWriter::new(io::BufWriter::new(self.dir.write_file(name, 0o666)?));

        match ty {
            Type::Text | Type::Binary => {}
            _ => {
                stream.write_i32::<BigEndian>(ty as i32)?;
            }
        }

        self.items.insert(
            name.to_owned(),
            InternalItemInfo {
                ty,
                storage: ItemStorage::Large(0), // XXX size unknown
            },
        );

        Ok(stream)
    }

    pub fn set_small_item<T: MiriadMappedType>(
        &mut self,
        name: &str,
        values: &[T],
    ) -> Result<(), MiriadFormatError> {
        // Need to do this to ensure that we don't get a small item that masks
        // a large item.
        if !self.large_items_scanned {
            self.scan_large_items()?;
            self.large_items_scanned = true;
        }

        // TODO: validate name

        if !self.items.contains_key(name) {
            self.items.insert(
                name.to_owned(),
                InternalItemInfo::new_small(T::TYPE, Vec::new()),
            );
        }

        let iii = self.items.get_mut(name).unwrap();

        if let ItemStorage::Small(ref mut data) = iii.storage {
            T::encode_values_into_vec(values, data);

            if data.len() > 64 {
                return Err(MiriadFormatError::Generic(
                    "value too large to be stored as a small MIRIAD header item".to_string(),
                ));
            }
        } else {
            return Err(MiriadFormatError::Generic(format!(
                "cannot set \"{name}\" as a small item; would mask an existing large item"
            )));
        }

        iii.ty = T::TYPE;
        self.needs_flush = true;
        Ok(())
    }

    pub fn set_scalar_item<T: MiriadMappedType>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), MiriadFormatError> {
        self.set_small_item(name, &[value])
    }

    /// Flush any pending changes to the overall dataset. In particular, this
    /// means that the "header" file is rewritten.
    pub fn flush(&mut self) -> Result<(), MiriadFormatError> {
        if !self.needs_flush {
            return Ok(());
        }

        let mut stream =
            AligningWriter::new(io::BufWriter::new(self.dir.write_file("header", 0o666)?));

        for (name, item) in &self.items {
            if let ItemStorage::Small(ref data) = item.storage {
                let mut buf = [0u8; 16];
                stream.align_to(16)?;

                let name_bytes = name.as_bytes();
                buf[..name_bytes.len()].copy_from_slice(name_bytes);

                let n_bytes = data.len();

                if n_bytes == 0 {
                    // Data-free items are allowed and have no type indicator.
                    buf[15] = 0;
                    stream.write_all(&buf)?;
                    continue;
                }

                let alignment = std::cmp::max(4, item.ty.size());
                let excess = (stream.offset() as usize + 4) % alignment;
                let n_alignment_bytes = if excess == 0 { 0 } else { alignment - excess };

                buf[15] = (4 + n_alignment_bytes + n_bytes) as u8;
                stream.write_all(&buf)?;
                stream.write_i32::<BigEndian>(item.ty as u8 as i32)?;

                if n_alignment_bytes > 0 {
                    for b in &mut buf[..n_alignment_bytes] {
                        *b = 0;
                    }

                    stream.write_all(&buf[..n_alignment_bytes])?;
                }

                stream.write_all(data)?;
            }
        }

        self.needs_flush = false;
        Ok(())
    }
}

impl Drop for DataSet {
    fn drop(&mut self) {
        // cf: https://github.com/rust-lang/rust/issues/32677
        let _r = self.flush();
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
            dset,
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
