//! Read a UV dataset file and report how long it took. This should basically
//! just be a test of the system's I/O throughput.

use anyhow::{Context, Error};
use clap::{Arg, Command};
use std::ffi::{OsStr, OsString};
use std::process;
use std::time::Instant;

fn main() {
    let matches = Command::new("uvblast")
        .version("0.1.0")
        .about("Read a MIRIAD UV data set as fast as possible.")
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
    let mut ds = rubbl_miriad::DataSet::open(path).context("error opening dataset")?;
    let mut uv = ds.open_uv().context("could not open as UV dataset")?;
    let mib = uv.visdata_bytes() as f64 / (1024. * 1024.);
    let mut n = 0usize;
    let t0 = Instant::now();

    while uv.next().context("could not read UV data")? {
        n += 1
    }

    let dur = t0.elapsed();
    let dur_secs = dur.subsec_nanos() as f64 * 1e-9 + dur.as_secs() as f64;

    println!(
        "{} records, {:.1} MiB in {:.3} seconds = {:.3} MiB/s",
        n,
        mib,
        dur_secs,
        mib / dur_secs
    );
    Ok(0)
}
