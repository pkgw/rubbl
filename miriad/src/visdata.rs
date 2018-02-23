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

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use failure::Error;
use rubbl_core::io::{AligningReader, AligningWriter, OpenResultExt};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::slice;

use mask::MaskDecoder;
use super::{AnyMiriadValue, DataSet, MiriadMappedType, Type};


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
            number: number,
            ty: ty,
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
    pub fn create(ds: &mut DataSet) -> Result<Self, Error> {
        let vislen = ds.get("vislen").require_found()?.read_scalar::<i64>()?;
        let mut vars = Vec::new();
        let mut vars_by_name = HashMap::new();
        let mut var_num = 0u8;

        for maybe_line in ds.get("vartable").require_found()?.into_lines()? {
            let line = maybe_line?;

            if line.len() < 3 {
                return mirerr!("illegal vartable line: {}", line);
            }

            let pieces: Vec<_> = line.split_whitespace().collect();

            if pieces.len() != 2 {
                return mirerr!("illegal vartable line: {}", line);
            }

            let ty = Type::try_from_abbrev(pieces[0])?;
            let name = pieces[1];

            vars.push(UvVariable::new(ty, name, var_num));

            // TODO: check for duplicates
            vars_by_name.insert(name.to_owned(), var_num);

            if var_num == 255 {
                return mirerr!("too many UV variables");
            }

            var_num += 1;
        }

        let stream = ds.get("visdata").require_found()?.into_byte_stream()?;

        Ok(Decoder {
            eff_vislen: vislen as u64 - 4, // this is always too big
            vars: vars,
            vars_by_name: vars_by_name,
            stream: stream,
        })
    }


    /// Get the size of the bulk visibility data file in bytes.
    pub fn visdata_bytes(&self) -> u64 {
        self.eff_vislen
    }


    /// Returns Ok(false) on EOF, Ok(true) if there are more data.
    pub fn next(&mut self) -> Result<bool, Error> {
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
                        return mirerr!("invalid visdata: too-large variable number");
                    }

                    let var = &mut self.vars[varnum as usize];
                    let n_bytes = self.stream.read_i32::<BigEndian>()?;

                    if n_bytes < 0 {
                        return mirerr!("invalid visdata: negative data size");
                    }

                    if n_bytes % var.ty.size() as i32 != 0 {
                        return mirerr!("invalid visdata: non-integral number of elements in array");
                    }

                    var.n_vals = (n_bytes / (var.ty.size() as i32)) as isize;
                    var.data.resize(n_bytes as usize, 0); // bit of slowness: zeroing out the data
                },
                DATA => {
                    if varnum as usize >= self.vars.len() {
                        return mirerr!("invalid visdata: too-large variable number");
                    }

                    let var = &mut self.vars[varnum as usize];
                    self.stream.align_to(var.ty.alignment() as usize)?;
                    self.stream.read_exact(&mut var.data)?;
                    var.just_updated = true;
                },
                EOR => {
                    keep_going = false;
                },
                z => {
                    return mirerr!("invalid visdata: unrecognized record code {}", z);
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
        self.vars_by_name.get(var_name).map(|o| UvVariableReference(*o))
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
    pub fn create(ds: &mut DataSet) -> Result<Self, Error> {
        let ot_str: String = ds.get("obstype").require_found()?.read_scalar()?;

        let obstype = if ot_str.starts_with("auto") {
            ObsType::Auto
        } else if ot_str.starts_with("cross") {
            ObsType::Cross
        } else if ot_str.starts_with("mixed") {
            ObsType::MixedAutoCross
        } else {
            return mirerr!("unexpected \"obstype\" value {}", ot_str);
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
            obstype: obstype,
            ncorr: ncorr as u64,
            nwcorr: nwcorr as u64,
            decoder: decoder,
            flags: flags,
            wflags: wflags,
        })
    }
}
