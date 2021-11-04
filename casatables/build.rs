// Copyright 2017-2020 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

use std::env;

const FILES: &[&str] = &["src/glue.cc"];

fn main() {
    let mut builder = cc::Build::new();

    builder
        .cpp(true)
        .warnings(true)
        .flag_if_supported("-std=c++11")
        // This allows us to treat rubbl's modified casacore as a separate 
        // namespace, so that both vanilla casacore and rubbl can be linked
        // at the same time. 
        .define("casacore", "rubbl_casacore")
        .include("src")
        .include(env::var_os("DEP_CASA_INCLUDE").unwrap())
        .files(FILES)
        .compile("libcasatables_glue.a");

    for file in FILES {
        println!("cargo:rerun-if-changed={}", file);
    }

    // Because our glue.cc references casatables C++ directly, we need to make
    // sure to explicitly link with it. If not, it looks like the dead code
    // elimination may cause link issues when we actually try to link
    // executables.
    println!("cargo:rustc-link-lib=static=casatables_impl");
}
