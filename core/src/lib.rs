// Copyright 2017 Peter Williams
// Licensed under the MIT License.

extern crate byteorder;
#[macro_use] extern crate error_chain;
extern crate num_complex;

#[macro_use] pub mod errors; // must come first to provide macros for other modules
pub mod io;

pub use num_complex::Complex; // convenience re-export
