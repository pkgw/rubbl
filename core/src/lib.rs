// Copyright 2017 Peter Williams
// Licensed under the MIT License.

/*!

Core types and concepts of the Rubbl framework.

This crate provides low-level types that are expected to be used throughout
the Rubbl framework.

*/

#![deny(missing_docs)]

// convenience re-exports
use anyhow::Context;
pub use anyhow::Error;
pub use ndarray::Array;
pub use num_complex::Complex;

/// A “contextualized try” macro.
///
/// Attempts an operation that returns a Result and returns its Ok value if
/// the operation is successful. If not, it returns an Err value of type
/// `anyhow::Context` that includes explanatory text formatted using the
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
            use $crate::Error;
            $op.with_context(|_| format!($( $chain_fmt_args ),*))?
        }
    }
}

pub mod io;
pub mod notify;
pub mod num;

/// A convenience Result type whose error half is fixed to be
/// `failure::Error`.
pub type Result<T> = ::std::result::Result<T, anyhow::Error>;
