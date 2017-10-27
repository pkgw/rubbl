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

use byteorder::{BigEndian, ReadBytesExt};
use rubbl_core::errors::Result;
use rubbl_core::io::OpenResultExt;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use super::{DataSet, MiriadMappedType, Type};


#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum ObsType {
    Auto,
    Cross,
    MixedAutoCross,
}


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct UvVariable {
    name: String,
    number: u8,
    ty: Type,
    n_vals: isize,
    data: Vec<u8>,
}

impl UvVariable {
    pub fn new(ty: Type, name: &str, number: u8) -> Self {
        UvVariable {
            name: name.to_owned(),
            number: number,
            ty: ty,
            n_vals: -1,
            data: Vec::new(),
        }
    }
}


#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct UvVariableReference(u8);


/// A struct that holds state for reading visibility data out of a MIRIAD uv
/// dataset.
#[derive(Debug)]
pub struct Reader {
    obstype: ObsType,
    eff_vislen: u64,
    ncorr: u64,
    //nwcorr: i64,
    vars: Vec<UvVariable>,
    vars_by_name: HashMap<String, u8>,
    stream: io::BufReader<File>,
    offset: u64,
}


impl Reader {
    pub fn create(ds: &mut DataSet) -> Result<Self> {
        let ot_str: String = ds.get("obstype").require_found()?.read_scalar()?;

        let obstype = if ot_str.starts_with("auto") {
            ObsType::Auto
        } else if ot_str.starts_with("cross") {
            ObsType::Cross
        } else if ot_str.starts_with("mixed") {
            ObsType::MixedAutoCross
        } else {
            return err_msg!("unexpected \"obstype\" value {}", ot_str);
        };

        let vislen = ds.get("vislen").require_found()?.read_scalar::<i64>()?;
        let ncorr = ds.get("ncorr").require_found()?.read_scalar::<i64>()?;
        //let nwcorr = ds.get("nwcorr").require_found()?.read_scalar()?;

        let mut vars = Vec::new();
        let mut vars_by_name = HashMap::new();
        let mut var_num = 0u8;

        for maybe_line in ds.get("vartable").require_found()?.into_lines()? {
            let line = maybe_line?;

            if line.len() < 3 {
                return err_msg!("illegal vartable line: {}", line);
            }

            let pieces: Vec<_> = line.split_whitespace().collect();

            if pieces.len() != 2 {
                return err_msg!("illegal vartable line: {}", line);
            }

            let ty = Type::try_from_abbrev(pieces[0])?;
            let name = pieces[1];

            vars.push(UvVariable::new(ty, name, var_num));

            // TODO: check for duplicates
            vars_by_name.insert(name.to_owned(), var_num);

            if var_num == 255 {
                return err_msg!("too many UV variables");
            }

            var_num += 1;
        }

        let stream = ds.get("visdata").require_found()?.into_byte_stream()?;

        Ok(Reader {
            obstype: obstype,
            eff_vislen: vislen as u64 - 4, // this is always too big
            ncorr: ncorr as u64,
            //nwcorr: nwcorr,
            vars: vars,
            vars_by_name: vars_by_name,
            stream: stream,
            offset: 4, // account for the mixed-binary tag.
        })
    }


    pub fn obs_type(&self) -> ObsType {
        self.obstype
    }


    pub fn num_correlations(&self) -> u64 {
        self.ncorr as u64
    }


    /// Get the size of the bulk visibility data file in bytes.
    pub fn visdata_bytes(&self) -> u64 {
        self.eff_vislen
    }


    /// Returns Ok(false) on EOF, Ok(true) if there are more data.
    pub fn next(&mut self) -> Result<bool> {
        let mut keep_going = true;
        let mut header_buf = [0u8; 4];

        while keep_going {
            self.stream.read_exact(&mut header_buf)?;
            self.offset += 4;
            let varnum = header_buf[0];
            let entry_type = header_buf[2];

            const SIZE: u8 = 0;
            const DATA: u8 = 1;
            const EOR: u8 = 2;

            match entry_type {
                SIZE => {
                    if varnum as usize >= self.vars.len() {
                        return err_msg!("invalid visdata: too-large variable number");
                    }

                    let var = &mut self.vars[varnum as usize];
                    let n_bytes = self.stream.read_i32::<BigEndian>()?;
                    self.offset += 4;

                    if n_bytes < 0 {
                        return err_msg!("invalid visdata: negative data size");
                    }

                    if n_bytes % var.ty.size() as i32 != 0 {
                        return err_msg!("invalid visdata: non-integral number of elements in array");
                    }

                    var.n_vals = (n_bytes / (var.ty.size() as i32)) as isize;
                    var.data.resize(n_bytes as usize, 0); // bit of slowness: zeroing out the data
                },
                DATA => {
                    if varnum as usize >= self.vars.len() {
                        return err_msg!("invalid visdata: too-large variable number");
                    }

                    let var = &mut self.vars[varnum as usize];

                    let remainder = self.offset % var.ty.alignment() as u64;
                    if remainder != 0 {
                        let misalignment = (var.ty.alignment() as u64 - remainder) as usize;
                        let mut align_buf = [0u8; 8];
                        self.stream.read_exact(&mut align_buf[..misalignment])?;
                        self.offset += misalignment as u64;
                    }

                    self.stream.read_exact(&mut var.data)?;
                    self.offset += var.data.len() as u64;
                },
                EOR => {
                    keep_going = false;
                },
                z => {
                    return err_msg!("invalid visdata: unrecognized record code {}", z);
                }
            }

            // The "vislen" variable is what we should use to determine when
            // to stop reading, rather than EOF -- it's insurance to save
            // datasets if some extra vis data are written out when a
            // data-taker crashes. "vislen" should always be set to land on
            // the end of a UV record.

            if self.offset >= self.eff_vislen {
                return Ok(false);
            }

            let remainder = self.offset % 8;

            if remainder != 0 {
                let misalignment = 8 - remainder as usize;
                let mut align_buf = [0u8; 8];
                self.stream.read_exact(&mut align_buf[..misalignment])?;
                self.offset += misalignment as u64;
            }
        }

        Ok(true)
    }


    pub fn lookup_variable(&self, var_name: &str) -> Option<UvVariableReference> {
        self.vars_by_name.get(var_name).map(|o| UvVariableReference(*o))
    }


    pub fn get<T: MiriadMappedType>(&self, var: UvVariableReference, buf: &mut Vec<T>) {
        let var = &self.vars[var.0 as usize];

        // TODO: upcasting
        if T::TYPE != var.ty {
            panic!("attempting to decode UV variable of incompatible type");
        }

        T::decode_buf_into_vec(&var.data, buf);
    }
}
