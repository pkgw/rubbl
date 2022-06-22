// Copyright 2017-2021 Peter Williams and collaborators
// Licensed under the MIT License.

//! Core types and concepts of the Rubbl framework.
//!
//! This crate provides low-level types that are expected to be used throughout
//! the Rubbl framework.

#![deny(missing_docs)]

// convenience re-exports
pub use ndarray::{Array, CowArray};
pub use num_complex::Complex;

// `approx` isn't (as of October 2021) used anywhere in Rubbl, but by including
// it as a dependency, we can get implementations of its traits for the Complex
// type that we export. Re-exporting the crate gives users an ability to name
// these traits if so desired.
//
// We could make this optional with a Cargo feature, for downstream users that
// don't need the functionality, but it's a very lightweight dependency, so
// let's just keep things simple.
pub use approx;

pub mod io;
pub mod notify;
pub mod num;
