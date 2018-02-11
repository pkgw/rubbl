// Copyright 2018 Peter Williams
// Licensed under the MIT License.

/*!

Access to FITS-format files.

With particular emphasis on UVFITS files, but that comes with more generic
support for FITS files.

 */

#![deny(missing_docs)]

extern crate byteorder;
extern crate failure;
#[macro_use] extern crate failure_derive;
extern crate rubbl_core;
//extern crate rubbl_visdata;

use failure::Error;
use rubbl_core::io::EofReadExactExt;
use std::io::prelude::*;


// Define this before any submodules are parsed.
macro_rules! fitserr {
    ($( $fmt_args:expr ),*) => {
        Err($crate::FitsFormatError(format!($( $fmt_args ),*)).into())
    }
}

/// An error type for when a FITS file is malformed.
#[derive(Debug, Fail)]
#[fail(display = "{}", _0)]
pub struct FitsFormatError(String);


/// An chunk of FITS file data, as produced by our low-level decoder.
#[derive(Clone, Debug)]
pub enum LowLevelFitsItem<'a> {
    /// A single header entry. The value of this variant is an 80-byte
    /// record representing a single FITS header keyword entry.
    Header(&'a [u8]),

    /// An item representing the end of the headers in the current HDU. Data
    /// bytes may follow, but not necessarily. The value of this variant is
    /// the number of data bytes that will follow.
    EndOfHeaders(usize),

    /// A chunk of data. The value of this variant is a chunk of unprocessed
    /// data bytes, no more than 2880 bytes long.
    Data(&'a [u8]),

    /// A chunk of "special record" data that follows the HDUs. Modern FITS
    /// files do not include these. Once special records are encountered, no
    /// more HDUs will be detected. The value of this variant is a chunk of
    /// this special record data that is exactly 2880 bytes in size.
    SpecialRecordData(&'a [u8]),
}


/// Possible values for the FITS "BITPIX" header, which identifies the storage
/// format of FITS binary data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i8)]
pub enum Bitpix {
    /// The data are stored as bytes or perhaps ASCII text.
    U8 = 8,

    /// The data map to the Rust type i16.
    I16 = 16,

    /// The data map to the Rust type i32.
    I32 = 32,

    /// The data map to the Rust type i64.
    I64 = 64,

    /// The data map to the Rust type f32.
    F32 = -32,

    /// The data map to the Rust type f64.
    F64 = -64,
}

impl Bitpix {
    /// Get the size of a single item in this BITPIX setting, in bytes.
    pub fn n_bytes(&self) -> usize {
        match *self {
            Bitpix::U8 => 1,
            Bitpix::I16 => 2,
            Bitpix::I32 => 4,
            Bitpix::I64 => 8,
            Bitpix::F32 => 4,
            Bitpix::F64 => 8,
        }
    }
}


/// FITS decoder.
///
/// This struct decodes its input stream assuming it is in FITS format. The
/// decoding is extremely low-level; only enough work is done to separate the
/// stream into headers and data correctly.
#[derive(Clone)] // can't Debug due to the [u8; 2880] item
pub struct FitsDecoder<R: Read> {
    inner: R,
    buf: [u8; 2880],
    offset: usize,

    state: DecoderState,
    hdu_num: usize,
    bitpix: Bitpix,
    naxis: Vec<usize>,
    primary_seen_groups: bool,
    pcount: isize,
    gcount: usize,
    data_remaining: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecoderState {
    Beginning,
    SizingHeaders,
    OtherHeaders,
    Data,
    NewHdu,
    SpecialRecords,
}


/// Note that we can't implement our I/O paradigm using Iterator because
/// Rust's iterators aren't "streaming": basically, the Rust paradigm is that
/// you can look at multiple iterator items at once, which isn't compatible
/// with our desire to be zero-copy and return byte slices rather than
/// allocate vecs. See [this
/// discussion](https://users.rust-lang.org/t/returning-borrowed-values-from-an-iterator/1096).
impl<R: Read> FitsDecoder<R> {
    /// Create a new decoder that gets data from the Read type passed as an argument.
    pub fn new(inner: R) -> Self {
        Self {
            inner: inner,
            buf: [0; 2880],
            offset: 2880,
            state: DecoderState::Beginning,
            hdu_num: 0,
            bitpix: Bitpix::U8,
            naxis: Vec::new(),
            primary_seen_groups: false,
            pcount: 0,
            gcount: 1,
            data_remaining: 0,
        }
    }

    /// Get the next item in the FITS stream.
    ///
    /// Returns Ok(None) at an expected EOF.
    pub fn next<'a>(&'a mut self) -> Result<Option<LowLevelFitsItem<'a>>, Error> {
        if self.offset == 2880 {
            if !self.inner.eof_read_exact::<Error>(&mut self.buf)? {
                if self.state != DecoderState::NewHdu && self.state != DecoderState::SpecialRecords {
                    return fitserr!("truncated-looking FITS file");
                }

                return Ok(None);
            }

            self.offset = 0;
        }

        if self.state == DecoderState::Data {
            if self.data_remaining > 2880 {
                self.offset = 2880;
                self.data_remaining -= 2880;
                return Ok(Some(LowLevelFitsItem::Data(&self.buf)));
            }

            let slice = &self.buf[..self.data_remaining];
            self.state = DecoderState::NewHdu;
            self.offset = 2880;
            self.bitpix = Bitpix::U8;
            self.gcount = 1;
            self.pcount = 0;
            self.naxis.clear();
            self.data_remaining = 0;
            self.primary_seen_groups = true; // all extension HDUs use the random-groups convention
            return Ok(Some(LowLevelFitsItem::Data(slice)));
        }

        if self.state == DecoderState::SpecialRecords {
            self.offset = 2880;
            return Ok(Some(LowLevelFitsItem::SpecialRecordData(&self.buf)));
        }

        let record = &self.buf[self.offset .. self.offset + 80];
        self.offset += 80;

        if self.state == DecoderState::Beginning {
            const FITS_MARKER: &[u8] = b"SIMPLE  =                    T";

            if &record[..FITS_MARKER.len()] != FITS_MARKER {
                return fitserr!("FITS data stream does not begin with \"SIMPLE = T\" marker");
            }

            self.state = DecoderState::SizingHeaders;
            return Ok(Some(LowLevelFitsItem::Header(record)));
        }

        if self.state == DecoderState::NewHdu {
            const XTENSION_MARKER: &[u8] = b"XTENSION= ";

            if &record[..XTENSION_MARKER.len()] != XTENSION_MARKER {
                // Almost no FITS files use them anymore, but we have to assume that
                // this file has "special records".
                self.state = DecoderState::SpecialRecords;
                return Ok(Some(LowLevelFitsItem::SpecialRecordData(&self.buf)));
            }

            self.state = DecoderState::SizingHeaders;
            self.hdu_num += 1;
            return Ok(Some(LowLevelFitsItem::Header(record)));
        }

        if self.state == DecoderState::SizingHeaders {
            // The standard requires that these headers appear in a prescribed
            // order, but we don't bother to enforce that.
            const BITPIX_MARKER: &[u8] = b"BITPIX  = ";
            const NAXIS_MARKER: &[u8] =  b"NAXIS   = ";
            let mut keep_going = false;

            if &record[..BITPIX_MARKER.len()] == BITPIX_MARKER {
                let bitpix = parse_fixed_int(record)?;

                self.bitpix = match bitpix {
                    8 => Bitpix::U8,
                    16 => Bitpix::I16,
                    32 => Bitpix::I32,
                    64 => Bitpix::I64,
                    -32 => Bitpix::F32,
                    -64 => Bitpix::F64,
                    other => {
                        return fitserr!("unsupported BITPIX value in FITS file: {}", other);
                    },
                };
            } else if &record[..NAXIS_MARKER.len()] == NAXIS_MARKER {
                let naxis = parse_fixed_int(record)?;

                if naxis < 0 || naxis > 999 {
                    return fitserr!("unsupported NAXIS value in FITS file: {}", naxis);
                }

                self.naxis.clear();
                self.naxis.reserve(naxis as usize);
            } else if &record[..5] == b"NAXIS" {
                if &record[8..10] != b"= " {
                    return fitserr!("malformed FITS NAXIS header");
                }

                // Laboriously figure out the axis number.

                let mut value = 0;
                let mut i = 5;

                while i < 8 {
                    if record[i] == b' ' {
                        break;
                    }

                    value *= 10;

                    match record[i] {
                        b'0' => {},
                        b'1' => { value += 1; },
                        b'2' => { value += 2; },
                        b'3' => { value += 3; },
                        b'4' => { value += 4; },
                        b'5' => { value += 5; },
                        b'6' => { value += 6; },
                        b'7' => { value += 7; },
                        b'8' => { value += 8; },
                        b'9' => { value += 9; },
                        other => {
                            return fitserr!("expected digit but got ASCII {:?} in NAXIS header", other);
                        },
                    }

                    i += 1;
                }

                while i < 8 {
                    if record[i] != b' ' {
                        return fitserr!("expected space but got ASCII {:?} in NAXIS header", record[i]);
                    }

                    i += 1;
                }

                if value != self.naxis.len() + 1 {
                    return fitserr!("misnumbered NAXIS header (expected {}, got {})",
                                    self.naxis.len() + 1, value);
                }

                let n = parse_fixed_int(record)?;

                if n < 0 {
                    return fitserr!("illegal negative NAXIS{} value {}", value, n);
                }

                self.naxis.push(n as usize);
            } else {
                keep_going = true;
                self.state = DecoderState::OtherHeaders;
            }

            if !keep_going {
                return Ok(Some(LowLevelFitsItem::Header(record)));
            }
        }

        if self.state == DecoderState::OtherHeaders {
            const END_MARKER: &[u8] = b"END                                                                             ";
            const GROUPS_MARKER: &[u8] = b"GROUPS  =                    T";
            const PCOUNT_MARKER: &[u8] = b"PCOUNT  = ";
            const GCOUNT_MARKER: &[u8] = b"GCOUNT  = ";

            if &record[..GROUPS_MARKER.len()] == GROUPS_MARKER {
                self.primary_seen_groups = true;
            } else if self.primary_seen_groups && &record[..PCOUNT_MARKER.len()] == PCOUNT_MARKER {
                self.pcount = parse_fixed_int(record)?;
            } else if self.primary_seen_groups && &record[..GCOUNT_MARKER.len()] == GCOUNT_MARKER {
                let n = parse_fixed_int(record)?;

                if n < 0 {
                    return fitserr!("illegal negative FITS GCOUNT value");
                }

                self.gcount = n as usize;
            } else if record == END_MARKER {
                let group_size = if self.hdu_num == 0 {
                    self.pcount + self.naxis.iter().skip(1).fold(1, |p, n| p * n) as isize
                } else {
                    self.pcount + self.naxis.iter().fold(1, |p, n| p * n) as isize
                };

                if group_size < 0 {
                    return fitserr!("illegal negative FITS group size");
                }

                self.offset = 2880;
                self.data_remaining = self.bitpix.n_bytes() * self.gcount * group_size as usize;

                if self.data_remaining != 0 {
                    self.state = DecoderState::Data;
                } else {
                    self.state = DecoderState::NewHdu;
                    self.offset = 2880;
                    self.bitpix = Bitpix::U8;
                    self.gcount = 1;
                    self.pcount = 0;
                    self.naxis.clear();
                    self.primary_seen_groups = true; // all extension HDUs use the random-groups convention
                }

                return Ok(Some(LowLevelFitsItem::EndOfHeaders(self.data_remaining)));
            }

            let mut i = 0;

            while i < 8 {
                match record[i] {
                    0x30...0x39 => {}, // 0-9
                    0x41...0x5A => {}, // A-Z
                    b'_' => {},
                    b'-' => {},
                    b' ' => { break; }
                    other => {
                        return fitserr!("illegal header keyword ASCII code {}", other);
                    }
                }

                i += 1;
            }

            while i < 8 {
                if record[i] != b' ' {
                    return fitserr!("malformed FITS header keyword");
                }

                i += 1;
            }

            return Ok(Some(LowLevelFitsItem::Header(record)));
        }

        Ok(None)
    }

    /// Consume this decoder and return the inner Read object.
    pub fn into_inner(self) -> R {
        self.inner
    }
}


fn parse_fixed_int(record: &[u8]) -> Result<isize, Error> {
    if record[30] != b' ' && record[30] != b'/' {
        return fitserr!("expected space or slash in byte 30 of fixed-format integer record");
    }

    let mut i = 10;

    while i < 30 {
        if record[i] != b' ' {
            break;
        }

        i += 1;
    }

    if i == 30 {
        return fitserr!("empty record that should have been a fixed-format integer");
    }

    let mut negate = false;

    if record[i] == b'-' {
        negate = true;
        i += 1;
    } else if record[i] == b'+' {
        i += 1;
    }

    if i == 30 {
        return fitserr!("empty record that should have been a fixed-format integer");
    }

    let mut value = 0;

    while i < 30 {
        value *= 10;

        match record[i] {
            b'0' => {},
            b'1' => { value += 1; },
            b'2' => { value += 2; },
            b'3' => { value += 3; },
            b'4' => { value += 4; },
            b'5' => { value += 5; },
            b'6' => { value += 6; },
            b'7' => { value += 7; },
            b'8' => { value += 8; },
            b'9' => { value += 9; },
            other => {
                return fitserr!("expected digit but got ASCII {:?} in fixed-format integer", other);
            },
        }

        i += 1;
    }

    if negate {
        value *= -1;
    }

    Ok(value)
}

#[cfg(test)]
#[test]
fn fixed_int_parsing() {
    // 0         1         2         3         4         5         6         7
    // 01234567890123456789012345678901234567890123456789012345678901234567890123456789
    //"NAXIS   =                  999 / comment                                        "

    let r = b"NAXIS   =                  999 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), 999);
    let r = b"NAXIS   =           2147483647 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), 2147483647);
    let r = b"NAXIS   =          -2147483647 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), -2147483647);
    let r = b"NAXIS   =                  999/ comment                                         ";
    assert_eq!(parse_fixed_int(r).unwrap(), 999);
    let r = b"NAXIS   =                  999                  / comment                       ";
    assert_eq!(parse_fixed_int(r).unwrap(), 999);
    let r = b"NAXIS   =                 +999 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), 999);
    let r = b"NAXIS   =                 -999 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), -999);
    let r = b"NAXIS   = -0000000000000000999 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), -999);
    let r = b"NAXIS   = A                  9 / comment                                        ";
    assert!(parse_fixed_int(r).is_err());
    let r = b"NAXIS   =                    9A / comment                                       ";
    assert!(parse_fixed_int(r).is_err());
}
