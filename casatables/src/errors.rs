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

use glue;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Core(rubbl_core::errors::Error, rubbl_core::errors::ErrorKind);
    }

    errors {
        CasacoreException(msg: String) {
            description("an error from inside the \"casacore\" codebase"),
            display("{}", msg),
        }

        NotScalarColumn {
            description("the column is not made out of scalar values"),
            display("the column is not made out of scalar values"),
        }

        UnexpectedCasaType(casa_type: glue::GlueDataType) {
            description("the CASA data have an unexpected type"),
            display("the CASA data have an unexpected type ({:?})", casa_type),
        }
    }
}
