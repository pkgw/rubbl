// Copyright 2017 Peter Williams and collaborators
// Licensed under the MIT License.

/*!
This module implements specialized error types for the rubbl_casatables crate.

It provides a generic, chainable error type using the infrastructure provided
by the very nice [error-chain](https://docs.rs/error-chain) crate. Its errors
extend those provided in rubbl_core.

*/

use rubbl_core;
use std::num::ParseIntError;

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


impl From<rubbl_core::errors::ErrorKind> for Error {
    fn from(kind: rubbl_core::errors::ErrorKind) -> Error {
        let tmp: rubbl_core::errors::Error = kind.into();
        Error::from(tmp)
    }
}


impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Error {
        let tmp: rubbl_core::errors::Error = err.into();
        Error::from(tmp)
    }
}
