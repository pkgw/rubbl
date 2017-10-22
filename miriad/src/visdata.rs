// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD "uv" data sets containing visibility data.

 */

//use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

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
    // data
}

impl UvVariable {
    pub fn new(ty: Type, name: &str, number: u8) -> Self {
        UvVariable {
            name: name.to_owned(),
            number: number,
            ty: ty,
            n_vals: -1,
        }
    }
}


/// A struct that holds state for reading visibility data out of a MIRIAD uv
/// dataset.
pub struct Reader {
    obstype: ObsType,
    vislen: i64,
    ncorr: i64,
    nwcorr: i64,
    vars: Vec<UvVariable>,
    stream: BufReader<File>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
enum EntryType {
    Size = 0,
    Data = 1,
    EndOfRecord = 2,
    EndOfStream = 3, // not actually used on-disk
    Error = 255,
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

        let vislen = ds.get("vislen")?.read_scalar()?;
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

        Ok(Reader {
            obstype: obstype,
            vislen: vislen,
            ncorr: ncorr,
            nwcorr: nwcorr,
            vars: vars,
            stream: stream,
        })
    }
}
