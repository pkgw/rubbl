/*!

Read a UV dataset file and report how long it took. This should basically just
be a test of the system's I/O throughput.

 */

extern crate clap;
extern crate rubbl_miriad;

use clap::{Arg, App};
use std::time::Instant;


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

    let mut ds = match rubbl_miriad::DataSet::open(path) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("error opening {}: {}", path.to_string_lossy(), e);
            std::process::exit(1);
        }
    };

    let mut uv = ds.open_uv().expect("could not open as UV dataset");
    let mib = uv.visdata_bytes() as f64 / (1024. * 1024.);
    let mut n = 0usize;
    let t0 = Instant::now();

    while uv.next().expect("could not read UV data") {
        n += 1
    }

    let dur = t0.elapsed();
    let dur_secs = dur.subsec_nanos() as f64 * 1e-9 + dur.as_secs() as f64;

    println!("{} records, {:.1} MiB in {:.3} seconds = {:.3} MiB/s", n, mib, dur_secs, mib / dur_secs);
}
