//! Emit the contents of a MIRIAD data set's history item.
//!
//! This can be done trivially since the history item is a standalone file, but
//! this shows how the rubbl MIRIAD API is used.

use clap::{Arg, Command};
use std::ffi::OsString;

fn main() {
    let matches = Command::new("dsls")
        .version("0.1.0")
        .about("Describe the contents of a MIRIAD data set.")
        .arg(
            Arg::new("PATH")
                .help("The path to the dataset directory")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.get_one::<OsString>("PATH").unwrap().as_os_str();

    let mut ds = match rubbl_miriad::DataSet::open(path) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("error opening {}: {}", path.to_string_lossy(), e);
            std::process::exit(1);
        }
    };

    for maybe_line in ds
        .get("history")
        .expect("cannot probe history")
        .expect("no history item")
        .into_lines()
        .expect("cannot open history")
    {
        let line = maybe_line.expect("error reading history");
        println!("{line}");
    }
}
