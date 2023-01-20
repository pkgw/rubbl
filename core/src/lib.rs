// Copyright 2017-2023 Peter Williams and collaborators
// Licensed under the MIT License.

//! Core types and concepts of the Rubbl framework.
//!
//! This crate provides low-level types that are expected to be used throughout
//! the Rubbl framework.
//!
//! # Crate Duplication and Re-Exports
//!
//! This crate depends on several foundational crates that your upstream project
//! may also explicitly depend on, such as [`ndarray`]. If your project depends
//! on a version of one of these crates that is not compatible with the version
//! required by this crate, Cargo will build duplicated versions of these crates
//! that, while they have the same name, cannot be intermixed. See [this crate’s
//! Crates.io README][1] for a more detailed discussion.
//!
//! [1]: https://crates.io/crates/rubbl_core/
//!
//! If you are in a situation where you can't avoid this duplication, this crate
//! re-exports some of its dependencies, providing a way to reliably name the specific
//! version that it’s referencing.

#![deny(missing_docs)]

// convenience re-exports; these can help consumers make sure they're referencing the
// same types if a crate gets duplicated. See also the README.
pub use anyhow;
pub use ndarray::{self, Array, CowArray};
pub use num_complex::{self, Complex};

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
