// Copyright 2017 Peter Williams
// Licensed under the MIT License.

extern crate byteorder;
extern crate clap;
#[macro_use] extern crate error_chain;
extern crate ndarray;
extern crate num_complex;
extern crate termcolor;

#[macro_use] pub mod errors; // must come first to provide macros for other modules
pub mod io;
pub mod notify;

pub use ndarray::Array;
pub use num_complex::Complex; // convenience re-export
