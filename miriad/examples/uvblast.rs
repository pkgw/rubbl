/*!

Read a UV dataset file and report how long it took. This should basically just
be a test of the system's I/O throughput.

 */

extern crate clap;
extern crate rubbl_miriad;

use clap::{Arg, App};


fn main() {
    let matches = App::new("uvblast")
        .version("0.1.0")
        .about("Read a MIRIAD UV data set as fast as possible.")
        .arg(Arg::with_name("PATH")
             .help("The path to the dataset directory")
             .required(true)
             .index(1))
        .get_matches();

    let path = matches.value_of_os("PATH").unwrap();

    let ds = match rubbl_miriad::DataSet::open(&path) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("error opening {}: {}", path.to_string_lossy(), e);
            std::process::exit(1);
        }
    };
}
