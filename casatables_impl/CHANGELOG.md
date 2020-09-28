# rubbl_casatables_impl 0.2.31101 (2020-09-28)

- Update the license to LGPL 2 rather than GPL 2 — this was adopted by the
  casacore project shortly after I copied the core sources into this package.
- Make this a `-sys` type package, even though the name disagrees (not great),
  saying that it links with the `casa` library
- Install headers into a Cargo target output directory so that depending
  packages can build against our C++ code. This is the "right" way to split up
  the base package and `rubbl_casatables` package.

# `rubbl_casatables_impl` 0.2.31100 (2019 Jun 10)

- Fix compilation with some older C++ compilers.
- Bump version number — now we can make new releases of this package even if the
  underlying version of casacore stays the same.

# `rubbl_casatables_impl` 0.2.311 (2019 Jun 10)

- Update to casacore 3.1.1 — this version is now encoded in the micro version
  number of this package
