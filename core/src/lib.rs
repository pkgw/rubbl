// Copyright 2017-2021 Peter Williams and collaborators
// Licensed under the MIT License.

//! Core types and concepts of the Rubbl framework.
//!
//! This crate provides low-level types that are expected to be used throughout
//! the Rubbl framework.

#![deny(missing_docs)]

// convenience re-exports
pub use failure::{Error, Fail, ResultExt};
pub use ndarray::Array;
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

/// A “contextualized try” macro.
///
/// Attempts an operation that returns a Result and returns its Ok value if
/// the operation is successful. If not, it returns an Err value of type
/// `failure::Context` that includes explanatory text formatted using the
/// `format!` macro and chains to the causative error. Example:
///
/// ```rust,ignore
/// ctry!(write!(myfile, "hello"); "couldn\'t write to {}", myfile_path);
/// ```
///
/// Note that the operation to be attempted and the arguments to `format!` are
/// separated by a semicolon within the `ctry!()` parentheses.
#[macro_export]
macro_rules! ctry {
    ($op:expr ; $( $chain_fmt_args:expr ),*) => {
        {
            use $crate::ResultExt;
            $op.with_context(|_| format!($( $chain_fmt_args ),*))?
        }
    }
}

pub mod io;
pub mod notify;
pub mod num;

/// A convenience Result type whose error half is fixed to be
/// `failure::Error`.
pub type Result<T> = ::std::result::Result<T, failure::Error>;
