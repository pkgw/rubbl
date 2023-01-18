// Copyright 2017-2023 Peter Williams and collaborators
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

/// A “contextualized try” macro.
///
/// This macro is syntactic sugar. The expression
///
/// ```rust
/// # use rubbl_core::ctry;
/// # use anyhow::*;
/// # fn myfun() -> Result<(), Error> {
/// #   let op: Result<(), Error> = Ok(());
/// #   let value = "something";
/// ctry!(op; "spec: {}", value)
/// #  ;
/// #  Ok(())
/// # }
/// ```
///
/// is equivalent to:
///
/// ```rust
/// # use anyhow::*;
/// # fn myfun() -> Result<(), Error> {
/// #   let op: Result<(), Error> = Ok(());
/// #   let value = "something";
/// {
///     use anyhow::Context;
///     op.with_context(|| format!("spec: {}", value))?
/// }
/// #   Ok(())
/// # }
/// ```
///
/// So, it attempts an operation that returns a [`Result`] (or [`Option`]) and
/// evaluates to its [`Ok`] (or [`Some`]) value if the operation is successful.
/// If not, it exits the current function with an [`Err`] value that has a
/// formatted context string attached to it.
///
/// #### Example
///
/// ```rust
/// # use anyhow::Error;
/// # fn myfun() -> Result<(), Error> {
/// use rubbl_core::ctry;
/// use std::{fs::File, io::Write};
///
/// let path = "myfile.txt";
/// let mut myfile = ctry!(File::open(path); "failed to open file `{}`", path);
/// ctry!(write!(myfile, "hello"); "failed to write to file `{}`", path);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! ctry {
    ($op:expr ; $( $chain_fmt_args:expr ),*) => {
        {
            use anyhow::Context;
            $op.with_context(|| format!($( $chain_fmt_args ),*))?
        }
    }
}
