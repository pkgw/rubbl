/*!

Print out the values of all of the UV variables as they are set in the very
first record of a UV data stream.

 */

extern crate clap;
extern crate failure;
extern crate rubbl_miriad;

use clap::{App, Arg};
use failure::{Error, ResultExt};
use std::ffi::OsStr;
use std::process;

fn main() {
    let matches = App::new("uvheadervars")
        .version("0.1.0")
        .about("Print initial values of UV variables")
        .arg(
            Arg::with_name("PATH")
                .help("The path to the dataset directory")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.value_of_os("PATH").unwrap();

    process::exit(match inner(path.as_ref()) {
        Ok(code) => code,

        Err(e) => {
            println!("fatal error while processing {}", path.to_string_lossy());
            for cause in e.causes() {
                println!("  caused by: {}", cause);
            }
            1
        }
    });
}

fn inner(path: &OsStr) -> Result<i32, Error> {
    let mut ds = rubbl_miriad::DataSet::open(path).context("error opening input dataset")?;
    let mut uv = ds.open_uv().context("could not open input as UV dataset")?;

    uv.next().context("could not read UV data")?;

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
