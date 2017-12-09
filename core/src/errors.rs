// Copyright 2017 Peter Williams and collaborators
// Licensed under the MIT License.

/*!
This module implements the core error types used by Rubbl crates.

It provides a generic, chainable error type using the infrastructure provided
by the very nice [error-chain](https://docs.rs/error-chain) crate.

This module is ripped off from the `errors` module used by the
[Tectonic](https://github.com/tectonic-typesetting/tectonic) typesetting
engine. (Which the author of this module also wrote.)

*/

use std::{convert, io, num, str};


error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(io::Error) #[doc = "An I/O-related error."];
        ParseInt(num::ParseIntError) #[doc = "An error related to parsing integers."];
        Utf8(str::Utf8Error) #[doc = "An error related to UTF-8 conversion."];
    }

    errors {
        /// An Error that occurs when an array did not have the expected dimensionality.
        DimensionMismatch(expected: usize, actual: usize) {
            description("the dimensions of two multi-dimensional arrays did not match"),
            display("expected {}-dimensional array but found one that was {}-dimensional",
                    expected, actual),
        }
    }
}


/// A “chained try” macro.
///
/// Attempts an operation that returns a Result and returns its Ok value if
/// the operation is successful. If not, it returns an Err value that chains
/// to the Err value that was returned. The Err has an ErrorKind of Msg and
/// includes explanatory text formatted using the `format!` macro. Example:
///
/// ```rust
/// // At top of crate:
/// #[macro_use] extern crate rubbl_core;
///
/// // Elsewhere:
/// use std::io::{Result, Write, stdout};
///
/// fn write_or_complain() -> Result<()> {
///     let dest = "standard output";
///     ctry!(write!(stdout(), "hello"); "couldn\'t write to {}", dest);
///     Ok(())
/// }
/// # 
/// # fn main() {
/// # write_or_complain().unwrap();
/// # }
/// ```
///
/// Note that the operation to be attempted and the arguments to `format!` are
/// separated by a semicolon within the `ctry!()` parentheses.
#[macro_export]
macro_rules! ctry {
    ($op:expr ; $( $chain_fmt_args:expr ),*) => {
        {
            use $crate::errors::ResultExt;
            $op.chain_err(|| format!($( $chain_fmt_args ),*))?
        }
    }
}


/// Format an error message.
///
/// This convenience macro expands into an Err(Error) object of kind
/// ErrorKind::Msg, and a message formatted using the standard `format!`
/// machinery.
#[macro_export]
macro_rules! err_msg {
    ($( $fmt_args:expr ),*) => {
        Err($crate::errors::ErrorKind::Msg(format!($( $fmt_args ),*)).into())
    }
}

impl convert::From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, format!("{}", err))
    }
}


//impl Error {
//    /// Write the information contained in this object to standard error in a
//    /// somewhat user-friendly form.
//    ///
//    /// The `error_chain` crate provides a Display impl for its Error objects
//    /// that ought to provide this functionality, but I have had enormous
//    /// trouble being able to use it. So instead we emulate their code. The
//    /// CLI program provides very similar code that produces similar output
//    /// but with fancy colorization.
//    pub fn dump_uncolorized(&self) {
//        let mut prefix = "error:";
//
//        for item in self.iter() {
//            eprintln!("{} {}", prefix, item);
//            prefix = "caused by:";
//        }
//
//        if let Some(backtrace) = self.backtrace() {
//            eprintln!("debugging: backtrace follows:");
//            eprintln!("{:?}", backtrace);
//        }
//    }
//}
