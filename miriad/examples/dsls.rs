/*!

Describe the contents of a generic MIRIAD data set.

 */

extern crate clap;
extern crate rubbl_miriad;

use clap::{Arg, App};


fn main() {
    let matches = App::new("dsls")
        .version("0.1.0")
        .about("Describe the contents of a MIRIAD data set.")
        .arg(Arg::with_name("PATH")
             .help("The path to the dataset directory")
             .required(true)
             .index(1))
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

    println!("ncorr: {}", ds.get("ncorr").unwrap().read_scalar::<i64>().expect("error extracting ncorr"));
    println!("nwcorr: {}", ds.get("nwcorr").unwrap().read_scalar::<i64>().expect("error extracting nwcorr"));
    println!("vislen: {}", ds.get("vislen").unwrap().read_scalar::<i64>().expect("error extracting vislen"));
    println!("obstype: {}", ds.get("obstype").unwrap().read_scalar::<String>().expect("error extracting obstype"));
}
