//! Print out the values of all of the UV variables as they are set in the very
//! first record of a UV data stream.

use anyhow::{Context, Error};
use clap::{Arg, Command};
use std::ffi::{OsStr, OsString};
use std::process;

fn main() {
    let matches = Command::new("uvheadervars")
        .version("0.1.0")
        .about("Print initial values of UV variables")
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
    let mut ds =
        rubbl_miriad::DataSet::open(path).with_context(|| "error opening input dataset")?;
    let mut uv = ds
        .open_uv()
        .with_context(|| "could not open input as UV dataset")?;

    uv.next().with_context(|| "could not read UV data")?;

    for var in uv.variables() {
        let n = var.n_vals();
        let v = var.get_as_any();

        if n == 1 {
            println!("{} ({}) = {}", var.name(), var.type_(), v);
        } else {
            println!("{} ({}[{}]) = {}", var.name(), var.type_(), n, v);
        }
    }

    Ok(0)
}
