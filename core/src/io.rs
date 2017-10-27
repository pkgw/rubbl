// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Basic I/O helpers.

 */

//use byteorder::{BigEndian, ReadBytesExt};
use std::io;

use errors::{ErrorKind, Result};


pub trait OpenResultExt {
    type Reprocessed;

    fn require_found(self) -> Self::Reprocessed;
}


impl<T> OpenResultExt for Result<Option<T>> {
    type Reprocessed = Result<T>;

    fn require_found(self) -> Self::Reprocessed {
        match self {
            Err(e) => Err(e),
            Ok(o) => {
                if let Some(x) = o {
                    Ok(x)
                } else {
                    Err(ErrorKind::Io(io::Error::new(io::ErrorKind::NotFound, "not found")).into())
                }
            }
        }
    }
}
