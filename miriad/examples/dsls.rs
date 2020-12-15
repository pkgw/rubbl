//! Describe the contents of a generic MIRIAD data set.

use clap::{App, Arg};

fn get<T: rubbl_miriad::MiriadMappedType>(ds: &mut rubbl_miriad::DataSet, name: &str) -> T {
    ds.get(name)
        .unwrap()
        .unwrap()
        .read_scalar::<T>()
        .expect("error reading item")
}

fn main() {
    let matches = App::new("dsls")
        .version("0.1.0")
        .about("Describe the contents of a MIRIAD data set.")
        .arg(
            Arg::with_name("PATH")
                .help("The path to the dataset directory")
                .required(true)
                .index(1),
        )
        .get_matches();

    let path = matches.value_of_os("PATH").unwrap();

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
