// Copyright 2018-2020 Peter Williams
// Licensed under the MIT License.

//! Access to FITS-format files.
//!
//! With particular emphasis on UVFITS files, but that comes with more generic
//! support for FITS files.

#![deny(missing_docs)]

use failure::Error;
use failure_derive::Fail;
use rubbl_core::io::EofReadExactExt;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::str;

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

/// A chunk of FITS file data, as produced by our low-level decoder.
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

/// A decoder for single-pass streaming of a FITS file.
///
/// This struct decodes its input stream assuming it is in FITS format. The
/// decoding is extremely low-level; only enough work is done to separate the
/// stream into headers and data correctly. The underlying stream need only
/// implement Read, and the items that are streamed out are well-suited for
/// reproducing the input file byte-for-byte.
///
/// This class is not very picking about checking FITS conformity.
#[derive(Clone)] // can't be Debug due to the [u8; 2880] item
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

const FITS_MARKER: &[u8] = b"SIMPLE  =                    T";
const XTENSION_MARKER: &[u8] = b"XTENSION= ";
const BITPIX_MARKER: &[u8] = b"BITPIX  = ";
const NAXIS_MARKER: &[u8] = b"NAXIS   = ";
const END_MARKER: &[u8] =
    b"END                                                                             ";
const GROUPS_MARKER: &[u8] = b"GROUPS  =                    T";
const PCOUNT_MARKER: &[u8] = b"PCOUNT  = ";
const GCOUNT_MARKER: &[u8] = b"GCOUNT  = ";
const EXTNAME_MARKER: &[u8] = b"EXTNAME = ";

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
                if self.state != DecoderState::NewHdu && self.state != DecoderState::SpecialRecords
                {
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

        let record = &self.buf[self.offset..self.offset + 80];
        self.offset += 80;

        if self.state == DecoderState::Beginning {
            if &record[..FITS_MARKER.len()] != FITS_MARKER {
                return fitserr!("FITS data stream does not begin with \"SIMPLE = T\" marker");
            }

            self.state = DecoderState::SizingHeaders;
            return Ok(Some(LowLevelFitsItem::Header(record)));
        }

        if self.state == DecoderState::NewHdu {
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
                    }
                };
            } else if &record[..NAXIS_MARKER.len()] == NAXIS_MARKER {
                let naxis = parse_fixed_int(record)?;

                if naxis < 0 || naxis > 999 {
                    return fitserr!("unsupported NAXIS value in FITS file: {}", naxis);
                }

                self.naxis.clear();
                self.naxis.reserve(naxis as usize);
            } else if accumulate_naxis_value(record, &mut self.naxis)? {
                // OK, has been handled.
            } else {
                keep_going = true;
                self.state = DecoderState::OtherHeaders;
            }

            if !keep_going {
                return Ok(Some(LowLevelFitsItem::Header(record)));
            }
        }

        if self.state == DecoderState::OtherHeaders {
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
                let group_size = if self.hdu_num == 0 && self.primary_seen_groups {
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
                    0x30..=0x39 => {} // 0-9
                    0x41..=0x5A => {} // A-Z
                    b'_' => {}
                    b'-' => {}
                    b' ' => {
                        break;
                    }
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

/// Parse the headers of a FITS file to allow navigation of its structure.
///
/// This struct parses a seekable steam assuming it is in FITS format, building
/// up a view of its overall structure.
#[derive(Clone, Debug)]
pub struct FitsParser<R: Read + Seek> {
    inner: R,
    hdus: Vec<ParsedHdu>,
    special_record_size: u64,
}

/// Different kinds of HDUs known to this module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HduKind {
    /// This HDU is the primary HDU, and it contains an N-dimensional array.
    PrimaryArray,

    /// This HDU is the primary HDU, and it contains a "random groups" binary
    /// table. This format is superseded by the BINTABLE extension type.
    PrimaryRandomGroups,

    /// This HDU is the primary HDU, and the array it specifies has zero total
    /// size.
    PrimaryNoData,

    /// This HDU contains an N-dimensional data array.
    ImageExtension,

    /// This HDU contains a textual data table.
    AsciiTableExtension,

    /// This HDU contains a binary data table.
    BinaryTableExtension,

    /// This HDU contains data of unrecognized format. The data are still
    /// accessible as an arbitrary binary stream, because the FITS format
    /// includes enough information to deduce the overall data structure, but
    /// beyond that you're on your own.
    OtherExtension(String),
}

/// Information about an HDU in a parsed FITS file.
#[derive(Clone, Debug)]
pub struct ParsedHdu {
    kind: HduKind,
    name: String,
    header_offset: u64,
    n_header_records: usize,
    bitpix: Bitpix,
    pcount: isize,
    gcount: usize,
    naxis: Vec<usize>,
}

impl<R: Read + Seek> FitsParser<R> {
    /// Parse the headers of a FITS file.
    pub fn new(mut inner: R) -> Result<Self, Error> {
        let file_size = inner.seek(SeekFrom::End(0))?;

        if file_size % 2880 != 0 {
            return fitserr!(
                "FITS stream should be a multiple of 2880 bytes long; got {}",
                file_size
            );
        }

        inner.seek(SeekFrom::Start(0))?;

        let mut hdus = Vec::new();
        let mut buf = [0u8; 2880];
        let mut cur_offset = 0; // current offset into the file.
        let mut hdu_header_offset = 0; // file offset at which the current HDU's headers started
        let mut special_record_size = 0; // number of bytes in "special records" at the end of the file

        loop {
            // We are at the beginning of an HDU, and `buf` doesn't contain
            // any valid data. Scan the headers to figure out how big they are
            // and how big the data are. We know how big the overall file is
            // so if we're here we know that there are more data to look at.

            inner.read_exact(&mut buf)?;
            cur_offset += 2880;

            // The first sizing headers will always fit into the first
            // 2880-byte chunk. But if there are more than 31 dimensions to
            // the data array, we'll need to do some reads to get the NAXIS
            // headers.
            //
            // First: SIMPLE or XTENSION.

            let mut kind = HduKind::PrimaryArray;

            if hdus.len() == 0 {
                if &buf[..FITS_MARKER.len()] != FITS_MARKER {
                    return fitserr!("file does not appear to be in FITS format");
                }
            } else {
                if &buf[..XTENSION_MARKER.len()] != XTENSION_MARKER {
                    // We must conclude that this FITS file has "special
                    // records" at the end.
                    special_record_size = file_size - hdu_header_offset;
                    break;
                }

                kind = match parse_fixed_string(&buf[..80])?.as_ref() {
                    "IMAGE" => HduKind::ImageExtension,
                    "TABLE" => HduKind::AsciiTableExtension,
                    "BINTABLE" => HduKind::BinaryTableExtension,
                    other => HduKind::OtherExtension(other.to_owned()), // gross copying, but whatever
                };
            }

            // Next: BITPIX.

            let bitpix_value = {
                let record = &buf[80..160];

                if &record[..BITPIX_MARKER.len()] != BITPIX_MARKER {
                    return fitserr!("second FITS header must be BITPIX");
                }

                parse_fixed_int(record)?
            };

            let bitpix = match bitpix_value {
                8 => Bitpix::U8,
                16 => Bitpix::I16,
                32 => Bitpix::I32,
                64 => Bitpix::I64,
                -32 => Bitpix::F32,
                -64 => Bitpix::F64,
                other => {
                    return fitserr!("unsupported BITPIX value in FITS file: {}", other);
                }
            };

            // Next: NAXIS

            let mut naxis = Vec::new();

            let naxis_value = {
                let record = &buf[160..240];

                if &record[..NAXIS_MARKER.len()] != NAXIS_MARKER {
                    return fitserr!("third FITS header must be NAXIS");
                }

                parse_fixed_int(record)?
            };

            if naxis_value < 0 || naxis_value > 999 {
                return fitserr!("unsupported NAXIS value in FITS file: {}", naxis_value);
            }

            naxis.reserve(naxis_value as usize);

            // From here on out we have to read dynamically.

            let mut buf_offset = 240;
            let mut seen_groups = hdus.len() > 0; // non-primary HDUs all have PCOUNT and GCOUNT.
            let mut pcount = 0;
            let mut gcount = 1;
            let mut n_header_records = 3; // SIMPLE/XTENSION; BITPIX; NAXIS
            let mut extname = None;

            loop {
                if buf_offset == 2880 {
                    inner.read_exact(&mut buf)?;
                    cur_offset += 2880;
                    buf_offset = 0;
                }

                let record = &buf[buf_offset..buf_offset + 80];

                if accumulate_naxis_value(record, &mut naxis)? {
                    // OK, new naxis value has been handled.
                } else if &record[..GROUPS_MARKER.len()] == GROUPS_MARKER {
                    seen_groups = true;
                } else if seen_groups && &record[..PCOUNT_MARKER.len()] == PCOUNT_MARKER {
                    pcount = parse_fixed_int(record)?;
                } else if seen_groups && &record[..GCOUNT_MARKER.len()] == GCOUNT_MARKER {
                    let n = parse_fixed_int(record)?;

                    if n < 0 {
                        return fitserr!("illegal negative FITS GCOUNT value");
                    }

                    gcount = n as usize;
                } else if &record[..EXTNAME_MARKER.len()] == EXTNAME_MARKER {
                    extname = Some(parse_fixed_string(record)?);
                } else if record == END_MARKER {
                    break;
                }

                n_header_records += 1;
                buf_offset += 80;
            }

            // OK, we're at the END record.

            let extname = if hdus.len() == 0 {
                "".to_owned()
            } else {
                match extname {
                    Some(s) => s,
                    None => {
                        return fitserr!("illegal extension HDU without EXTNAME header");
                    }
                }
            };

            if seen_groups && hdus.len() == 0 {
                naxis.remove(0); // dummy 0 value when primary HDU is random-groups
            }

            let group_size = pcount + naxis.iter().fold(1, |p, n| p * n) as isize;

            if group_size < 0 {
                return fitserr!("illegal negative FITS group size");
            }

            let data_size = bitpix.n_bytes() * gcount * group_size as usize;

            if hdus.len() == 0 {
                kind = if data_size == 0 {
                    HduKind::PrimaryNoData
                } else if seen_groups {
                    HduKind::PrimaryRandomGroups
                } else {
                    HduKind::PrimaryArray
                };
            }

            hdus.push(ParsedHdu {
                kind: kind,
                name: extname,
                header_offset: hdu_header_offset,
                n_header_records: n_header_records,
                bitpix: bitpix,
                pcount: pcount,
                gcount: gcount,
                naxis: naxis,
            });

            // If there's more stuff in the file, skip up to the next HDU
            // beginning (or maaaybe "special records").

            hdu_header_offset = cur_offset + (((data_size + 2879) / 2880) * 2880) as u64;

            if hdu_header_offset == file_size {
                break;
            }

            inner.seek(SeekFrom::Start(hdu_header_offset))?;
        }

        Ok(Self {
            inner: inner,
            hdus: hdus,
            special_record_size: special_record_size,
        })
    }

    /// Get the set of HDUs that comprise this file.
    pub fn hdus(&self) -> &[ParsedHdu] {
        &self.hdus[..]
    }

    /// Consume this parser and return the inner stream.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl ParsedHdu {
    /// Get the "name" of this HDU. If this is an extension HDU, this is the
    /// value of the EXTNAME header keyword. For the primary HDU, it is an
    /// empty string.
    pub fn extname(&self) -> &str {
        &self.name
    }

    /// Query what kind of HDU this is.
    pub fn kind(&self) -> HduKind {
        self.kind.clone()
    }

    /// Query the "BITPIX" of this HDU, which defines the format in which
    /// its data are stored.
    pub fn bitpix(&self) -> Bitpix {
        self.bitpix
    }

    /// Query the shape of this HDU's data. Returns `(gcount, pcount, naxis)`.
    pub fn shape(&self) -> (usize, isize, &[usize]) {
        (self.gcount, self.pcount, &self.naxis[..])
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
            b'0' => {}
            b'1' => {
                value += 1;
            }
            b'2' => {
                value += 2;
            }
            b'3' => {
                value += 3;
            }
            b'4' => {
                value += 4;
            }
            b'5' => {
                value += 5;
            }
            b'6' => {
                value += 6;
            }
            b'7' => {
                value += 7;
            }
            b'8' => {
                value += 8;
            }
            b'9' => {
                value += 9;
            }
            other => {
                return fitserr!(
                    "expected digit but got ASCII {:?} in fixed-format integer",
                    other
                );
            }
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
    let r = b"NAXIS   =          -2147483648 / comment                                        ";
    assert_eq!(parse_fixed_int(r).unwrap(), -2147483648);
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

fn parse_fixed_string(record: &[u8]) -> Result<String, Error> {
    if &record[8..11] != b"= '" {
        return fitserr!("expected opening equals and quote in fixed-format string record");
    }

    let mut buf = [0u8; 69];
    let mut n_chars = 0;
    let mut any_chars = false;
    let mut last_non_blank_pos = 0;
    let mut state = State::Chars;

    const SINGLE_QUOTE: u8 = 0x27;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum State {
        Chars,
        JustSawSingleQuote,
        PreCommentSpaces,
        Comment,
    }

    for i in 0..69 {
        let c = record[i + 11];

        if c < 0x20 || c > 0x7E {
            return fitserr!("illegal non-printable-ASCII value in fixed-format string record");
        }

        match state {
            State::Chars => {
                if c == SINGLE_QUOTE {
                    state = State::JustSawSingleQuote;
                } else {
                    buf[n_chars] = c;

                    if c != b' ' {
                        last_non_blank_pos = n_chars;
                    }

                    n_chars += 1;
                    any_chars = true;
                }
            }

            State::JustSawSingleQuote => match c {
                SINGLE_QUOTE => {
                    buf[n_chars] = SINGLE_QUOTE;
                    last_non_blank_pos = n_chars;
                    n_chars += 1;
                    any_chars = true;
                    state = State::Chars;
                }

                b' ' => {
                    state = State::PreCommentSpaces;
                }

                b'/' => {
                    state = State::Comment;
                }

                other => {
                    return fitserr!(
                        "illegal ASCII value {} after single quote in \
                         fixed-format string record",
                        other
                    );
                }
            },

            State::PreCommentSpaces => match c {
                b' ' => {}

                b'/' => {
                    state = State::Comment;
                }

                other => {
                    return fitserr!(
                        "illegal ASCII value {} after string in \
                         fixed-format string record",
                        other
                    );
                }
            },

            State::Comment => {
                break;
            }
        }
    }

    if state == State::Chars {
        return fitserr!("illegal unterminated fixed-format string record");
    }

    Ok(if !any_chars {
        ""
    } else {
        str::from_utf8(&buf[..last_non_blank_pos + 1])?
    }
    .to_owned())
}

#[cfg(test)]
#[test]
fn fixed_string_parsing() {
    let r = b"XTENSION= 'hello'                                                               ";
    assert_eq!(parse_fixed_string(r).unwrap(), "hello");
    let r = b"XTENSION= ''                                                                    ";
    assert_eq!(parse_fixed_string(r).unwrap(), "");
    let r = b"XTENSION= '     '                                                               ";
    assert_eq!(parse_fixed_string(r).unwrap(), " ");
    let r = b"XTENSION= ''''                                                                  ";
    assert_eq!(parse_fixed_string(r).unwrap(), "'");
    let r = b"XTENSION= 'IMAGE   '                                                            ";
    assert_eq!(parse_fixed_string(r).unwrap(), "IMAGE");
    let r = b"XTENSION= 'looooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooong'";
    assert_eq!(
        parse_fixed_string(r).unwrap(),
        "looooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooong"
    );
    let r = b"XTENSION= 'hello'/ok comment goes here                                          ";
    assert_eq!(parse_fixed_string(r).unwrap(), "hello");
    let r = b"XTENSION= 'hello'   / ok comment goes here                                      ";
    assert_eq!(parse_fixed_string(r).unwrap(), "hello");
    let r = b"XTENSION= nope                                                                  ";
    assert!(parse_fixed_string(r).is_err());
    let r = b"XTENSION= 'OK' nope                                                             ";
    assert!(parse_fixed_string(r).is_err());
    let r = b"XTENSION= 'nope                                                                 ";
    assert!(parse_fixed_string(r).is_err());
}

/// Returns Ok(true) if this record in question was the appropriate NAXISnnn
/// header; Ok(false) if it was some other valid-looking header; Err(_) if it
/// looks like it should have been a NAXIS header but something went wrong.
fn accumulate_naxis_value(record: &[u8], naxis: &mut Vec<usize>) -> Result<bool, Error> {
    if &record[..5] != b"NAXIS" {
        return Ok(false); // Th
    }

    if &record[8..10] != b"= " {
        return fitserr!("malformed FITS NAXIS header");
    }

    let mut value = 0;
    let mut i = 5;

    while i < 8 {
        if record[i] == b' ' {
            break;
        }

        value *= 10;

        match record[i] {
            b'0' => {}
            b'1' => {
                value += 1;
            }
            b'2' => {
                value += 2;
            }
            b'3' => {
                value += 3;
            }
            b'4' => {
                value += 4;
            }
            b'5' => {
                value += 5;
            }
            b'6' => {
                value += 6;
            }
            b'7' => {
                value += 7;
            }
            b'8' => {
                value += 8;
            }
            b'9' => {
                value += 9;
            }
            other => {
                return fitserr!("expected digit but got ASCII {:?} in NAXIS header", other);
            }
        }

        i += 1;
    }

    while i < 8 {
        if record[i] != b' ' {
            return fitserr!(
                "expected space but got ASCII {:?} in NAXIS header",
                record[i]
            );
        }

        i += 1;
    }

    if value != naxis.len() + 1 {
        return fitserr!(
            "misnumbered NAXIS header (expected {}, got {})",
            naxis.len() + 1,
            value
        );
    }

    let n = parse_fixed_int(record)?;

    if n < 0 {
        return fitserr!("illegal negative NAXIS{} value {}", value, n);
    }

    naxis.push(n as usize);
    Ok(true)
}
