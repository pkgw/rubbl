# rubbl_casatables_impl 0.3.31100 (2021-11-04)

- Use a preprocessor `#define` to put the underlying C++ code in the namespace
  `rubbl_casacore` (#178, @derwentx). The most important effect of this is to
  make it possible to build an executable that links with both Rubbl *and* the
  "standard" `libcasa_*` shared libraries. This is desirable if you want to
  combine Rubbl-based I/O with existing C++/CASA analysis libraries. There might
  be a possibility of strange issues if you use both I/O subsystems on the same
  data at the same time, but we think that you would have to try pretty hard to
  cause issues.


# rubbl_casatables_impl 0.2.31105 (2021-10-07)

- Define `USE_THREADS=1` when building the C++ code to make it threadsafe. Yikes!
  (@pkgw)

# rubbl_casatables_impl 0.2.31104 (2020-12-15)

- No code changes from the previous release, but it *also* wasn't successfully
  published to Cargo.

# rubbl_casatables_impl 0.2.31103 (2020-12-15)

- No code changes from the previous release, but that wasn't successfully
  published to Cargo.

# rubbl_casatables_impl 0.2.31102 (2020-12-15)

- Bump to the 2018 edition
