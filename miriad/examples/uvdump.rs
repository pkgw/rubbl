//! Decode the low-level details of MIRIAD UV data.

use anyhow::{Context, Error};
use clap::{Arg, Command};
use std::ffi::{OsStr, OsString};
use std::io;
use std::process;

fn main() {
    let matches = Command::new("uvdump")
        .version("0.1.0")
        .about("Decode MIRIAD UV data verbosely.")
        .arg(
            Arg::new("PATH")
                .help("The path to the dataset directory")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.get_one::<OsString>("PATH").unwrap();

    process::exit(match inner(path.as_ref()) {
        Ok(code) => code,

        Err(e) => {
            println!("fatal error while processing {}", path.to_string_lossy());
            for cause in e.chain() {
                println!("  caused by: {}", cause);
            }
            1
        }
    });
}

fn inner(path: &OsStr) -> Result<i32, Error> {
    let mut ds = rubbl_miriad::DataSet::open(path).with_context(|| "error opening dataset")?;
    let mut uv = ds
        .open_uv()
        .with_context(|| "could not open as UV dataset")?;
    uv.dump_diagnostic(io::stdout())?;
    Ok(0)
}
