// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Basic I/O helpers.

 */

use byteorder::{BigEndian, ByteOrder};
use num_complex::Complex;
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


pub trait EofReadExactExt: io::Read {
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


impl<R: io::Read> EofReadExactExt for R {
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
