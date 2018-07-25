// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

/*!

In the Git checkout, the `src/` directory contains a symbolic link to
`../casatables_impl/casacore`. When building from a checkout, this works
straightforwardly. When creating a package that is uploaded to crates.io,
Cargo does the right thing and copies over all of the files, which is a bit
bloaty but means that we can maintain the helpful casatables/casatables_impl
split.

*/

extern crate cc;


const FILES: &[&str] = &[
    "src/glue.cc",
];

fn main() {
    let mut builder = cc::Build::new();

    builder
        .cpp(true)
        .warnings(true)
        .include("src")
        .files(FILES)
        .compile("libcasatables_glue.a");

    for file in FILES {
        println!("cargo:rerun-if-changed={}", file);
    }
}
