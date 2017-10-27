// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Reading MIRIAD mask-format files, such as UV data flags.

 */

use byteorder::{BigEndian, ReadBytesExt};
use rubbl_core::errors::Result;
use std::io;


#[derive(Debug)]
pub struct MaskDecoder<R: io::Read> {
    stream: R,
    current_val: u32,
    bits_left_in_current: usize,
}


impl<R: io::Read> MaskDecoder<R> {
    pub fn new(stream: R) -> Self {
        MaskDecoder {
            stream: stream,
            current_val: 0,
            bits_left_in_current: 0
        }
    }


    pub fn expand(&mut self, dest: &mut [bool]) -> Result<()> {
        let mut ofs = 0;
        let mut cur = self.current_val;
        let mut n_bits = dest.len();

        while n_bits > 0 {
            if self.bits_left_in_current > 0 {
                let mut toread = ::std::cmp::min(self.bits_left_in_current, n_bits);
                n_bits -= toread;
                self.bits_left_in_current -= toread;

                let mut i = 31 - self.bits_left_in_current;

                while toread > 0 {
                    dest[ofs] = if cur & (1 << i) != 0 {
                        true
                    } else {
                        false
                    };

                    ofs += 1;
                    i += 1;
                    toread -= 1;
                }
            }

            if n_bits == 0 {
                return Ok(());
            }

            cur = self.stream.read_u32::<BigEndian>()?;
            self.current_val = cur;
            self.bits_left_in_current = 31;
        }

        return Ok(());
    }
}
