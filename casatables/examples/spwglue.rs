// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

extern crate byteorder;
extern crate clap;
extern crate itertools;
#[macro_use] extern crate ndarray;
#[macro_use] extern crate nom;
extern crate num_traits;
extern crate pbr;
extern crate rubbl_casatables;
#[macro_use] extern crate rubbl_core;

use clap::{App, Arg};
use ndarray::{Ix1, Ix2};
use num_traits::{Float, One, Signed, Zero};
use rubbl_casatables::{CasaDataType, CasaScalarData, Table, TableOpenMode, TableRow};
use rubbl_casatables::errors::{Error, Result};
use rubbl_core::{Array, Complex};
use rubbl_core::notify::ClapNotificationArgsExt;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Display;
use std::fs::File;
use std::marker::PhantomData;
use std::ops::{AddAssign, BitOrAssign, Range, Sub};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;


// Quick .npy file parsing, stealing work from the `npy` crate version 0.3.2.

mod mini_npy_parser {
    use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
    use ndarray::{Array, Dimension};
    use nom::*;
    use rubbl_casatables::DimFromShapeSlice;
    use rubbl_casatables::errors::Result;
    use std::collections::HashMap;
    use std::io::Read;

    #[derive(PartialEq, Eq, Debug)]
    enum LimitedPyLiteral {
        String(String),
        Integer(i64),
        Bool(bool),
        List(Vec<LimitedPyLiteral>),
        Map(HashMap<String,LimitedPyLiteral>),
    }

    pub fn npy_stream_to_ndarray<R: Read, D: Dimension + DimFromShapeSlice>(stream: &mut R) -> Result<Array<f64, D>> {
        let mut preamble = [0u8; 10];

        stream.read_exact(&mut preamble)?;

        if &preamble[..8] != b"\x93NUMPY\x01\x00" {
            return err_msg!("stream does not appear to be NPY-format save data");
        }

        // Because header_len is only a u16, it can't be absurdly large even
        // in a maliciously-constructed file.
        //
        // The Python header is padded such that the data start at a multiple
        // of 16 bytes into the file. If the un-padded header length is a
        // total of X bytes, we can calculate the padded length as ((X + 15) /
        // 16) * 16 with standard truncating integer division. We've already
        // read 10 bytes though, so we need to account for those in the total
        // length as well as for the length that remains to be read.
        let header_len = LittleEndian::read_u16(&preamble[8..]);
        let aligned_len = ((header_len as usize + 10 + 15) / 16) * 16 - 10;

        let mut header = Vec::with_capacity(aligned_len);
        unsafe { header.set_len(aligned_len); }
        stream.read_exact(&mut header[..])?;

        let pyinfo = match map(&header) {
            IResult::Done(_, info) => info,
            IResult::Error(e) => {
                return err_msg!("failed to parse NPY Python header: {}", e);
            },
            IResult::Incomplete(_) => {
                return err_msg!("failed to parse NPY Python header: incomplete data");
            },
        };

        let pyinfo = match pyinfo {
            LimitedPyLiteral::Map(m) => m,
            other => {
                return err_msg!("bad NPY Python header: expected toplevel map but got {:?}", other);
            },
        };

        let descr = match pyinfo.get("descr") {
            Some(&LimitedPyLiteral::String(ref s)) => s,
            other => {
                return err_msg!("bad NPY Python header: expected string item \"descr\" but got {:?}", other);
            },
        };

        let fortran_order = match pyinfo.get("fortran_order") {
            Some(&LimitedPyLiteral::Bool(b)) => b,
            other => {
                return err_msg!("bad NPY Python header: expected bool item \"fortran_order\" but got {:?}", other);
            },
        };

        let py_shape = match pyinfo.get("shape") {
            Some(&LimitedPyLiteral::List(ref ell)) => ell,
            other => {
                return err_msg!("bad NPY Python header: expected list item \"shape\" but got {:?}", other);
            },
        };

        // We could support more choices here ...
        if descr != "<f8" {
            return err_msg!("unsupported NPY file: data type must be little-endian \
                             f64 (\"<f8\") but got \"{}\"", descr);
        }

        // ... and here.
        if fortran_order {
            return err_msg!("unsupported NPY file: data ordering must be C, but got Fortran");
        }

        let mut shape = Vec::new();

        for py_shape_item in py_shape {
            match py_shape_item {
                &LimitedPyLiteral::Integer(i) => {
                    shape.push(i as u64);
                },
                other => {
                    return err_msg!("bad NPY Python header: expected \"shape\" to be all integers but got {:?}", other);
                },
            }
        }

        let mut arr = unsafe { Array::uninitialized(D::from_shape_slice(&shape)) };

        // Note: we "should" probably use a BufReader here, but the
        // performance of this bit is totally insignificant in the grand
        // scheme of things.

        for item in arr.iter_mut() {
            *item = stream.read_f64::<LittleEndian>()?;
        }

        Ok(arr)
    }

    named!(item<LimitedPyLiteral>, alt!(integer | boolean | string | list | map));

    named!(integer<LimitedPyLiteral>,
        map!(
            map_res!(
                map_res!(
                    ws!(digit),
                    ::std::str::from_utf8
                ),
                ::std::str::FromStr::from_str
            ),
            LimitedPyLiteral::Integer
        )
    );

    named!(boolean<LimitedPyLiteral>,
        ws!(alt!(
            tag!("True") => { |_| LimitedPyLiteral::Bool(true) } |
            tag!("False") => { |_| LimitedPyLiteral::Bool(false) }
        ))
    );

    named!(string<LimitedPyLiteral>,
        map!(
            map!(
                map_res!(
                    ws!(alt!(
                        delimited!(tag!("\""),
                            is_not_s!("\""),
                            tag!("\"")) |
                        delimited!(tag!("\'"),
                            is_not_s!("\'"),
                            tag!("\'"))
                        )),
                    ::std::str::from_utf8
                ),
                |s: &str| s.to_string()
            ),
            LimitedPyLiteral::String
        )
    );

    // Note that we do not distriguish between tuples and lists.
    named!(list<LimitedPyLiteral>,
        map!(
            ws!(alt!(
                delimited!(tag!("["),
                    terminated!(separated_list!(tag!(","), item), alt!(tag!(",") | tag!(""))),
                    tag!("]")) |
                delimited!(tag!("("),
                    terminated!(separated_list!(tag!(","), item), alt!(tag!(",") | tag!(""))),
                    tag!(")"))
            )),
            LimitedPyLiteral::List
        )
    );

    // Note that we only allow string keys.
    named!(map<LimitedPyLiteral>,
        map!(
            ws!(
                delimited!(tag!("{"),
                    terminated!(separated_list!(tag!(","),
                        separated_pair!(map_opt!(string, |it| match it { LimitedPyLiteral::String(s) => Some(s), _ => None }), tag!(":"), item)
                    ), alt!(tag!(",") | tag!(""))),
                    tag!("}"))
            ),
            |v: Vec<_>| LimitedPyLiteral::Map(v.into_iter().collect())
        )
    );
}

use mini_npy_parser::npy_stream_to_ndarray;


/// Code for combining spw-associated quantities. We have to implement these
/// as discrete types so that we can leverage Rust's generics. It's a bit of a
/// shame that we can't handle the options purely with data tables, but
/// type-based column reading has a lot of advantages more generally.
mod spw_table {
    use super::*;

    /// Columns handled by this struct are simply ignored.
    struct IgnoreColumn<T> { _nope: PhantomData<T> }

    impl<T> IgnoreColumn<T> {
        pub fn new() -> Self { Self { _nope: PhantomData } }

        pub fn process(&self, _src_table: &mut Table, _col_name: &str,
                       _mappings: &[OutputSpwInfo], _dest_table: &mut Table) -> Result<()> {
            Ok(())
        }
    }


    /// Columns handled by this struct use the first value that appears.
    struct UseFirstColumn<T: CasaScalarData> { _nope: PhantomData<T> }

    impl<T: CasaScalarData> UseFirstColumn<T> {
        pub fn new() -> Self { Self { _nope: PhantomData } }

        pub fn process(&self, src_table: &mut Table, col_name: &str,
                       mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
            let data = src_table.get_col_as_vec::<T>(col_name)?;

            for (i, mapping) in mappings.iter().enumerate() {
                let mut idx_iter = mapping.spw_indices();
                let first_idx = idx_iter.next().unwrap(); // assume we have a nonzero number of spws
                dest_table.put_cell(col_name, i as u64, &data[first_idx])?;
            }

            Ok(())
        }
    }


    /// In columns handled by this struct, every value must be the same.
    struct MustMatchColumn<T: CasaScalarData> { _nope: PhantomData<T> }

    impl<T: CasaScalarData + Display> MustMatchColumn<T> {
        pub fn new() -> Self { Self { _nope: PhantomData } }

        pub fn process(&self, src_table: &mut Table, col_name: &str,
                       mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
            let data = src_table.get_col_as_vec::<T>(col_name)?;

            for (i, mapping) in mappings.iter().enumerate() {
                let mut idx_iter = mapping.spw_indices();
                let first_idx = idx_iter.next().unwrap(); // assume we have a nonzero number of spws
                let first_value = data[first_idx].clone();

                for idx in idx_iter {
                    if data[idx] != first_value {
                        return err_msg!("value changed from {} to {}", first_value, data[idx]);
                    }
                }

                dest_table.put_cell(col_name, i as u64, &first_value)?;
            }

            Ok(())
        }
    }


    /// In columns handled by this struct, the cell values are scalars and the
    /// output is the sum of the inputs.
    struct SumScalarColumn<T> { _nope: PhantomData<T> }

    impl<T: CasaScalarData + AddAssign + Copy + Default> SumScalarColumn<T> {
        pub fn new() -> Self { Self { _nope: PhantomData } }

        pub fn process(&self, src_table: &mut Table, col_name: &str,
                       mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
            let data = src_table.get_col_as_vec::<T>(col_name)?;

            for (i, mapping) in mappings.iter().enumerate() {
                let mut value = T::default();

                for idx in mapping.spw_indices() {
                    value += data[idx];
                }

                dest_table.put_cell(col_name, i as u64, &value)?;
            }

            Ok(())
        }
    }


    /// In columns handled by this struct, the cell values are 1D vectors, and the
    /// output is the concatenation of all of the inputs.
    struct ConcatVectorColumn<T: CasaScalarData> { _nope: PhantomData<T> }

    impl<T: CasaScalarData> ConcatVectorColumn<T> where Vec<T>: CasaDataType {
        pub fn new() -> Self { Self { _nope: PhantomData } }

        pub fn process(&self, src_table: &mut Table, col_name: &str,
                       mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
            for (i, mapping) in mappings.iter().enumerate() {
                let mut vec = Vec::<T>::new();

                for idx in mapping.spw_indices() {
                    let mut item = src_table.get_cell_as_vec(col_name, idx as u64)?;
                    vec.append(&mut item)
                }

                dest_table.put_cell(col_name, i as u64, &vec)?;
            }

            Ok(())
        }
    }


    /// This macro creates the enum type that handles all of the possible
    /// columns that might appear in the SPECTRAL_WINDOW table. The enum type
    /// is kind of awkward, but once you work out the macro magic it becomes
    /// fairly straightforward to use, and as mentioned before this approach
    /// allows us to leverage Rust's type system nicely.
    ///
    /// Because macros work on an AST level, we can't capture the "state type"
    /// of each column as a genuine type (e.g. UseFirstColumn<i32>) because we
    /// are then unable to refer to that type in expression contexts.
    /// Therefore we have to put that piece of info in terms of an "ident"
    /// typed capture.
    macro_rules! spectral_window_columns {
        {$($variant_name:ident($col_name:ident, $state_type:ident, $data_type:ty)),+} => {
            /// This enumeration type represents a column that may appear in the
            /// SPECTRAL_WINDOW table of a visibility data measurement set. We
            /// have to use an enumeration type so that we can leverage Rust's
            /// generics to read in the data with strong, correct typing.
            enum SpectralWindowColumn {
                $(
                    $variant_name($state_type<$data_type>),
                )+
            }

            impl SpectralWindowColumn {
                pub fn col_name(&self) -> &'static str {
                    match self {
                        $(
                            &SpectralWindowColumn::$variant_name(_) => stringify!($col_name),
                        )+
                    }
                }

                pub fn process(&self, src_table: &mut Table, mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
                    match self {
                        $(
                            &SpectralWindowColumn::$variant_name(ref h) =>
                                h.process(src_table, self.col_name(), mappings, dest_table),
                        )+
                    }
                }
            }

            impl FromStr for SpectralWindowColumn {
                type Err = Error;

                fn from_str(s: &str) -> Result<Self> {
                    match s {
                        $(
                            stringify!($col_name) => Ok(SpectralWindowColumn::$variant_name($state_type::new())),
                        )+
                            _ => err_msg!("unrecognized column in SPECTRAL_WINDOW table: \"{}\"", s)
                    }
                }
            }
        };
    }


    spectral_window_columns! {
        AssocNature(ASSOC_NATURE, IgnoreColumn, ()),
        AssocSpwId(ASSOC_SPW_ID, IgnoreColumn, ()),
        BbcNo(BBC_NO, MustMatchColumn, i32),
        ChanFreq(CHAN_FREQ, ConcatVectorColumn, f64),
        ChanWidth(CHAN_WIDTH, ConcatVectorColumn, f64),
        DopplerId(DOPPLER_ID, UseFirstColumn, i32),
        EffectiveBw(EFFECTIVE_BW, ConcatVectorColumn, f64),
        FlagRow(FLAG_ROW, MustMatchColumn, bool),
        FreqGroup(FREQ_GROUP, MustMatchColumn, i32),
        FreqGroupName(FREQ_GROUP_NAME, MustMatchColumn, String),
        IfConvChain(IF_CONV_CHAIN, MustMatchColumn, i32),
        MeasFreqRef(MEAS_FREQ_REF, MustMatchColumn, i32),
        Name(NAME, UseFirstColumn, String),
        NetSideband(NET_SIDEBAND, MustMatchColumn, i32),
        NumChan(NUM_CHAN, SumScalarColumn, i32),
        RefFrequency(REF_FREQUENCY, UseFirstColumn, f64),
        Resolution(RESOLUTION, ConcatVectorColumn, f64),
        TotalBandwidth(TOTAL_BANDWIDTH, SumScalarColumn, f64)
    }


    // Quick wrapper type to avoid type visibility complaints

    pub struct WrappedSpectralWindowColumn(SpectralWindowColumn);

    impl FromStr for WrappedSpectralWindowColumn {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self> {
            Ok(WrappedSpectralWindowColumn(s.parse()?))
        }
    }

    impl WrappedSpectralWindowColumn {
        pub fn process(&self, src_table: &mut Table, mappings: &[OutputSpwInfo], dest_table: &mut Table) -> Result<()> {
            self.0.process(src_table, mappings, dest_table)
        }
    }
}

use spw_table::WrappedSpectralWindowColumn as SpectralWindowColumn;


type MaybeVisFactor = Option<Array<Complex<f32>, Ix1>>;

/// This module goes through the same kind of rigamarole for the main
/// visbility data table.
mod main_table {
    use super::*;

    /// These columns are passed through unmodified.
    #[derive(Clone, Debug, PartialEq)]
    struct IdentityColumn<T: CasaScalarData> {
        value: Option<T>,
    }

    impl<T: CasaScalarData> IdentityColumn<T> {
        fn new() -> Self {
            Self { value: None }
        }

        fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
            // Since this column helps define the record's identity, subsequent rows match the
            // first row by definition.
            if self.value.is_none() {
                self.value = Some(row.get_cell(col_name)?);
            }

            Ok(())
        }

        fn emit(&self, col_name: &str, _vis_factor: &MaybeVisFactor,
                table: &mut Table, row: u64) -> Result<()>
        {
            if let Some(ref v) = self.value {
                table.put_cell(col_name, row, v)?;
            }

            Ok(())
        }

        fn reset(&mut self) {
            self.value = None;
        }
    }


    /// This is kind of ridiculous, but I can't figure out a way to get a
    /// literal constant that's agnostic as to floating types, which stymies a
    /// type-agnostic implementation of the ApproxMatchColumn.
    trait CheckApproximateMatch {
        type Element: Float + One + PartialOrd + Signed;

        fn approx_match_tol() -> Self::Element {
            Self::Element::one().exp2().powi(-20)
        }

        fn is_approximately_same(&self, other: &Self) -> bool;
    }

    /// Without the following, the typechecker considers our impls to not be
    /// mutually exclusive because num_traits could in principle impl Float, etc,
    /// for Vec<T>.
    trait NeverImpledForVec {}
    impl NeverImpledForVec for f32 {}
    impl NeverImpledForVec for f64 {}

    impl<T: Float + One + NeverImpledForVec + PartialOrd + Signed + Sub + Zero> CheckApproximateMatch for T {
        type Element = T;

        fn is_approximately_same(&self, other: &Self) -> bool {
            if *self == Zero::zero() {
                other.abs() < Self::approx_match_tol()
            } else if *other == Zero::zero() {
                self.abs() < Self::approx_match_tol()
            } else {
                let diff = *self - *other;
                diff.abs() / self.abs() < Self::approx_match_tol()
            }
        }
    }

    impl<T: Float + One + PartialOrd + Signed + Sub + Zero> CheckApproximateMatch for Vec<T> {
        type Element = T;

        fn is_approximately_same(&self, other: &Self) -> bool {
            assert_eq!(self.len(), other.len());

            let tol = Self::approx_match_tol();

            for (v1, v2) in self.iter().zip(other.iter()) {
                if *v1 == Zero::zero() {
                    if Signed::abs(v2) > tol {
                        return false;
                    }
                } else if *v2 == Zero::zero() {
                    if Signed::abs(v1) > tol {
                        return false;
                    }
                } else {
                    let diff = *v1 - *v2;
                    if Signed::abs(&diff) / Signed::abs(v1) > tol {
                        return false;
                    }
                }
            }

            true
        }
    }


    /// Data in columns handled here must be *about* the same.
    ///
    /// This feature implemented since EVLA dataset
    /// 11A-266.sb4865287.eb4875705.55772.08031621527.ms has 22 rows out of ~9
    /// million that have an EXPOSURE value that differs from the others by 1
    /// part in ~10^9.
    #[derive(Clone, Debug, PartialEq)]
    struct ApproxMatchColumn<T: CasaDataType> {
        value: Option<T>,
    }

    impl<T: CasaDataType + CheckApproximateMatch> ApproxMatchColumn<T> {
        fn new() -> Self {
            Self { value: None }
        }

        fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
            let cur = row.get_cell(col_name)?;

            if let Some(ref prev) = self.value {
                if !prev.is_approximately_same(&cur) {
                    return err_msg!("column {} should be approximately constant across spws, but values changed", col_name);
                }
            } else {
                self.value =  Some(cur);
            }

            Ok(())
        }

        // I tried to use a writeable output row for this, but I couldn't find a
        // way to leave the FLAG_CATEGORY column undefined.
        fn emit(&self, col_name: &str, _vis_factor: &MaybeVisFactor,
                table: &mut Table, row: u64) -> Result<()>
        {
            if let Some(ref v) = self.value {
                table.put_cell(col_name, row, v)?;
            }

            Ok(())
        }

        fn reset(&mut self) {
            self.value = None;
        }
    }


    /// The cells in this column are ignored and left empty in the output.
    #[derive(Clone, Debug, PartialEq)]
    struct EmptyColumn<T: CasaDataType> {
        _nope: PhantomData<T>
    }

    impl<T> EmptyColumn<Vec<T>> where Vec<T>: CasaDataType {
        fn new() -> Self {
            Self { _nope: PhantomData }
        }

        fn process(&mut self, _col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, _row: &mut TableRow) -> Result<()> {
            Ok(())
        }

        fn emit(&self, _col_name: &str, _vis_factor: &MaybeVisFactor,
                _table: &mut Table, _row: u64) -> Result<()>
        {
            Ok(())
        }

        fn reset(&mut self) {
        }
    }


    /// The cells in this column are logically OR-ed together.
    #[derive(Clone, Debug, PartialEq)]
    struct LogicalOrColumn<T: CasaDataType + Default> {
        value: T,
    }

    impl<T: CasaDataType + Default + BitOrAssign> LogicalOrColumn<T> {
        fn new() -> Self {
            Self { value: T::default() }
        }

        fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
            let cur = row.get_cell(col_name)?;
            self.value |= cur;
            Ok(())
        }

        fn emit(&self, col_name: &str, _vis_factor: &MaybeVisFactor,
                table: &mut Table, row: u64) -> Result<()>
        {
            table.put_cell(col_name, row, &self.value)
        }

        fn reset(&mut self) {
            self.value = T::default();
        }
    }


    /// The cells in this column are expected to be filled with 2D arrays that
    /// have a polarization and frequency axis.
    #[derive(Clone, Debug, PartialEq)]
    struct PolConcatColumn<T: CasaScalarData> {
        buf: Array<T, Ix2>
    }

    fn process_pol_concat_record<T>(col_name: &str, in_spw: &InputSpwInfo,
                                    out_spw: &OutputSpwInfo, row: &mut TableRow,
                                    buf: &mut Array<T, Ix2>) -> Result<()>
        where T: CasaScalarData + Copy + Default + std::fmt::Debug
    {
        let chunk: Array<T, Ix2> = row.get_cell(col_name)?;

        let n_chunk_chan = chunk.shape()[0];
        let n_chunk_pol = chunk.shape()[1];
        let n_buf_chan = buf.shape()[0];
        let n_buf_pol = buf.shape()[1];

        if n_buf_chan != out_spw.num_chans() || n_buf_pol != n_chunk_pol {
            *buf = Array::default((out_spw.num_chans(), n_chunk_pol));
        }

        let c0 = in_spw.out_spw_offset() as isize;
        buf.slice_mut(s![c0..c0+n_chunk_chan as isize, ..]).assign(&chunk);

        Ok(())
    }

    impl<T: CasaScalarData + Copy + Default + std::fmt::Debug> PolConcatColumn<T>
        where Array<T, Ix2>: CasaDataType
    {
        fn new() -> Self {
            Self {
                buf: Array::default((0, 0))
            }
        }

        fn process(&mut self, col_name: &str, in_spw: &InputSpwInfo,
                   out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()>
        {
            process_pol_concat_record(col_name, in_spw, out_spw, row, &mut self.buf)
        }

        fn emit(&self, col_name: &str, _vis_factor: &MaybeVisFactor,
                table: &mut Table, row: u64) -> Result<()>
        {
            table.put_cell(col_name, row, &self.buf)
        }

        fn reset(&mut self) {
            // We live dangerously and don't de-initialize the buffer, for speed.
        }
    }


    /// This is just like PolConcatColumn, except we might also multiply the
    /// final result by the inverse squared mean bandpass before writing the
    /// output.
    #[derive(Clone, Debug, PartialEq)]
    struct VisibilitiesColumn<T> {
        buf: Array<Complex<f32>, Ix2>,

        // Hack so that we still take a type parameter to make life easier
        // with our macro system.
        _nope: PhantomData<T>,
    }

    impl<T> VisibilitiesColumn<T> {
        fn new() -> Self {
            Self {
                buf: Array::default((0, 0)),
                _nope: PhantomData
            }
        }

        fn process(&mut self, col_name: &str, in_spw: &InputSpwInfo,
                   out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()>
        {
            process_pol_concat_record(col_name, in_spw, out_spw, row, &mut self.buf)
        }

        fn emit(&mut self, col_name: &str, vis_factor: &MaybeVisFactor,
                table: &mut Table, row: u64) -> Result<()> {
            if let &Some(ref arr) = vis_factor {
                self.buf *= &arr.view().into_shape((arr.len(), 1)).unwrap();
            }

            table.put_cell(col_name, row, &self.buf)
        }

        fn reset(&mut self) {
            // We live dangerously and don't de-initialize the buffer, for speed.
        }
    }


    /// This macro generates the column-handling enum for the main visibility
    /// data columns.
    macro_rules! vis_data_columns {
        {$($variant_name:ident($col_name:ident, $state_type:ident, $data_type:ty)),+} => {
            /// This enumeration type represents a column that may appear in the
            /// main table of a visibility data measurement set. We have to use an
            /// enumeration type so that we can leverage Rust's generics to read
            /// in the data with strong, correct typing.
            #[derive(Clone, Debug, PartialEq)]
            enum VisDataColumn {
                $(
                    $variant_name($state_type<$data_type>),
                )+
            }

            impl VisDataColumn {
                fn col_name(&self) -> &'static str {
                    match self {
                        $(
                            &VisDataColumn::$variant_name(_) => stringify!($col_name),
                        )+
                    }
                }

                fn process(&mut self, in_spw: &InputSpwInfo, out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
                    let col_name = self.col_name();

                    Ok(ctry!(match self {
                        $(
                            &mut VisDataColumn::$variant_name(ref mut s) => s.process(col_name, in_spw, out_spw, row),
                        )+
                    }; "problem processing column {}", col_name))
                }

                fn emit(&mut self, vis_factor: &MaybeVisFactor, table: &mut Table, row: u64) -> Result<()> {
                    let col_name = self.col_name();

                    Ok(ctry!(match self {
                        $(
                            &mut VisDataColumn::$variant_name(ref mut s) => s.emit(col_name, vis_factor, table, row),
                        )+
                    }; "problem emitting column {}", col_name))
                }

                fn reset(&mut self) {
                    match self {
                        $(
                            &mut VisDataColumn::$variant_name(ref mut s) => s.reset(),
                        )+
                    }
                }
            }

            impl FromStr for VisDataColumn {
                type Err = Error;

                fn from_str(s: &str) -> Result<Self> {
                    match s {
                        $(
                            stringify!($col_name) => Ok(VisDataColumn::$variant_name($state_type::new())),
                        )+
                            _ => err_msg!("unrecognized column in visibility data table: \"{}\"", s)
                    }
                }
            }
        };
    }

    vis_data_columns! {
        Antenna1(ANTENNA1, IdentityColumn, i32),
        Antenna2(ANTENNA2, IdentityColumn, i32),
        ArrayId(ARRAY_ID, IdentityColumn, i32),
        CorrectedData(CORRECTED_DATA, VisibilitiesColumn, ()),
        DataDescId(DATA_DESC_ID, IdentityColumn, i32),
        Data(DATA, VisibilitiesColumn, ()),
        Exposure(EXPOSURE, ApproxMatchColumn, f64),
        Feed1(FEED1, IdentityColumn, i32),
        Feed2(FEED2, IdentityColumn, i32),
        FieldId(FIELD_ID, IdentityColumn, i32),
        FlagCategory(FLAG_CATEGORY, EmptyColumn, Vec<bool>),
        FlagRow(FLAG_ROW, LogicalOrColumn, bool),
        Flag(FLAG, PolConcatColumn, bool),
        Interval(INTERVAL, ApproxMatchColumn, f64),
        ModelData(MODEL_DATA, VisibilitiesColumn, ()),
        ObservationId(OBSERVATION_ID, IdentityColumn, i32),
        ProcessorId(PROCESSOR_ID, IdentityColumn, i32),
        ScanNumber(SCAN_NUMBER, IdentityColumn, i32),
        Sigma(SIGMA, ApproxMatchColumn, Vec<f32>),
        StateId(STATE_ID, IdentityColumn, i32),
        TimeCentroid(TIME_CENTROID, ApproxMatchColumn, f64),
        Time(TIME, IdentityColumn, f64),
        Uvw(UVW, ApproxMatchColumn, Vec<f64>),
        WeightSpectrum(WEIGHT_SPECTRUM, PolConcatColumn, f32),
        Weight(WEIGHT, ApproxMatchColumn, Vec<f32>)
    }


    // Quick wrapper type to avoid type visibility complaints

    #[derive(Clone, Debug, PartialEq)]
    pub struct WrappedVisDataColumn(VisDataColumn);

    impl FromStr for WrappedVisDataColumn {
        type Err = Error;

        #[inline(always)]
        fn from_str(s: &str) -> Result<Self> {
            Ok(WrappedVisDataColumn(s.parse()?))
        }
    }

    impl WrappedVisDataColumn {
        #[inline(always)]
        pub fn process(&mut self, in_spw: &InputSpwInfo, out_spw: &OutputSpwInfo,
                       row: &mut TableRow) -> Result<()>
        {
            self.0.process(in_spw, out_spw, row)
        }

        #[inline(always)]
        pub fn emit(&mut self, vis_factor: &MaybeVisFactor, table: &mut Table, row: u64) -> Result<()> {
            self.0.emit(vis_factor, table, row)
        }

        #[inline(always)]
        pub fn reset(&mut self) {
            self.0.reset()
        }
    }
}

use main_table::WrappedVisDataColumn as VisDataColumn;


// DATA_DESC_ID is the one that we ignore because that encodes the SPW
// information. TODO: POLARIZATION_ID is hidden in DATA_DESC_ID and we
// could/should multiplex on that, but we currently hardcode a limitation to
// just one POLARIZATION_ID anyway.

#[derive(Clone,Debug,Eq,Hash,PartialEq)]
struct VisRecordIdentity<T: Clone + std::fmt::Debug + Eq + std::hash::Hash> {
    discriminant: T,

    antenna1: i32,
    antenna2: i32,
    array_id: i32,
    feed1: i32,
    feed2: i32,
    field_id: i32,
    observation_id: i32,
    processor_id: i32,
    scan_number: i32,
    state_id: i32,

    /// Needed since f64 is not Hash or Eq; we take care to ensure bitwise equality below.
    recast_time: u64,
}

impl<T: Clone + std::fmt::Debug + Eq + std::hash::Hash> VisRecordIdentity<T> {
    pub fn create(discriminant: T, row: &mut TableRow, last_time: f64) -> Result<Self> {
        let mut time: f64 = row.get_cell("TIME")?;

        // Times are seconds since MJD=0, so they have magnitudes of about
        // several billion. Fractional variations of 1e-11 represent ~0.1
        // second variations which seems like the right tolerance.
        if ((time - last_time) / time).abs() < 1e-11 {
            time = last_time;
        }

        Ok(Self {
            discriminant: discriminant.clone(),
            antenna1: row.get_cell("ANTENNA1")?,
            antenna2: row.get_cell("ANTENNA2")?,
            array_id: row.get_cell("ARRAY_ID")?,
            feed1: row.get_cell("FEED1")?,
            feed2: row.get_cell("FEED2")?,
            field_id: row.get_cell("FIELD_ID")?,
            observation_id: row.get_cell("OBSERVATION_ID")?,
            processor_id: row.get_cell("PROCESSOR_ID")?,
            scan_number: row.get_cell("SCAN_NUMBER")?,
            state_id: row.get_cell("STATE_ID")?,
            recast_time: unsafe { std::mem::transmute(time) },
        })
    }
}


/// Information about an output spectral window.
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct OutputSpwInfo {
    in_spw0: usize,
    in_spw1: usize,
    num_chans: usize,
}

impl OutputSpwInfo {
    pub fn n_input_spws(&self) -> usize {
        self.in_spw1 + 1 - self.in_spw0
    }

    pub fn spw_indices(&self) -> Range<usize> {
        self.in_spw0..(self.in_spw1 + 1)
    }

    pub fn max_spw(&self) -> usize {
        self.in_spw1
    }

    pub fn num_chans(&self) -> usize {
        self.num_chans
    }

    pub fn register_new_input_spw(&mut self, num_chans: usize, out_spw_num: usize) -> InputSpwInfo {
        let rv = InputSpwInfo::new(out_spw_num, self.num_chans);
        self.num_chans += num_chans;
        rv
    }
}

impl FromStr for OutputSpwInfo {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let pieces: Vec<_> = s.split("-").collect();

        if pieces.len() != 2 {
            return err_msg!("expected one dash");
        }

        let i0 = pieces[0].parse::<usize>()?;
        let i1 = pieces[1].parse::<usize>()?;

        // Note that i0 cannot be negative because it is parsed as a usize.
        if i0 > i1 {
            return err_msg!("first spw may not be bigger than second spw");
        }

        Ok(OutputSpwInfo {
            in_spw0: i0,
            in_spw1: i1,
            num_chans: 0,
        })
    }
}


/// Information about each *input* spw. We currently hardcode the limitation
/// that each input spw can only appear in one output spw, which is something
/// that we could in principle lift.
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct InputSpwInfo {
    out_spw: usize,
    offset: usize,
}

impl InputSpwInfo {
    pub fn new(out_spw: usize, offset: usize) -> Self {
        InputSpwInfo {
            out_spw: out_spw,
            offset: offset,
        }
    }

    pub fn out_spw_id(&self) -> usize {
        self.out_spw
    }

    pub fn out_spw_offset(&self) -> usize {
        self.offset
    }
}


/// Internal state of a partially-glued output spectral window.
#[derive(Debug)]
struct OutputRecordState<'a> {
    spw_info: &'a OutputSpwInfo,
    n_input_spws_seen: usize,
    columns: Vec<VisDataColumn>,
}

impl<'a> OutputRecordState<'a> {
    pub fn new(spw_info: &'a OutputSpwInfo, columns: Vec<VisDataColumn>) -> Self {
        Self {
            spw_info: spw_info,
            n_input_spws_seen: 0,
            columns: columns,
        }
    }

    /// Returns true if this row represents the final spectral window needed
    /// to complete this record.
    pub fn process(&mut self, in_spw: &InputSpwInfo, row: &mut TableRow) -> Result<bool> {
        for col in &mut self.columns {
            col.process(in_spw, self.spw_info, row)?;
        }

        self.n_input_spws_seen += 1;
        Ok(self.n_input_spws_seen == self.spw_info.n_input_spws())
    }

    pub fn emit(&mut self, vis_factor: &MaybeVisFactor, table: &mut Table, row: u64) -> Result<()> {
        for col in &mut self.columns {
            col.emit(vis_factor, table, row)?;
        }

        Ok(())
    }

    pub fn reset(mut self, spw_info: &'a OutputSpwInfo) -> Self {
        self.spw_info = spw_info;
        self.n_input_spws_seen = 0;

        for col in &mut self.columns {
            col.reset();
        }

        self
    }
}


// Let's get this show on the road.

fn main() {
    let matches = App::new("spwglue")
        .version("0.1.0")
        .rubbl_notify_args()
        .arg(Arg::with_name("window")
             .short("w")
             .long("window")
             .long_help("Define a glued spectral window that concatenates \
                         input windows numbers N through M, inclusive. The \
                         numbers are zero-based.")
             .value_name("N-M")
             .takes_value(true)
             .number_of_values(1)
             .required(true)
             .multiple(true))
        .arg(Arg::with_name("meanbp")
             .long("meanbp")
             .help("Path a .npy save file with mean bandpass")
             .value_name("PATH")
             .takes_value(true)
             .number_of_values(1))
        .arg(Arg::with_name("out_field")
             .short("f")
             .long("field")
             .long_help("Output data from field FIELDNUM into file OUTPATH")
             .value_name("FIELDNUM OUTPATH")
             .takes_value(true)
             .number_of_values(2)
             .multiple(true))
        .arg(Arg::with_name("out_default")
             .short("D")
             .long("default")
             .long_help("Output any data not associated with a `-f` argument into file OUTPATH")
             .value_name("OUTPATH")
             .takes_value(true)
             .number_of_values(1))
        .arg(Arg::with_name("IN-TABLE")
             .help("The path of the input data set")
             .required(true)
             .index(1))
        .get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(matches, |matches, _nbe| -> Result<i32> {
        // Deal with args. The field mapping is awkward because clap doesn't
        // distinguish between multiple appearances of the same option; `-f A
        // B C -f D` is just returned to us as a list [A B C D]. Therefore to
        // enable the "field hack" mode we have to pay attention for when
        // multiple `-f` options specify the same output path. I think there's
        // some Iterator interface to bunch the items into groups of two but I
        // can't find it right now.

        let inpath_os = matches.value_of_os("IN-TABLE").unwrap();
        let inpath_str = inpath_os.to_string_lossy();
        let inpath = Path::new(inpath_os).to_owned();

        let mut destinations = Vec::new();
        let mut field_id_to_dest_index = HashMap::new();
        let mut dest_path_to_dest_index = HashMap::new();

        if let Some(out_field_items) = matches.values_of("out_field") {
            let mut field_item_is_id = true;
            let mut last_field_id = 0i32;

            for info in out_field_items {
                if field_item_is_id {
                    last_field_id = ctry!(info.parse();
                                          "bad field ID \"{}\" in field output arguments", info);
                } else {
                    let dest = Path::new(info).to_owned();
                    let mut idx = destinations.len();

                    if let Some(prev_idx) = dest_path_to_dest_index.insert(dest.clone(), idx) {
                        // This dest path already appeared; re-use its entry. This lets us
                        // write multiple fields to the same output file.
                        idx = prev_idx;
                    } else {
                        destinations.push(dest);
                    }

                    if field_id_to_dest_index.insert(last_field_id, idx).is_some() {
                        return err_msg!("field ID {} appears multiple times in field output arguments", last_field_id);
                    }
                }

                field_item_is_id = !field_item_is_id;
            }
        }

        let default_dest_index = matches.value_of_os("out_default").map(|info| {
            let dest = Path::new(info).to_owned();
            let mut idx = destinations.len();

            if let Some(prev_idx) = dest_path_to_dest_index.insert(dest.clone(), idx) {
                idx = prev_idx;
            } else {
                destinations.push(dest);
            }

            idx
        });

        if destinations.len() == 0 {
            return err_msg!("must specify at least one destination path with `-f` or `-D`");
        }

        let mut out_spws = Vec::new();

        for descr in matches.values_of("window").unwrap() {
            let m = ctry!(descr.parse::<OutputSpwInfo>();
                          "bad window specification; they should have the form \"M-N\" where M \
                           and N are numbers, but I got \"{}\"", descr);
            out_spws.push(m);
        }

        let inv_sq_mean_bp = match matches.value_of_os("meanbp") {
            None => None,
            Some(meanbp_path) => {
                use itertools::Itertools;
                use itertools::FoldWhile::{Continue, Done};

                let mut meanbp = File::open(&meanbp_path)?;
                let mut arr: Array<f64, Ix1> = npy_stream_to_ndarray(&mut meanbp)?;

                if arr.iter().fold_while(false, |_acc, x| {
                    if *x <= 0. {
                        Done(true)
                    } else {
                        Continue(false)
                    }
                }).into_inner() {
                    return err_msg!("illegal bandpass file \"{}\": some values are nonpositive",
                                    meanbp_path.to_string_lossy());
                }

                arr.mapv_inplace(|x| x.powi(-2));
                Some(arr.map(|x| Complex::new(*x as f32, 0.)))
            },
        };

        // Copy the basic table structure.

        fn open_table(base: &Path, extension: &str, is_input: bool) -> Result<(PathBuf, Table)> {
            let mut p = base.to_owned();

            if extension.len() > 0 {
                p.push(extension);
            }

            let mode = if is_input { TableOpenMode::Read } else { TableOpenMode::ReadWrite };

            let t = ctry!(Table::open(&p, mode);
                          "failed to open {} {}table \"{}\"",
                          if is_input { "input" } else { "output" },
                          if extension.len() > 0 { "sub-" } else { "" },
                          p.display()
            );

            Ok((p, t))
        }

        let (_, mut in_main_table) = open_table(&inpath, "", true)?;

        for dest in &destinations {
            ctry!(in_main_table.deep_copy_no_rows(&dest.to_string_lossy());
                  "failed to copy the structure of table \"{}\" to new table \"{}\"",
                  inpath_str, dest.display());
        }

        // Copy POLARIZATION. We currently require that there be only one
        // polarization type in the input file. This tool *could* work with
        // multiple pol types, but it would be more of a hassle and my data
        // don't currently have that structure. But I've separated out the
        // relevant code here rather than grouped it in with the rest of
        // the miscellaneous tables Just In Case.

        {
            let (_, mut in_pol_table) = open_table(&inpath, "POLARIZATION", true)?;

            let n_pol_types = in_pol_table.n_rows();

            if n_pol_types != 1 {
                return err_msg!("input data set has {} \"POLARIZATION\" rows; I require exactly 1", n_pol_types);
            }

            for dest in &destinations {
                let (_, mut out_pol_table) = open_table(dest, "POLARIZATION", false)?;
                in_pol_table.copy_rows_to(&mut out_pol_table)?;
            }
        };

        // Process the SPECTRAL_WINDOW table, building up our database of
        // information about how to map input spectral windows to output
        // spectral windows.

        let mut in_spws: HashMap<usize, InputSpwInfo> = HashMap::new();

        {
            let (in_spw_path, mut in_spw_table) = open_table(&inpath, "SPECTRAL_WINDOW", true)?;

            let n_in_spws = in_spw_table.n_rows();

            for m in &out_spws {
                if m.max_spw() >= n_in_spws as usize {
                    return err_msg!("you asked to map window #{} but the maximum number is {}",
                                    m.max_spw(), n_in_spws - 1);
                }
            }

            let col_names = ctry!(in_spw_table.column_names();
                                  "failed to get names of columns in \"{}\"", in_spw_path.display());

            // Process everything into the first destination.

            let (out_spw_path, mut out_spw_table) = open_table(&destinations[0], "SPECTRAL_WINDOW", false)?;

            ctry!(out_spw_table.add_rows(out_spws.len());
                  "failed to add {} rows to \"{}\"", out_spws.len(), out_spw_path.display());

            for n in col_names {
                let handler = ctry!(n.parse::<SpectralWindowColumn>();
                                    "unhandled column \"{}\" in input sub-table \"{}\"",
                                    n, in_spw_path.display());
                ctry!(handler.process(&mut in_spw_table, &out_spws, &mut out_spw_table);
                      "failed to fill output sub-table \"{}\"", out_spw_path.display());

                if n == "NUM_CHAN" {
                    // A bit inefficient since we reread the column but whatever.
                    let num_chans = in_spw_table.get_col_as_vec::<i32>(&n)?;

                    for (i, out_spw) in out_spws.iter_mut().enumerate() {
                        for in_spw_num in out_spw.spw_indices() {
                            let ism = out_spw.register_new_input_spw(num_chans[in_spw_num] as usize, i);
                            if in_spws.insert(in_spw_num, ism).is_some() {
                                return err_msg!("cannot (currently) include a single input spw in multiple output spws");
                            }
                        }
                    }
                }
            }

            // Consistency check.

            if let Some(ref arr) = inv_sq_mean_bp {
                for (i, out_spw) in out_spws.iter().enumerate() {
                    if out_spw.num_chans != arr.len() {
                        return err_msg!("all output spws must have {} channels to match the meanbp \
                                         file, but #{} has {} channels", arr.len(), i, out_spw.num_chans);
                    }
                }
            }

            // Now propagate into remaining destinations (if any).

            for more_dest in &destinations[1..] {
                let (_, mut more_spw_table) = open_table(more_dest, "SPECTRAL_WINDOW", false)?;
                out_spw_table.copy_rows_to(&mut more_spw_table)?;
            }
        }

        // Now process DATA_DESCRIPTION. By the restriction to one
        // polarization type, there should be exactly one row per input spw.
        // If we were being fancy we'd reuse the infrastructure that merges
        // the various SPECTRAL_WINDOW columns, but this table only has three
        // columns.

        let mut ddid_to_in_spw_id = HashMap::new();

        {
            let (in_ddid_path, mut in_ddid_table) = open_table(&inpath, "DATA_DESCRIPTION", true)?;

            if in_ddid_table.n_rows() != in_spws.len() as u64 {
                return err_msg!("consistency failure: expected {} rows in input sub-table \"{}\"; got {}",
                                in_spws.len(), in_ddid_path.display(), in_ddid_table.n_rows());
            }

            let flag_row = in_ddid_table.get_col_as_vec::<bool>("FLAG_ROW")?;
            let pol_id = in_ddid_table.get_col_as_vec::<i32>("POLARIZATION_ID")?;
            let spw_id = in_ddid_table.get_col_as_vec::<i32>("SPECTRAL_WINDOW_ID")?;

            // Process everything into first destination.

            let (out_ddid_path, mut out_ddid_table) = open_table(&destinations[0], "DATA_DESCRIPTION", false)?;

            ctry!(out_ddid_table.add_rows(out_spws.len());
                  "failed to add {} rows to \"{}\"", out_spws.len(), out_ddid_path.display());

            for (out_spw_idx, out_spw) in out_spws.iter().enumerate() {
                let mut spw_idx_iter = out_spw.spw_indices();
                let first_in_spw = spw_idx_iter.next().unwrap();

                let the_flag_row = flag_row[first_in_spw];

                if pol_id[first_in_spw] != 0 {
                    return err_msg!("consistency failure: expected POLARIZATION_ID[{}] = 0; got {}",
                                    first_in_spw, pol_id[first_in_spw]);
                }

                if spw_id[first_in_spw] as usize != first_in_spw {
                    return err_msg!("consistency failure: expected SPECTRAL_WINDOW_ID[{}] = {}; got {}",
                                    first_in_spw, first_in_spw, spw_id[first_in_spw]);
                }

                // note: baking in assumption that DDID = SPWID (also done in insert call below)
                ddid_to_in_spw_id.insert(first_in_spw, first_in_spw);

                for in_spw in spw_idx_iter {
                    if pol_id[in_spw] != 0 {
                        return err_msg!("consistency failure: expected POLARIZATION_ID[{}] = 0; got {}",
                                        in_spw, pol_id[in_spw]);
                    }

                    if spw_id[in_spw] as usize != in_spw {
                        return err_msg!("consistency failure: expected SPECTRAL_WINDOW_ID[{}] = {}; got {}",
                                        in_spw, in_spw, spw_id[in_spw]);
                    }

                    ddid_to_in_spw_id.insert(in_spw, in_spw);
                }

                out_ddid_table.put_cell("FLAG_ROW", out_spw_idx as u64, &the_flag_row)?;
                out_ddid_table.put_cell("POLARIZATION_ID", out_spw_idx as u64, &0i32)?;
                out_ddid_table.put_cell("SPECTRAL_WINDOW_ID", out_spw_idx as u64, &(out_spw_idx as i32))?;
            }

            // Now propagate into remaining destinations (if any).

            for more_dest in &destinations[1..] {
                let (_, mut more_ddid_table) = open_table(more_dest, "DATA_DESCRIPTION", false)?;
                out_ddid_table.copy_rows_to(&mut more_ddid_table)?;
            }
        }

        // SOURCE table also needs some custom processing.

        {
            let (in_src_path, mut in_src_table) = open_table(&inpath, "SOURCE", true)?;

            let n_source_rows = in_src_table.n_rows() as usize;
            let n_sources = n_source_rows / in_spws.len();

            if n_sources * in_spws.len() != n_source_rows {
                return err_msg!("consistency failure: expected {} rows in input sub-table \"{}\"; got {}",
                                n_sources * in_spws.len(), in_src_path.display(), n_source_rows);
            }

            // First destination ...

            let (out_src_path, mut out_src_table) = open_table(&destinations[0], "SOURCE", false)?;

            ctry!(out_src_table.add_rows(n_sources * out_spws.len());
                  "failed to add {} rows to \"{}\"", n_sources * out_spws.len(), out_src_path.display());

            let mut out_row = out_src_table.get_row_writer()?;
            let mut n_rows_written = 0;

            in_src_table.for_each_row(|in_row| {
                let srcid = in_row.get_cell::<i32>("SOURCE_ID")?;
                let spwid = in_row.get_cell::<i32>("SPECTRAL_WINDOW_ID")?;

                // We assume, but don't verify, that all other columns are
                // stable, and so our task here is simply one of filtering the
                // source table.

                if (srcid as usize) < n_sources && (spwid as usize) < out_spws.len() {
                    in_row.copy_and_put(&mut out_row, n_rows_written)?;
                    n_rows_written += 1;
                }

                Ok(())
            })?;

            // The rest.

            for more_dest in &destinations[1..] {
                let (_, mut more_src_table) = open_table(more_dest, "SOURCE", false)?;
                out_src_table.copy_rows_to(&mut more_src_table)?;
            }
        }

        // Copy over the remaining sub-tables as-is.

        let table_kw_names = ctry!(in_main_table.table_keyword_names();
                                   "failed to get keyword info in \"{}\"", inpath.display());

        for kw_name in &table_kw_names {
            match kw_name.as_str() {
                "DATA_DESCRIPTION" => {},
                "POLARIZATION" => {},
                "SOURCE" => {},
                "SPECTRAL_WINDOW" => {},
                "SYSPOWER" => {}, // large and my pipeline pre-applies it!
                n => {
                    let (_, mut in_misc_table) = open_table(&inpath, n, true)?;

                    for dest in &destinations {
                        let (_, mut out_misc_table) = open_table(dest, n, false)?;
                        in_misc_table.copy_rows_to(&mut out_misc_table)?;
                    }
                },
            }
        }

        // Finally, the main visibility data.

        let col_names = ctry!(in_main_table.column_names();
                              "failed to get names of columns in \"{}\"", inpath.display());
        let mut col_state_template = Vec::new();

        for n in col_names {
            let handler = ctry!(n.parse::<VisDataColumn>();
                                "unhandled column \"{}\" in input table \"{}\"", n, inpath.display());
            col_state_template.push(handler);
        }

        let mut records_in_progress = HashMap::new();
        let mut state_pool: Vec<OutputRecordState> = Vec::new();
        let mut last_time = 0f64;
        let mut in_row_num = 0usize;
        let mut pb = pbr::ProgressBar::new(in_main_table.n_rows());
        pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(500)));

        struct DestinationRecord<'a> {
            path: &'a Path,
            table: Table,
            num_rows: u64,
        }

        let mut out_tables = Vec::with_capacity(destinations.len());

        for dest in &destinations {
            let (_, t) = open_table(dest, "", false)?;
            out_tables.push(DestinationRecord {
                path: dest,
                table: t,
                num_rows: 0
            });
        }

        in_main_table.for_each_row(|mut in_row| {
            let ddid = in_row.get_cell::<i32>("DATA_DESC_ID")?;
            let fieldid = in_row.get_cell::<i32>("FIELD_ID")?;
            let in_spw_id = *ddid_to_in_spw_id.get(&(ddid as usize)).unwrap();
            let in_spw_info = in_spws.get(&in_spw_id).unwrap();
            let out_spw_id = in_spw_info.out_spw_id();
            let row_ident = VisRecordIdentity::create(out_spw_id, &mut in_row, last_time)?;

            if !records_in_progress.contains_key(&row_ident) {
                let state = match state_pool.pop(){
                    Some(s) => s.reset(&out_spws[out_spw_id]),
                    None => OutputRecordState::new(&out_spws[out_spw_id], col_state_template.clone())
                };

                records_in_progress.insert(row_ident.clone(), state);
            }

            let record_complete = {
                let state = records_in_progress.get_mut(&row_ident).unwrap();
                ctry!(state.process(in_spw_info, &mut in_row);
                      "problem processing input row #{}", in_row_num)
            };

            if record_complete {
                let mut state = records_in_progress.remove(&row_ident).unwrap();

                let maybe_out_rec = if let Some(idx) = field_id_to_dest_index.get_mut(&fieldid) {
                    Some(&mut out_tables[*idx])
                } else if let Some(ddi) = default_dest_index {
                    Some(&mut out_tables[ddi])
                } else {
                    None
                };

                if let Some(out_rec) = maybe_out_rec {
                    ctry!(out_rec.table.add_rows(1); "failed to add row to \"{}\"", out_rec.path.display());
                    state.emit(&inv_sq_mean_bp, &mut out_rec.table, out_rec.num_rows)?;
                    // Rewriting this is kind of lame, but eh.
                    out_rec.table.put_cell("DATA_DESC_ID", out_rec.num_rows, &(out_spw_id as i32))?;
                    out_rec.num_rows += 1;
                }

                state_pool.push(state);
            }

            last_time = in_row.get_cell("TIME")?;
            in_row_num += 1;
            pb.inc();
            Ok(())
        })?;

        // All done!

        pb.finish();
        Ok(0)
    }));
}
