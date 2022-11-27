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

pub mod io;
#[cfg(feature = "notifications")]
pub mod notify;
pub mod num;
