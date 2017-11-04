// Copyright 2017 Peter Williams and collaborators
// Licensed under the MIT License.

/*!
This module implements the error types used by the blobman crate.

It provides a generic, chainable error type using the infrastructure provided
by the very nice [error-chain](https://docs.rs/error-chain) crate.

This module is ripped off from the `errors` module used by the
[Tectonic](https://github.com/tectonic-typesetting/tectonic) typesetting
engine. (Which the author of this module also wrote.)

*/

use rubbl_core;


error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Core(rubbl_core::errors::Error, rubbl_core::errors::ErrorKind);
    }

    errors {
        CasacoreException(msg: String) {
            description("an error from inside the \"casacore\" codebase")
                display("{}", msg)
        }
    }
}
