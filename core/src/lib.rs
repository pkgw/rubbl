// Copyright 2017 Peter Williams
// Licensed under the MIT License.

extern crate byteorder;
#[macro_use] extern crate error_chain;


#[macro_use] pub mod errors; // must come first to provide macros for other modules
pub mod io;
