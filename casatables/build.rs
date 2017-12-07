// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

extern crate gcc;


const FILES: &[&str] = &[
    "src/glue.cc",
];

fn main() {
    let mut builder = gcc::Build::new();

    builder
        .cpp(true)
        .warnings(true)
        .include("../casatables_impl")
        .include("src")
        .files(FILES)
        .compile("libcasatables_glue.a");

    for file in FILES {
        println!("cargo:rerun-if-changed={}", file);
    }
}
