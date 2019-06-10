/*!

Summarize the structure of a FITS file. This uses the FITS "parser" framework,
which seeks around a file, rather than the "decoder" framework which doesn't
seek but therefore has to actually read through all of the data.

 */

extern crate clap;
extern crate failure;
extern crate rubbl_core;
extern crate rubbl_fits;

use clap::{App, Arg};
use failure::{Error, ResultExt};
use std::ffi::OsStr;
use std::fs;
use std::process;

fn main() {
    let matches = App::new("fitssummary")
        .version("0.1.0")
        .about("Summarize the structure of a FITS file.")
        .arg(
            Arg::with_name("PATH")
                .help("The path to the FITS file")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.value_of_os("PATH").unwrap();

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
    let fits = rubbl_fits::FitsParser::new(file)?;

    for (num, hdu) in fits.hdus().iter().enumerate() {
        let extname_display = if num == 0 { "(primary)" } else { hdu.extname() };

        println!("HDU #{}: {:?} {}", num, hdu.kind(), extname_display);
        println!("    bitpix: {:?}", hdu.bitpix());

        let (gcount, pcount, naxis) = hdu.shape();
        println!("    shape: {:?} pcount={} gcount={}", naxis, pcount, gcount);
    }

    Ok(0)
}
