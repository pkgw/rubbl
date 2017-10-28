// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Basic I/O helpers.

 */

use byteorder::{BigEndian, ByteOrder};
use num_complex::Complex;
use std::io;
use std::io::Read;

use errors::{ErrorKind, Result};


#[derive(Debug)]
pub struct AligningReader<R: Read> {
    inner: R,
    offset: u64
}


impl<R: Read> AligningReader<R> {
    pub fn new(inner: R) -> Self {
        AligningReader {
            inner: inner,
            offset: 0,
        }
    }


    pub fn into_inner(self) -> R {
        self.inner
    }


    pub fn offset(&self) -> u64 {
        self.offset
    }


    pub fn align_to(&mut self, alignment: usize) -> Result<bool> {
        let mut buf = [0u8; 64];

        if alignment > 64 {
            panic!("maximum alignment size is 64");
        }

        let excess = (self.offset % alignment as u64) as usize;

        if excess == 0 {
            Ok(true)
        } else {
            let amount = alignment - excess;
            let result = self.inner.eof_read_exact(&mut buf[..amount]);

            if result.is_ok() {
                self.offset += amount as u64;
            }

            result
        }
    }
}

impl<R: Read> Read for AligningReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self.inner.read(buf);

        if let Ok(n) = result {
            self.offset += n as u64;
        }

        result
    }
}


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


pub trait EofReadExactExt: Read {
    /// Like `Read::read_exact`, except returns Ok(false) if EOF was
    /// encountered at the first read attempt. Returns Ok(true) if everything
    /// was OK and EOF has not yet been hit. Returns Err with an IoError with
    /// a "kind" of UnexpectedEof if EOF was encountered somewhere in the
    /// midst of the buffer.
    fn eof_read_exact(&mut self, buf: &mut [u8]) -> Result<bool>;

    /// Like `byteorder::ReadBytesExt::read_i64::<BigEndian>`, except returns
    /// Some(n) on success and None if EOF was encountered at the first read
    /// attempt.
    fn eof_read_be_i64(&mut self) -> Result<Option<i64>> {
        let mut buf = [0u8; 8];

        if self.eof_read_exact(&mut buf)? {
            Ok(Some(BigEndian::read_i64(&buf)))
        } else {
            Ok(None)
        }
    }

    fn eof_read_be_f32(&mut self) -> Result<Option<f32>> {
        let mut buf = [0u8; 4];

        if self.eof_read_exact(&mut buf)? {
            Ok(Some(BigEndian::read_f32(&buf)))
        } else {
            Ok(None)
        }
    }

    fn eof_read_be_c64(&mut self) -> Result<Option<Complex<f32>>> {
        let mut buf = [0u8; 8];

        if self.eof_read_exact(&mut buf)? {
            Ok(Some(Complex::new(
                BigEndian::read_f32(&buf[..4]),
                BigEndian::read_f32(&buf[4..])
            )))
        } else {
            Ok(None)
        }
    }
}


impl<R: Read> EofReadExactExt for R {
    fn eof_read_exact(&mut self, buf: &mut [u8]) -> Result<bool> {
        let mut n_left = buf.len();
        let mut ofs = 0;

        while n_left > 0 {
            let n_read = match self.read(&mut buf[ofs..]) {
                Ok(n) => n,
                Err(e) => {
                    if e.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }

                    return Err(e.into());
                }
            };

            if n_read == 0 {
                return if ofs == 0 {
                    Ok(false) // no more data at an expected stopping point
                } else {
                    Err(ErrorKind::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "unexpected EOF")).into())
                };
            }

            ofs += n_read;
            n_left -= n_read;
        }

        Ok(true) // more data, we think
    }
}
