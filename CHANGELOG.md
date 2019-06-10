# `rubbl_casatables_impl` 0.2.31100 (2019 Jun 10)

- Fix compilation with some older C++ compilers.
- Bump version number — now we can make new releases of this package even if
  the underlying version of casacore stays the same.

# `rubbl_casatables` 0.1.2 (2019 Jun 10)

- Update to `casatables_impl` 0.2.311.
- Add a couple more routines for row-based data retrieval.
- Clearly remove the confusing `spwglue` example, superseded by my `rxpackage`
  version.

# `rubbl_casatables_impl` 0.2.311 (2019 Jun 10)

- Update to casacore 3.1.1 — this version is now encoded in
  the micro version number of this package

# `rubbl_core` 0.1.2 (2019 Jun 10)

- Use `Error.iter_chain()` rather than the deprecated `.causes()`
- Update a bunch of dependencies
- `rustfmt` all the source
