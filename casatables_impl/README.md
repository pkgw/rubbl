# `rubbl_casatables_impl`

The C++ code backing the Rubbl interface to the CASA table format.

## Rationale

This crate contains no actual Rust code — it just provides a mechanism to
compile the large C++ library that backs the `rubbl_casatables` crate. This
way, we can iterate the crate and the C++ glue layer that binds the two,
without having to recompile 300 C++ files every time the glue layer changes.

## Versioning

The micro version of this package takes the form "MMMNN", where "MMM" is the
version of casacore upon which the C++ codebase is derived, except without any
periods. For instance, casacore 3.1.1 becomes "311". The "NN" component allows
for us to make up to 100 releases of the Rust crate between updates to the
version of casacore.

Note that this numbering scheme will break if casacore hits a version like
3.1.10. We'll cross that bridge when we get there.

## Crate Duplication

See [the `rubbl_core` README on Crates.io][1] for a discussion of crate
duplication issues that may arise with key dependencies such as [`ndarray`][2].

[1]: https://crates.io/crates/rubbl_core/
[2]: https://crates.io/crates/ndarray/
