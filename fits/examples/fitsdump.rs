//! Decode a FITS file in a very low-level way, and report how long it took.
//! This should basically just be a test of the system's I/O throughput.

use clap::{Arg, Command};
use failure::{Error, ResultExt};
use rubbl_core::io::AligningReader;
use rubbl_fits::LowLevelFitsItem;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::process;
use std::str;
use std::time::Instant;

fn main() {
    let matches = Command::new("fitsdump")
        .version("0.1.0")
        .about("Parse and dump a FITS data file in low-level fashion.")
        .arg(
            Arg::new("PATH")
                .help("The path to the data file")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.get_one::<OsString>("PATH").unwrap();

    process::exit(match inner(path.as_ref()) {
        Ok(code) => code,

        Err(e) => {
            println!("fatal error while processing {}", path.to_string_lossy());
            for cause in e.iter_chain() {
                println!("  caused by: {}", cause);
            }
            1
        }
    });
}

fn inner(path: &OsStr) -> Result<i32, Error> {
    let file = fs::File::open(path).context("error opening file")?;
    let mut dec = rubbl_fits::FitsDecoder::new(AligningReader::new(file));
    let t0 = Instant::now();
    let mut last_was_data = false;

    loop {
        match dec.next().context("error parsing FITS")? {
            None => {
                break;
            }
            Some(item) => match item {
                LowLevelFitsItem::Header(rec) => {
                    println!("{}", str::from_utf8(rec)?);
                    last_was_data = false;
                }

                LowLevelFitsItem::EndOfHeaders(n_bytes) => {
                    println!("-- end of headers (expect {} bytes of data) --", n_bytes);
                    last_was_data = false;
                }

                LowLevelFitsItem::Data(_) => {
                    if !last_was_data {
                        println!("data ...");
                    }

                    last_was_data = true;
                }

                LowLevelFitsItem::SpecialRecordData(_) => {
                    println!("-- block of \"special record\" data --");
                    last_was_data = false;
                }
            },
        }
    }

    let n_bytes = dec.into_inner().offset();
    let mib = n_bytes as f64 / (1024. * 1024.);
    let dur = t0.elapsed();
    let dur_secs = dur.subsec_nanos() as f64 * 1e-9 + dur.as_secs() as f64;

    println!(
        "{:.1} MiB in {:.3} seconds = {:.3} MiB/s",
        mib,
        dur_secs,
        mib / dur_secs
    );
    Ok(0)
}
