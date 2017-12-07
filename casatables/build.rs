// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

extern crate gcc;

use std::{env, str};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;


const FILES: &[&str] = &[
    "src/glue.cc",
];

fn main() {
    let mut builder = gcc::Build::new();

    builder
        .cpp(true)
        .warnings(true)
        .include("../casatables_impl")
        .include("src");

    // Here we pause to figure out how big a `casa::String` C++ structure is.
    // We need to know this in order to efficiently trade strings between the
    // Rust and C++ layers, but doing a full bindgen run that parses the C++
    // STL headers to retrieve this number dynamically is *painful*.
    // Therefore, we compile a run a small C++ program that just prints this
    // number out. Here, we save the number in a file within src/. Meanwhile,
    // `gen-bindings.sh` munges the naive output of bindgen to include that
    // file to obtain the right value. This way we have the benefits of simple,
    // pre-generated bindgen bindings *and* fast mapping between C++ and Rust
    // strings.
    //
    // The downside is that we break cross-compilation, but I don't expect
    // that to be a remotely common use case. If it ever comes up, I think the
    // best solution is to allow the string size to be hardcoded through an
    // environment variable.

    let out_dir = env::var("OUT_DIR").unwrap();
    let mut probe_dest = PathBuf::new();
    probe_dest.push(out_dir);
    probe_dest.push("probe_string_size");

    let mut cc_base = builder.get_compiler().to_command();
    cc_base
        .arg("-o")
        .arg(&probe_dest)
        .arg("src/probe_string_size.cc");

    match cc_base.status() {
        Ok(s) if s.success() => {},
        _ => panic!("could not build casacore::String size prober"),
    }

    let output = match Command::new(&probe_dest).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("failed to run casacore::String size prober: {}", e);
            panic!("failed to run casacore::String size prober -- cross-compiling?!");
        }
    };

    if !output.status.success() {
        panic!("casacore::String size prober exited with error code!?");
    }

    let stdout_text = str::from_utf8(&output.stdout[..]).expect("could not decode output of string size prober");
    let str_size: usize = stdout_text
        .lines()
        .next()
        .unwrap()
        .parse()
        .expect("could not parse output of string size prober");

    {
        let mut str_size_file = File::create("src/casa_string_size.txt")
            .expect("could not create \"src/casa_string_size.txt\"");
        writeln!(str_size_file, "{}", str_size)
            .expect("could not write to \"src/casa_string_size.txt\"");
    }

    // Finally we can get back to building our glue library.

    builder
        .files(FILES)
        .compile("libcasatables_glue.a");

    for file in FILES {
        println!("cargo:rerun-if-changed={}", file);
    }
}
