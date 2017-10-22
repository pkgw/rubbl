// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD "uv" data sets containing visibility data.

 */

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
//use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use errors::Result;
use super::{DataSet, Type};


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


/// A struct that holds state for reading visibility data out of a MIRIAD uv
/// dataset.
pub struct Reader {
    obstype: ObsType,
    eff_vislen: usize,
    ncorr: i64,
    nwcorr: i64,
    vars: Vec<UvVariable>,
    stream: io::BufReader<File>,
    offset: usize,
}


impl Reader {
    pub fn create(ds: &mut DataSet) -> Result<Self> {
        let ot_str: String = ds.get("obstype")?.read_scalar()?;

        let obstype = if ot_str.starts_with("auto") {
            ObsType::Auto
        } else if ot_str.starts_with("cross") {
            ObsType::Cross
        } else if ot_str.starts_with("mixed") {
            ObsType::MixedAutoCross
        } else {
            return err_msg!("unexpected \"obstype\" value {}", ot_str);
        };

        let vislen = ds.get("vislen")?.read_scalar::<i64>()?;
        let ncorr = ds.get("ncorr")?.read_scalar()?;
        let nwcorr = ds.get("nwcorr")?.read_scalar()?;

        let mut vars = Vec::new();
        let mut var_num = 0u8;

        for maybe_line in ds.get("vartable")?.into_lines()? {
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

            if var_num == 255 {
                return err_msg!("too many UV variables");
            }

            var_num += 1;
        }

        let stream = ds.get("visdata")?.into_byte_stream()?;

        println!("OFS: {}", stream.into_inner().seek(io::SeekFrom::Current(0))?);
        panic!("X");

        Ok(Reader {
            obstype: obstype,
            eff_vislen: vislen as usize - 4, // this is always too big
            ncorr: ncorr,
            nwcorr: nwcorr,
            vars: vars,
            stream: stream,
            offset: 4, // account for the mixed-binary tag.
        })
    }


    /// Returns Ok(false) on EOF, Ok(true) if there are more data.
    pub fn next(&mut self) -> Result<bool> {
        let mut keep_going = true;
        let mut header_buf = [0u8; 4];

        while keep_going {
            // The "vislen" variable is what we should use to determine when
            // to stop reading, rather than the EOF signal -- it's insurance
            // to save datasets if some extra vis data are written out when a
            // data-taker crashes.
            println!("{} {}", self.offset, self.eff_vislen);
            if self.offset == self.eff_vislen {
                return Ok(false);
            }

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

                    let remainder = self.offset % var.ty.alignment() as usize;
                    if remainder != 0 {
                        let misalignment = var.ty.alignment() as usize - remainder;
                        let mut align_buf = [0u8; 8];
                        self.stream.read_exact(&mut align_buf[..misalignment])?;
                        self.offset += misalignment;
                    }

                    self.stream.read_exact(&mut var.data)?;
                    self.offset += var.data.len();
                },
                EOR => {
                    keep_going = false;
                },
                z => {
                    return err_msg!("invalid visdata: unrecognized record code {}", z);
                }
            }

            let remainder = self.offset % 8;

            if remainder != 0 {
                let misalignment = 8 - remainder;
                let mut align_buf = [0u8; 8];
                self.stream.read_exact(&mut align_buf[..misalignment])?;
                self.offset += misalignment;
            }
        }

        Ok(true)
    }
}
