// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD "uv" data sets containing visibility data.

TODO:

- overrides
- writing UV data
- upcasting of data types
- special handling of j-format "corr" variable?

 */

#![allow(dead_code)]

use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use rubbl_core::io::{AligningReader, AligningWriter, OpenResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::slice;

use super::{AnyMiriadValue, DataSet, MiriadMappedType, Type};
use crate::{mask::MaskDecoder, MiriadFormatError};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum ObsType {
    Auto,
    Cross,
    MixedAutoCross,
}

/// Information about a "UV variable" defined in the UV data stream.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UvVariable {
    name: String,
    number: u8,
    ty: Type,
    n_vals: isize,
    data: Vec<u8>,
    just_updated: bool,
}

impl UvVariable {
    fn new(ty: Type, name: &str, number: u8) -> Self {
        UvVariable {
            name: name.to_owned(),
            number,
            ty,
            n_vals: -1,
            data: Vec::new(),
            just_updated: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn type_(&self) -> Type {
        self.ty
    }

    pub fn n_vals(&self) -> isize {
        self.n_vals
    }

    pub fn just_updated(&self) -> bool {
        self.just_updated
    }

    pub fn get_as_any(&self) -> AnyMiriadValue {
        AnyMiriadValue::from_type_and_buf(self.ty, &self.data)
    }

    pub fn as_reference(&self) -> UvVariableReference {
        UvVariableReference(self.number)
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct UvVariableReference(u8);

/// A struct that holds state for decoding the MIRIAD UV data format.
#[derive(Debug)]
pub struct Decoder {
    eff_vislen: u64,
    vars: Vec<UvVariable>,
    vars_by_name: HashMap<String, u8>,
    stream: AligningReader<io::BufReader<File>>,
}

impl Decoder {
    pub fn create(ds: &mut DataSet) -> Result<Self, MiriadFormatError> {
        let vislen = ds.get("vislen").require_found()?.read_scalar::<i64>()?;
        let mut vars = Vec::new();
        let mut vars_by_name = HashMap::new();
        let mut var_num = 0u8;

        for maybe_line in ds.get("vartable").require_found()?.into_lines()? {
            let line = maybe_line?;

            if line.len() < 3 {
                return Err(MiriadFormatError::Generic(format!(
                    "illegal vartable line: {line}"
                )));
            }

            let pieces: Vec<_> = line.split_whitespace().collect();

            if pieces.len() != 2 {
                return Err(MiriadFormatError::Generic(format!(
                    "illegal vartable line: {line}"
                )));
            }

            let ty = Type::try_from_abbrev(pieces[0])?;
            let name = pieces[1];

            vars.push(UvVariable::new(ty, name, var_num));

            // TODO: check for duplicates
            vars_by_name.insert(name.to_owned(), var_num);

            if var_num == 255 {
                return Err(MiriadFormatError::Generic(
                    "too many UV variables".to_string(),
                ));
            }

            var_num += 1;
        }

        let stream = ds.get("visdata").require_found()?.into_byte_stream()?;

        Ok(Decoder {
            eff_vislen: vislen as u64 - 4, // this is always too big
            vars,
            vars_by_name,
            stream,
        })
    }

    /// Get the current position into the bulk visibility data
    pub fn position(&self) -> u64 {
        self.stream.offset()
    }

    /// Get the size of the bulk visibility data file in bytes.
    pub fn visdata_bytes(&self) -> u64 {
        self.eff_vislen
    }

    /// Returns Ok(false) on EOF, Ok(true) if there are more data.
    pub fn next(&mut self) -> Result<bool, MiriadFormatError> {
        let mut keep_going = true;
        let mut header_buf = [0u8; 4];

        for var in &mut self.vars {
            var.just_updated = false;
        }

        while keep_going {
            self.stream.read_exact(&mut header_buf)?;
            let varnum = header_buf[0];
            let entry_type = header_buf[2];

            const SIZE: u8 = 0;
            const DATA: u8 = 1;
            const EOR: u8 = 2;

            match entry_type {
                SIZE => {
                    if varnum as usize >= self.vars.len() {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: too-large variable number".to_string(),
                        ));
                    }

                    let var = &mut self.vars[varnum as usize];
                    let n_bytes = self.stream.read_i32::<BigEndian>()?;

                    if n_bytes < 0 {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: negative data size".to_string(),
                        ));
                    }

                    if n_bytes % var.ty.size() as i32 != 0 {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: non-integral number of elements in array".to_string(),
                        ));
                    }

                    var.n_vals = (n_bytes / (var.ty.size() as i32)) as isize;
                    var.data.resize(n_bytes as usize, 0); // bit of slowness: zeroing out the data
                }
                DATA => {
                    if varnum as usize >= self.vars.len() {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: too-large variable number".to_string(),
                        ));
                    }

                    let var = &mut self.vars[varnum as usize];
                    self.stream.align_to(var.ty.alignment() as usize)?;
                    self.stream.read_exact(&mut var.data)?;
                    var.just_updated = true;
                }
                EOR => {
                    keep_going = false;
                }
                z => {
                    return Err(MiriadFormatError::Generic(format!(
                        "invalid visdata: unrecognized record code {z}"
                    )));
                }
            }

            // The "vislen" variable is what we should use to determine when
            // to stop reading, rather than EOF -- it's insurance to save
            // datasets if some extra vis data are written out when a
            // data-taker crashes. "vislen" should always be set to land on
            // the end of a UV record.

            if self.stream.offset() >= self.eff_vislen {
                return Ok(false);
            }

            self.stream.align_to(8)?;
        }

        Ok(true)
    }

    pub fn variables<'a>(&'a self) -> UvVariablesIterator<'a> {
        UvVariablesIterator(self.vars.iter())
    }

    pub fn lookup_variable(&self, var_name: &str) -> Option<UvVariableReference> {
        self.vars_by_name
            .get(var_name)
            .map(|o| UvVariableReference(*o))
    }

    pub fn get_var(&self, var: UvVariableReference) -> &UvVariable {
        &self.vars[var.0 as usize]
    }

    pub fn get_data<T: MiriadMappedType>(&self, var: UvVariableReference, buf: &mut Vec<T>) {
        let var = &self.vars[var.0 as usize];

        // TODO: upcasting
        if T::TYPE != var.ty {
            panic!("attempting to decode UV variable of incompatible type");
        }

        T::decode_buf_into_vec(&var.data, buf);
    }

    pub fn get_scalar<T: MiriadMappedType>(&self, var: UvVariableReference) -> T {
        let var = &self.vars[var.0 as usize];

        // TODO: upcasting
        if T::TYPE != var.ty {
            panic!("attempting to decode UV variable of incompatible type");
        }

        // TODO: more efficient.
        let mut buf = Vec::new();
        T::decode_buf_into_vec(&var.data, &mut buf);
        buf.swap_remove(0)
    }

    /// Diagnostic helper.
    pub fn dump_diagnostic<W: Write>(&mut self, mut dest: W) -> Result<(), MiriadFormatError> {
        let mut header_buf = [0u8; 4];
        let mut bl_buf = vec![0f32];

        loop {
            self.stream.read_exact(&mut header_buf)?;
            let varnum = header_buf[0];
            let entry_type = header_buf[2];

            const SIZE: u8 = 0;
            const DATA: u8 = 1;
            const EOR: u8 = 2;

            match entry_type {
                SIZE => {
                    if varnum as usize >= self.vars.len() {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: too-large variable number".to_string(),
                        ));
                    }

                    let var = &mut self.vars[varnum as usize];
                    let n_bytes = self.stream.read_i32::<BigEndian>()?;

                    if n_bytes < 0 {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: negative data size".to_string(),
                        ));
                    }

                    if n_bytes % var.ty.size() as i32 != 0 {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: non-integral number of elements in array".to_string(),
                        ));
                    }

                    var.n_vals = (n_bytes / (var.ty.size() as i32)) as isize;
                    var.data.resize(n_bytes as usize, 0); // bit of slowness: zeroing out the data
                    writeln!(dest, "size {}({}) = {}", var.name, varnum, var.n_vals)?;
                }

                DATA => {
                    if varnum as usize >= self.vars.len() {
                        return Err(MiriadFormatError::Generic(
                            "invalid visdata: too-large variable number".to_string(),
                        ));
                    }

                    let var = &mut self.vars[varnum as usize];
                    self.stream.align_to(var.ty.alignment() as usize)?;
                    self.stream.read_exact(&mut var.data)?;

                    if var.name == "baseline" {
                        f32::decode_buf_into_vec(&var.data, &mut bl_buf);
                        let bl = decode_baseline(bl_buf[0])?;
                        writeln!(
                            dest,
                            "data {}({}) = {} [{}-{}]",
                            var.name, varnum, bl_buf[0], bl.0, bl.1
                        )?;
                    } else {
                        writeln!(dest, "data {}({}) = {}", var.name, varnum, var.get_as_any())?;
                    }
                }

                EOR => {
                    writeln!(dest, "-- EOR --")?;
                }

                z => {
                    return Err(MiriadFormatError::Generic(format!(
                        "invalid visdata: unrecognized record code {z}"
                    )));
                }
            }

            // The "vislen" variable is what we should use to determine when
            // to stop reading, rather than EOF -- it's insurance to save
            // datasets if some extra vis data are written out when a
            // data-taker crashes. "vislen" should always be set to land on
            // the end of a UV record.

            if self.stream.offset() >= self.eff_vislen {
                break;
            }

            self.stream.align_to(8)?;
        }

        Ok(())
    }
}

/// An iterator over the variables defined in a UV data stream
#[derive(Debug)]
pub struct UvVariablesIterator<'a>(slice::Iter<'a, UvVariable>);

impl<'a> Iterator for UvVariablesIterator<'a> {
    type Item = &'a UvVariable;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Decode a MIRIAD baseline value.
///
/// Denote the (validated) return value (ant1, ant2). Ant1 is always supposed
/// to be less than or equal to ant2. The maximum allowed value of each is
/// 2047. In Rubbl's convention, antenna numbers begin at 0; this is different
/// than MIRIAD!
///
/// Because of the antnum limitation we could return u16s, but ant numbers are
/// often used as array indices, so it's more convenient to keep them as
/// usizes.
pub fn decode_baseline(bl_float: f32) -> Result<(usize, usize), MiriadFormatError> {
    let bl = bl_float as isize;

    let (ant1, ant2) = if bl > 65536 {
        let ofs = bl - 65536;
        let ant1 = ofs / 2048;
        (ant1, (ofs - 2048 * ant1))
    } else {
        let ant1 = bl / 256;
        (ant1, (bl - 256 * ant1))
    };

    if ant1 < 1 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value {bl_float:?}: resulting ant1 is < 1"
        )));
    }

    if ant2 < 1 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value {bl_float:?}: resulting ant2 is < 1"
        )));
    }

    if ant1 > 2048 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value {bl_float:?}: resulting ant1 is > 2048"
        )));
    }

    if ant2 > 2048 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value {bl_float:?}: resulting ant2 is > 2048"
        )));
    }

    if ant1 > ant2 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value {bl_float:?}: resulting ant1 is > ant2"
        )));
    }

    Ok((ant1 as usize - 1, ant2 as usize - 1))
}

/// Encode a MIRIAD baseline value.
///
/// Antenna numbers may be between 0 and 2047, and ant1 must be less than or
/// equal to ant2. In Rubbl's convention, antenna numbers begin at 0; this is
/// different than MIRIAD!
pub fn encode_baseline(ant1: usize, ant2: usize) -> Result<f32, MiriadFormatError> {
    if ant1 > 2047 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value: antenna1 is {ant1} but limit is 2047"
        )));
    }

    if ant2 > 2047 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline value: antenna2 is {ant2} but limit is 2047"
        )));
    }

    if ant1 > ant2 {
        return Err(MiriadFormatError::Generic(format!(
            "illegal baseline pair ({ant1}, {ant2}); ant1 may not exceed ant2"
        )));
    }

    Ok(if ant2 > 254 {
        2048 * (ant1 + 1) + (ant2 + 1) + 65536
    } else {
        256 * (ant1 + 1) + (ant2 + 1)
    } as f32)
}

/// A struct that adapts the MIRIAD uv format into our VisStream interface.
#[derive(Debug)]
pub struct Reader {
    obstype: ObsType,
    ncorr: u64,
    nwcorr: u64,
    decoder: Decoder,
    flags: Option<MaskDecoder<AligningReader<io::BufReader<File>>>>,
    wflags: Option<MaskDecoder<AligningReader<io::BufReader<File>>>>,
}

impl Reader {
    pub fn create(ds: &mut DataSet) -> Result<Self, MiriadFormatError> {
        let ot_str: String = ds.get("obstype").require_found()?.read_scalar()?;

        let obstype = if ot_str.starts_with("auto") {
            ObsType::Auto
        } else if ot_str.starts_with("cross") {
            ObsType::Cross
        } else if ot_str.starts_with("mixed") {
            ObsType::MixedAutoCross
        } else {
            return Err(MiriadFormatError::Generic(format!(
                "unexpected \"obstype\" value {ot_str}"
            )));
        };

        let ncorr = ds.get("ncorr").require_found()?.read_scalar::<i64>()?;
        let nwcorr = ds.get("nwcorr").require_found()?.read_scalar::<i64>()?;

        let decoder = Decoder::create(ds)?;

        let flags = match ds.get("flags")? {
            Some(iii) => Some(MaskDecoder::new(iii.into_byte_stream()?)),
            None => None,
        };

        let wflags = match ds.get("wflags")? {
            Some(iii) => Some(MaskDecoder::new(iii.into_byte_stream()?)),
            None => None,
        };

        Ok(Reader {
            obstype,
            ncorr: ncorr as u64,
            nwcorr: nwcorr as u64,
            decoder,
            flags,
            wflags,
        })
    }
}

/// A struct that holds state for writing a variable stream in the MIRIAD UV
/// data format.
#[derive(Debug)]
pub struct Encoder {
    eff_vislen: u64,
    vars: Vec<UvVariable>,
    vars_by_name: HashMap<String, u8>,
    stream: AligningWriter<io::BufWriter<File>>,
    tot_nschan: i64,
    tot_nwchan: i64,
    ncorr: i64,
    nwcorr: i64,
    flushed: bool,
}

impl Encoder {
    /// Create a new Encoder that has the same variables as some input Decoder
    /// struct.
    pub fn new_like(ds: &mut DataSet, template: &Decoder) -> Result<Self, MiriadFormatError> {
        let mut vars = template.vars.clone();
        let vars_by_name = template.vars_by_name.clone();
        let mut vartable = ds.create_large_item("vartable", Type::Text)?;

        for var in &mut vars {
            var.n_vals = -1;
            var.data.clear();
            writeln!(vartable, "{} {}", var.ty.abbrev_char(), var.name)?;
        }

        let stream = ds.create_large_item("visdata", Type::Binary)?;

        Ok(Encoder {
            eff_vislen: 0,
            vars,
            vars_by_name,
            stream,
            tot_nschan: 0,
            tot_nwchan: 0,
            ncorr: 0,
            nwcorr: 0,
            flushed: false,
        })
    }

    pub fn write_var(&mut self, var: &UvVariable) -> Result<(), MiriadFormatError> {
        let our_num = self
            .vars_by_name
            .get(&var.name)
            .ok_or(MiriadFormatError::Generic(format!(
                "target stream does not have variable named \"{}\"",
                var.name
            )))?;
        let our_var = &mut self.vars[*our_num as usize];

        let mut header_buf = [0u8; 4];
        const SIZE: u8 = 0;
        const DATA: u8 = 1;

        if var.data.len() == 0 {
            return Err(MiriadFormatError::Generic(format!(
                "may not write zero-size array for variable \"{}\"",
                var.name
            )));
        }

        if var.name == "nschan" {
            self.tot_nschan = 0;

            for chunk in var.data.chunks(4) {
                self.tot_nschan = BigEndian::read_i32(chunk) as i64;
            }
        } // XXX: ignoring wide channels

        self.stream.align_to(8)?;
        self.flushed = false;

        if var.n_vals != our_var.n_vals {
            our_var.n_vals = var.n_vals;
            header_buf[0] = our_var.number;
            header_buf[2] = SIZE;
            self.stream.write_all(&header_buf)?;
            self.stream
                .write_i32::<BigEndian>((var.ty.size() * var.n_vals as usize) as i32)?;
        }

        header_buf[0] = our_var.number;
        header_buf[2] = DATA;
        self.stream.write_all(&header_buf)?;
        self.stream.align_to(our_var.ty.alignment() as usize)?;
        self.stream.write_all(&var.data)?;
        Ok(())
    }

    pub fn write<T: MiriadMappedType>(
        &mut self,
        name: &str,
        values: &[T],
    ) -> Result<(), MiriadFormatError> {
        let num = self
            .vars_by_name
            .get(name)
            .ok_or(MiriadFormatError::Generic(format!(
                "target stream does not have variable named \"{}\"",
                name
            )))?;
        let var = &mut self.vars[*num as usize];

        if values.len() == 0 {
            return Err(MiriadFormatError::Generic(format!(
                "may not write zero-size array for variable \"{}\"",
                name
            )));
        }

        // TODO: upcasting
        if T::TYPE != var.ty {
            return Err(MiriadFormatError::Generic(
                "attempting to encode UV variable of incompatible type".to_string(),
            ));
        }

        let mut header_buf = [0u8; 4];
        const SIZE: u8 = 0;
        const DATA: u8 = 1;

        self.stream.align_to(8)?;
        self.flushed = false;

        let n_vals = T::get_miriad_count(values) as isize;

        if var.n_vals != n_vals {
            var.n_vals = n_vals;
            let n_bytes = var.ty.size() * var.n_vals as usize;
            var.data.resize(n_bytes, 0);

            header_buf[0] = var.number;
            header_buf[2] = SIZE;
            self.stream.write_all(&header_buf)?;
            self.stream.write_i32::<BigEndian>(n_bytes as i32)?;
        }

        T::encode_values_into_vec(values, &mut var.data);

        if var.name == "nschan" {
            self.tot_nschan = 0;

            for chunk in var.data.chunks(4) {
                self.tot_nschan = BigEndian::read_i32(chunk) as i64;
            }
        } // XXX: ignoring wide channels

        header_buf[0] = var.number;
        header_buf[2] = DATA;
        self.stream.write_all(&header_buf)?;
        self.stream.align_to(var.ty.alignment() as usize)?;
        self.stream.write_all(&var.data)?;
        Ok(())
    }

    pub fn write_scalar<T: MiriadMappedType>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), MiriadFormatError> {
        self.write(name, &[value])
    }

    pub fn finish_record(&mut self) -> Result<(), std::io::Error> {
        self.ncorr += self.tot_nschan;
        self.nwcorr += self.tot_nwchan;

        const EOR: &[u8] = &[0u8, 0u8, 2u8, 0u8];
        self.stream.align_to(8)?;
        self.flushed = false;
        Ok(self.stream.write_all(EOR)?)
    }

    /// Returns the number of visdata bytes written thus far.
    pub fn flush(&mut self, ds: &mut DataSet) -> Result<u64, MiriadFormatError> {
        ds.set_scalar_item("ncorr", self.ncorr)?;
        ds.set_scalar_item("nwcorr", self.nwcorr)?;
        ds.set_scalar_item("vislen", (self.stream.offset() + 4) as i64)?;
        self.flushed = true;
        Ok(self.stream.offset())
    }
}

// Note: I wanted to add a Drop impl that panicked if the UV info had not been
// properly flushed before going away, but you are really not supposed to do
// anything that panics inside drop() because that can lead to aborts if
// something is dropped during unwinding. So we just silently let things go
// wrong. Cf. https://github.com/rust-lang/rust/issues/32677 .
