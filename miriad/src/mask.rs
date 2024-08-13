// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Reading MIRIAD mask-format files, such as UV data flags.

 */

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
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
            stream,
            current_val: 0,
            bits_left_in_current: 0,
        }
    }

    pub fn expand(&mut self, dest: &mut [bool]) -> Result<(), io::Error> {
        let mut ofs = 0;
        let mut cur = self.current_val;
        let mut n_bits = dest.len();

        while n_bits > 0 {
            if self.bits_left_in_current > 0 {
                let mut toread = ::std::cmp::min(self.bits_left_in_current, n_bits);
                let mut i = 31 - self.bits_left_in_current;

                n_bits -= toread;
                self.bits_left_in_current -= toread;

                while toread > 0 {
                    dest[ofs] = cur & (1 << i) != 0;

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

        Ok(())
    }
}

#[derive(Debug)]
pub struct MaskEncoder<W: io::Write> {
    stream: W,
    current_val: u32,
    bits_left_in_current: usize,
    closed: bool,
}

impl<W: io::Write> MaskEncoder<W> {
    pub fn new(stream: W) -> Self {
        MaskEncoder {
            stream,
            current_val: 0,
            bits_left_in_current: 0,
            closed: false,
        }
    }

    pub fn append_mask(&mut self, data: &[bool]) -> Result<(), io::Error> {
        if self.closed {
            panic!("cannot append to mask after closing it");
        }

        let mut ofs = 0;
        let mut n_bits = data.len();
        let mut cur = self.current_val;
        let mut bits_left = self.bits_left_in_current;

        while n_bits > 0 {
            // There should always be at least one bit left up here.

            let mut towrite = ::std::cmp::min(bits_left, n_bits);
            let mut i = 31 - bits_left;

            n_bits -= towrite;
            bits_left -= towrite;

            while towrite > 0 {
                if data[ofs] {
                    cur |= 1 << i;
                }

                ofs += 1;
                i += 1;
                towrite -= 1;
            }

            // We stopped either because this u32 is full, or because there
            // are no more data. (Well, possibly both at once.)

            if bits_left == 0 {
                self.stream.write_u32::<BigEndian>(cur)?;
                cur = 0;
                bits_left = 31;
            }
        }

        self.current_val = cur;
        self.bits_left_in_current = bits_left;
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), io::Error> {
        if self.closed {
            return Ok(());
        }

        if self.bits_left_in_current != 31 {
            self.stream.write_u32::<BigEndian>(self.current_val)?;
        }

        self.stream.flush()?;
        self.closed = true;
        Ok(())
    }
}

impl<W: io::Write> Drop for MaskEncoder<W> {
    fn drop(&mut self) {
        if !self.closed {
            let _r = self.close();
        }
    }
}
