// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Access to MIRIAD "uv" data sets containing visibility data.

 */

use errors::Result;
use super::DataSet;


#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum EntryType {
    Size = 0,
    Data = 1,
    EndOfRecord = 2,
    EndOfStream = 3, // not actually used on-disk
    Error = 255,
}


pub struct VisDataItem {
}

impl VisDataItem {
    fn open(ds: &mut DataSet) -> Result<Self> {
        Ok(VisDataItem {})
    }
}
