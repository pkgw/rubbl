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

    let ds = match rubbl_miriad::DataSet::open(path) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("error opening {}: {}", path.to_string_lossy(), e);
            std::process::exit(1);
        }
    };
    
    for item_name in ds.item_names().expect("cannot scan directory") {
        //let ii = ds.item_info(&item_name);
        //println!("{:8}  {:8}  {}", item_name, ii.ty, ii.n_vals);
        println!("{}", item_name);
    }

    println!("ncorr: {}", ds.read_scalar_item::<i64>("ncorr").expect("error extracting ncorr"));
    println!("nwcorr: {}", ds.read_scalar_item::<i64>("nwcorr").expect("error extracting nwcorr"));
    println!("vislen: {}", ds.read_scalar_item::<i64>("vislen").expect("error extracting vislen"));
    println!("obstype: {}", ds.read_scalar_item::<String>("obstype").expect("error extracting obstype"));
}
