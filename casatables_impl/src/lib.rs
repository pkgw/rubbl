// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

/*! This crate provides the compiled C++ casacore table access code.

The actual interface is handled in a separate module. That way the shim code
that bridges C++ to Rust can be edited without having to recompile the large
casacore codebase every time.

 */
