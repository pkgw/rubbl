// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

#[macro_use] extern crate ndarray;
extern crate num_traits;
extern crate rubbl_casatables;
#[macro_use] extern crate rubbl_core;
extern crate clap;

use clap::{App, Arg};
use num_traits::{Float, One, Signed, Zero};
use rubbl_casatables::{CasaDataType, CasaScalarData, Table, TableOpenMode, TableRow};
use rubbl_casatables::errors::{Error, Result};
use rubbl_core::{Array, Complex, Ix2};
use rubbl_core::notify::ClapNotificationArgsExt;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::{AddAssign, BitOrAssign, Range, Sub};
use std::path::Path;
use std::process;
use std::str::FromStr;


// Types for combining spw-associated quantities. We have to implement these
// as discrete types so that we can leverage Rust's generics. It's a bit of a
// shame that we can't handle the options purely with data tables, but
// type-based column reading has a lot of advantages more generally.

/// Columns handled by this struct are simply ignored.
struct SpwIgnoreColumn<T> { _nope: PhantomData<T> }

impl<T> SpwIgnoreColumn<T> {
    pub fn new() -> Self { Self { _nope: PhantomData } }

    pub fn process(&self, _src_table: &mut Table, _col_name: &str,
                   _mappings: &[OutputSpwInfo], _dest_table: &mut Table) -> Result<()> {
        Ok(())
    }
}


/// Columns handled by this struct use the first value that appears.
struct SpwUseFirstColumn<T: CasaScalarData> { _nope: PhantomData<T> }

impl<T: CasaScalarData> SpwUseFirstColumn<T> {
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
struct SpwMustMatchColumn<T: CasaScalarData> { _nope: PhantomData<T> }

impl<T: CasaScalarData + Display> SpwMustMatchColumn<T> {
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
struct SpwSumScalarColumn<T> { _nope: PhantomData<T> }

impl<T: CasaScalarData + AddAssign + Copy + Default> SpwSumScalarColumn<T> {
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
struct SpwConcatVectorColumn<T: CasaScalarData> { _nope: PhantomData<T> }

impl<T: CasaScalarData> SpwConcatVectorColumn<T> where Vec<T>: CasaDataType {
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


// Now we use a macro to create the enum type that handles all of the possible
// columns that might appear in the SPECTRAL_WINDOW table. The enum type is
// kind of awkward, but once you work out the macro magic it becomes fairly
// straightforward to use, and as mentioned before this approach allows us to
// leverage Rust's type system nicely.
//
// Because macros work on an AST level, we can't capture the "state type" of
// each column as a genuine type (e.g. SpwUseFirstColumn<i32>) because we are
// then unable to refer to that type in expression contexts. Therefore we have
// to put our macros all in terms of "ident" typed captures.
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
    AssocNature(ASSOC_NATURE, SpwIgnoreColumn, ()),
    AssocSpwId(ASSOC_SPW_ID, SpwIgnoreColumn, ()),
    BbcNo(BBC_NO, SpwMustMatchColumn, i32),
    ChanFreq(CHAN_FREQ, SpwConcatVectorColumn, f64),
    ChanWidth(CHAN_WIDTH, SpwConcatVectorColumn, f64),
    DopplerId(DOPPLER_ID, SpwUseFirstColumn, i32),
    EffectiveBw(EFFECTIVE_BW, SpwConcatVectorColumn, f64),
    FlagRow(FLAG_ROW, SpwMustMatchColumn, bool),
    FreqGroup(FREQ_GROUP, SpwMustMatchColumn, i32),
    FreqGroupName(FREQ_GROUP_NAME, SpwMustMatchColumn, String),
    IfConvChain(IF_CONV_CHAIN, SpwMustMatchColumn, i32),
    MeasFreqRef(MEAS_FREQ_REF, SpwMustMatchColumn, i32),
    Name(NAME, SpwUseFirstColumn, String),
    NetSideband(NET_SIDEBAND, SpwMustMatchColumn, i32),
    NumChan(NUM_CHAN, SpwSumScalarColumn, i32),
    RefFrequency(REF_FREQUENCY, SpwUseFirstColumn, f64),
    Resolution(RESOLUTION, SpwConcatVectorColumn, f64),
    TotalBandwidth(TOTAL_BANDWIDTH, SpwSumScalarColumn, f64)
}


// Now the same rigamarole for the main visbility data table.

#[derive(Clone, Debug, PartialEq)]
struct VisIdentityColumn<T: CasaScalarData> {
    value: Option<T>,
}

impl<T: CasaScalarData> VisIdentityColumn<T> {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
        // Since this column helps define the record's identity, subsequent rows match the
        // first row by definition.
        if self.value.is_none() {
            self.value = Some(row.get_cell(col_name)?);
        }

        Ok(())
    }

    pub fn emit(&self, col_name: &str, table: &mut Table, row: u64) -> Result<()> {
        if let Some(ref v) = self.value {
            table.put_cell(col_name, row, v)?;
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.value = None;
    }
}


trait CheckApproximateMatch {
    type Element: Float + One + PartialOrd + Signed;

    fn approx_match_tol() -> Self::Element {
        // This is kind of ridiculous, but I can't figure out a way to get a
        // literal constant that's agnostic as to the type T ...
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


#[derive(Clone, Debug, PartialEq)]
struct VisApproxMatchColumn<T: CasaDataType> {
    value: Option<T>,
}

impl<T: CasaDataType + CheckApproximateMatch> VisApproxMatchColumn<T> {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
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
    pub fn emit(&self, col_name: &str, table: &mut Table, row: u64) -> Result<()> {
        if let Some(ref v) = self.value {
            table.put_cell(col_name, row, v)?;
        }

        Ok(())
    }

    pub fn reset(&mut self) {
        self.value = None;
    }
}


#[derive(Clone, Debug, PartialEq)]
struct VisEmptyColumn<T: CasaDataType> {
    _nope: PhantomData<T>
}

impl<T> VisEmptyColumn<Vec<T>> where Vec<T>: CasaDataType {
    pub fn new() -> Self {
        Self { _nope: PhantomData }
    }

    pub fn process(&mut self, _col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, _row: &mut TableRow) -> Result<()> {
        Ok(())
    }

    pub fn emit(&self, _col_name: &str, _table: &mut Table, _row: u64) -> Result<()> {
        Ok(())
    }

    pub fn reset(&mut self) {
    }
}


#[derive(Clone, Debug, PartialEq)]
struct VisLogicalOrColumn<T: CasaDataType + Default> {
    value: T,
}

impl<T: CasaDataType + Default + BitOrAssign> VisLogicalOrColumn<T> {
    pub fn new() -> Self {
        Self { value: T::default() }
    }

    pub fn process(&mut self, col_name: &str, _in_spw: &InputSpwInfo, _out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
        let cur = row.get_cell(col_name)?;
        self.value |= cur;
        Ok(())
    }

    pub fn emit(&self, col_name: &str, table: &mut Table, row: u64) -> Result<()> {
        table.put_cell(col_name, row, &self.value)
    }

    pub fn reset(&mut self) {
        self.value = T::default();
    }
}


#[derive(Clone, Debug, PartialEq)]
struct VisPolConcatColumn<T: CasaScalarData> {
    buf: Array<T, Ix2>
}

impl<T: CasaScalarData + Default + std::fmt::Debug> VisPolConcatColumn<T> where Array<T, Ix2>: CasaDataType {
    pub fn new() -> Self {
        Self {
            buf: Array::default((0, 0))
        }
    }

    pub fn process(&mut self, col_name: &str, in_spw: &InputSpwInfo, out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
        let chunk: Array<T, Ix2> = row.get_cell(col_name)?;

        let n_chunk_chan = chunk.shape()[0];
        let n_chunk_pol = chunk.shape()[1];
        let n_buf_chan = self.buf.shape()[0];
        let n_buf_pol = self.buf.shape()[1];

        if n_buf_chan != out_spw.num_chans() || n_buf_pol != n_chunk_pol {
            self.buf = Array::default((out_spw.num_chans(), n_chunk_pol));
        }

        let c0 = in_spw.out_spw_offset() as isize;
        self.buf.slice_mut(s![c0..c0+n_chunk_chan as isize, ..]).assign(&chunk);

        Ok(())
    }

    pub fn emit(&self, col_name: &str, table: &mut Table, row: u64) -> Result<()> {
        table.put_cell(col_name, row, &self.buf)
    }

    pub fn reset(&mut self) {
        // We live dangerously and don't de-initialize the buffer, for speed.
    }
}


// Now the same sort of macro stuff, for the main visibility data columns.
macro_rules! vis_data_columns {
    {$($variant_name:ident($col_name:ident, $state_type:ident, $data_type:ty)),+} => {
        /// This enumeration type represents a column that may appear in the
        /// main table of a visibility data measurement set. We have to use an
        /// enumeration type so that we can leverage Rust's generics to read
        /// in the data with strong, correct typing.
        #[derive(Clone, Debug, PartialEq)]
        enum VisDataColumnHandler {
            $(
                $variant_name($state_type<$data_type>),
            )+
        }

        impl VisDataColumnHandler {
            pub fn col_name(&self) -> &'static str {
                match self {
                    $(
                        &VisDataColumnHandler::$variant_name(_) => stringify!($col_name),
                    )+
                }
            }

            pub fn process(&mut self, in_spw: &InputSpwInfo, out_spw: &OutputSpwInfo, row: &mut TableRow) -> Result<()> {
                let col_name = self.col_name();

                Ok(ctry!(match self {
                    $(
                        &mut VisDataColumnHandler::$variant_name(ref mut s) => s.process(col_name, in_spw, out_spw, row),
                    )+
                }; "problem processing column {}", col_name))
            }

            pub fn emit(&self, table: &mut Table, row: u64) -> Result<()> {
                let col_name = self.col_name();

                Ok(ctry!(match self {
                    $(
                        &VisDataColumnHandler::$variant_name(ref s) => s.emit(col_name, table, row),
                    )+
                }; "problem emitting column {}", col_name))
            }

            pub fn reset(&mut self) {
                match self {
                    $(
                        &mut VisDataColumnHandler::$variant_name(ref mut s) => s.reset(),
                    )+
                }
            }
        }

        impl FromStr for VisDataColumnHandler {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self> {
                match s {
                    $(
                        stringify!($col_name) => Ok(VisDataColumnHandler::$variant_name($state_type::new())),
                    )+
                    _ => err_msg!("unrecognized column in visibility data table: \"{}\"", s)
                }
            }
        }
    };
}

vis_data_columns! {
    Antenna1(ANTENNA1, VisIdentityColumn, i32),
    Antenna2(ANTENNA2, VisIdentityColumn, i32),
    ArrayId(ARRAY_ID, VisIdentityColumn, i32),
    CorrectedData(CORRECTED_DATA, VisPolConcatColumn, Complex<f32>),
    DataDescId(DATA_DESC_ID, VisIdentityColumn, i32),
    Data(DATA, VisPolConcatColumn, Complex<f32>),
    Exposure(EXPOSURE, VisApproxMatchColumn, f64),
    Feed1(FEED1, VisIdentityColumn, i32),
    Feed2(FEED2, VisIdentityColumn, i32),
    FieldId(FIELD_ID, VisIdentityColumn, i32),
    FlagCategory(FLAG_CATEGORY, VisEmptyColumn, Vec<bool>),
    FlagRow(FLAG_ROW, VisLogicalOrColumn, bool),
    Flag(FLAG, VisPolConcatColumn, bool),
    Interval(INTERVAL, VisApproxMatchColumn, f64),
    ModelData(MODEL_DATA, VisPolConcatColumn, Complex<f32>),
    ObservationId(OBSERVATION_ID, VisIdentityColumn, i32),
    ProcessorId(PROCESSOR_ID, VisIdentityColumn, i32),
    ScanNumber(SCAN_NUMBER, VisIdentityColumn, i32),
    Sigma(SIGMA, VisApproxMatchColumn, Vec<f32>),
    StateId(STATE_ID, VisIdentityColumn, i32),
    TimeCentroid(TIME_CENTROID, VisApproxMatchColumn, f64),
    Time(TIME, VisIdentityColumn, f64),
    Uvw(UVW, VisApproxMatchColumn, Vec<f64>),
    WeightSpectrum(WEIGHT_SPECTRUM, VisPolConcatColumn, f32),
    Weight(WEIGHT, VisApproxMatchColumn, Vec<f32>)
}


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
struct OutputSpwInfo {
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
struct InputSpwInfo {
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
    columns: Vec<VisDataColumnHandler>,
}

impl<'a> OutputRecordState<'a> {
    pub fn new(spw_info: &'a OutputSpwInfo, columns: Vec<VisDataColumnHandler>) -> Self {
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

    pub fn emit(&self, table: &mut Table, row: u64) -> Result<()> {
        for col in &self.columns {
            col.emit(table, row)?;
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
             .long_help("Define a glued spectral window that concatenates
input windows numbers N through M, inclusive. The numbers are
zero-based.")
             .value_name("N-M")
             .takes_value(true)
             .number_of_values(1)
             .required(true)
             .multiple(true))
        .arg(Arg::with_name("IN-TABLE")
             .help("The path of the input data set")
             .required(true)
             .index(1))
        .arg(Arg::with_name("OUT-TABLE")
             .help("The path of the output data set")
             .required(true)
             .index(2))
        .get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(matches, |matches, _nbe| -> Result<i32> {
        // Deal with args.

        let inpath_os = matches.value_of_os("IN-TABLE").unwrap();
        let inpath_str = inpath_os.to_string_lossy();
        let inpath = Path::new(inpath_os).to_owned();
        let outpath_os = matches.value_of_os("OUT-TABLE").unwrap();
        let outpath_str = outpath_os.to_string_lossy();
        let outpath = Path::new(outpath_os).to_owned();

        let mut out_spws = Vec::new();

        for descr in matches.values_of("window").unwrap() {
            let m = ctry!(descr.parse::<OutputSpwInfo>();
                          "bad window specification; they should have the form \"M-N\" where M \
                           and N are numbers, but I got \"{}\"", descr);
            out_spws.push(m);
        }

        // Copy the basic table structure.

        let mut in_main_table = ctry!(Table::open(&inpath, TableOpenMode::Read);
                                      "failed to open input table \"{}\"", inpath_str);

        ctry!(in_main_table.deep_copy_no_rows(&outpath_str);
              "failed to copy the structure of table \"{}\" to new table \"{}\"",
              inpath_str, outpath_str);

        // Copy POLARIZATION. We currently require that there be only one
        // polarization type in the input file. This tool *could* work with
        // multiple pol types, but it would be more of a hassle and my data
        // don't currently have that structure. But I've separated out the
        // relevant code here rather than grouped it in with the rest of
        // the miscellaneous tables Just In Case.

        {
            let mut in_pol_path = inpath.clone();
            in_pol_path.push("POLARIZATION");
            let mut in_pol_table = ctry!(Table::open(&in_pol_path, TableOpenMode::Read);
                                         "failed to open input sub-table \"{}\"", in_pol_path.display());

            let n_pol_types = in_pol_table.n_rows();

            if n_pol_types != 1 {
                return err_msg!("input data set has {} \"POLARIZATION\" rows; I require exactly 1", n_pol_types);
            }

            let mut out_pol_path = outpath.clone();
            out_pol_path.push("POLARIZATION");
            let mut out_pol_table = ctry!(Table::open(&out_pol_path, TableOpenMode::ReadWrite);
                                          "failed to open output sub-table \"{}\"", out_pol_path.display());

            in_pol_table.copy_rows_to(&mut out_pol_table)?;
        };

        // Process the SPECTRAL_WINDOW table, building up our database of
        // information about how to map input spectral windows to output
        // spectral windows.

        let mut in_spws: HashMap<usize, InputSpwInfo> = HashMap::new();

        {
            let mut in_spw_path = inpath.clone();
            in_spw_path.push("SPECTRAL_WINDOW");
            let mut in_spw_table = ctry!(Table::open(&in_spw_path, TableOpenMode::Read);
                                         "failed to open input sub-table \"{}\"", in_spw_path.display());

            let n_in_spws = in_spw_table.n_rows();

            for m in &out_spws {
                if m.max_spw() >= n_in_spws as usize {
                    return err_msg!("you asked to map window #{} but the maximum number is {}",
                                    m.max_spw(), n_in_spws - 1);
                }
            }

            let col_names = ctry!(in_spw_table.column_names();
                                  "failed to get names of columns in \"{}\"", in_spw_path.display());

            let mut out_spw_path = outpath.clone();
            out_spw_path.push("SPECTRAL_WINDOW");
            let mut out_spw_table = ctry!(Table::open(&out_spw_path, TableOpenMode::ReadWrite);
                                          "failed to open output sub-table \"{}\"", out_spw_path.display());

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
        }

        // Now process DATA_DESCRIPTION. By the restriction to one
        // polarization type, there should be exactly one row per input spw.
        // If we were being fancy we'd reuse the infrastructure that merges
        // the various SPECTRAL_WINDOW columns, but this table only has three
        // columns.

        let mut ddid_to_in_spw_id = HashMap::new();

        {
            let mut in_ddid_path = inpath.clone();
            in_ddid_path.push("DATA_DESCRIPTION");
            let mut in_ddid_table = ctry!(Table::open(&in_ddid_path, TableOpenMode::Read);
                                          "failed to open input sub-table \"{}\"", in_ddid_path.display());

            if in_ddid_table.n_rows() != in_spws.len() as u64 {
                return err_msg!("consistency failure: expected {} rows in input sub-table \"{}\"; got {}",
                                in_spws.len(), in_ddid_path.display(), in_ddid_table.n_rows());
            }

            let flag_row = in_ddid_table.get_col_as_vec::<bool>("FLAG_ROW")?;
            let pol_id = in_ddid_table.get_col_as_vec::<i32>("POLARIZATION_ID")?;
            let spw_id = in_ddid_table.get_col_as_vec::<i32>("SPECTRAL_WINDOW_ID")?;

            let mut out_ddid_path = outpath.clone();
            out_ddid_path.push("DATA_DESCRIPTION");
            let mut out_ddid_table = ctry!(Table::open(&out_ddid_path, TableOpenMode::ReadWrite);
                                           "failed to open output sub-table \"{}\"", out_ddid_path.display());

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
        }

        // SOURCE table also needs some custom processing.

        {
            let mut in_src_path = inpath.clone();
            in_src_path.push("SOURCE");
            let mut in_src_table = ctry!(Table::open(&in_src_path, TableOpenMode::Read);
                                          "failed to open input sub-table \"{}\"", in_src_path.display());

            let n_source_rows = in_src_table.n_rows() as usize;
            let n_sources = n_source_rows / in_spws.len();

            if n_sources * in_spws.len() != n_source_rows {
                return err_msg!("consistency failure: expected {} rows in input sub-table \"{}\"; got {}",
                                n_sources * in_spws.len(), in_src_path.display(), n_source_rows);
            }

            let mut out_src_path = outpath.clone();
            out_src_path.push("SOURCE");
            let mut out_src_table = ctry!(Table::open(&out_src_path, TableOpenMode::ReadWrite);
                                           "failed to open output sub-table \"{}\"", out_src_path.display());

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
                    let mut in_misc_path = inpath.clone();
                    in_misc_path.push(n);
                    let mut in_misc_table = ctry!(Table::open(&in_misc_path, TableOpenMode::Read);
                                                 "failed to open input sub-table \"{}\"", in_misc_path.display());

                    let mut out_misc_path = outpath.clone();
                    out_misc_path.push(n);
                    let mut out_misc_table = ctry!(Table::open(&out_misc_path, TableOpenMode::ReadWrite);
                                                  "failed to open output sub-table \"{}\"", out_misc_path.display());

                    in_misc_table.copy_rows_to(&mut out_misc_table)?;
                },
            }
        }

        // Finally, the main visibility data.

        let col_names = ctry!(in_main_table.column_names();
                              "failed to get names of columns in \"{}\"", inpath.display());
        let mut col_state_template = Vec::new();

        for n in col_names {
            let handler = ctry!(n.parse::<VisDataColumnHandler>();
                                "unhandled column \"{}\" in input table \"{}\"", n, inpath.display());
            col_state_template.push(handler);
        }

        let mut records_in_progress = HashMap::new();
        let mut state_pool: Vec<OutputRecordState> = Vec::new();
        let mut last_time = 0f64;
        let mut in_row_num = 0usize;

        let mut out_main_table = ctry!(Table::open(&outpath, TableOpenMode::ReadWrite);
                                      "failed to open output \"{}\"", outpath.display());
        let mut out_row_num = 0;

        in_main_table.for_each_row(|mut in_row| {
            let ddid = in_row.get_cell::<i32>("DATA_DESC_ID")?;
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
                let state = records_in_progress.remove(&row_ident).unwrap();
                ctry!(out_main_table.add_rows(1); "failed to add row to \"{}\"", outpath.display());
                state.emit(&mut out_main_table, out_row_num)?;
                // Rewriting this is kind of lame, but eh.
                out_main_table.put_cell("DATA_DESC_ID", out_row_num, &(out_spw_id as i32))?;
                out_row_num += 1;
                state_pool.push(state);
            }

            last_time = in_row.get_cell("TIME")?;
            in_row_num += 1;
            Ok(())
        })?;

        // All done!

        Ok(0)
    }));
}
