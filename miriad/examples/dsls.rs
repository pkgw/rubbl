//! Describe the contents of a generic MIRIAD data set.

use clap::{Arg, Command};
use std::ffi::OsString;

fn get<T: rubbl_miriad::MiriadMappedType>(ds: &mut rubbl_miriad::DataSet, name: &str) -> T {
    ds.get(name)
        .unwrap()
        .unwrap()
        .read_scalar::<T>()
        .expect("error reading item")
}

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

    for item in ds.items().expect("cannot scan directory") {
        println!("{:8}  {:8}  {}", item.name(), item.type_(), item.n_vals());
    }

    println!("ncorr: {}", get::<i64>(&mut ds, "ncorr"));
    println!("nwcorr: {}", get::<i64>(&mut ds, "nwcorr"));
    println!("vislen: {}", get::<i64>(&mut ds, "vislen"));
    println!("obstype: {}", get::<String>(&mut ds, "obstype"));
}
