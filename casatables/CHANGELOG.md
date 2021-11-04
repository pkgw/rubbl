# rc: minor bump

- Require the latest release of `casatables_impl` and track its rename of the
  C++ namespace used for the table I/O implementation (#178, @derwentx). The
  most important effect of this is to make it possible to build an executable
  that links with both Rubbl *and* the "standard" `libcasa_*` shared libraries.
  This is desirable if you want to combine Rubbl-based I/O with existing
  C++/CASA analysis libraries. There might be a possibility of strange issues if
  you use both I/O subsystems on the same data at the same time, but we think
  that you would have to try pretty hard to cause issues.
- Add `TableDesc::set_ndims`, to set the number of dimensions that an array column
  has without fixing the exact shape of the array (#175, #176, @derwentx).
- Fix a bug with the internal handling of string-array columns that prevented them
  from being populated (#174, @derwentx).


# rubbl_casatables 0.4.0 (2021-10-07)

It is now possible to create CASA tables thanks to [@derwentx]!

[@derwentx]: https://github.com/derwentx

- Derive Eq, PartialEq, and Debug for ColumnDescription (#167, #168, @derwentx)
- Add `Table::put_table_keyword` (#164, #166, @derwentx)
- Add `Table::add_scalar_column` and `Table::add_array_column` (#162, #163, @derwentx)
- Add `Table::new` and supporting machinery (#114, #160, @derwentx)
- Increase the flexibility of allowed versions of the `ndarray` and
  `num-complex` dependencies (#154, @cjordan)

# rubbl_casatables 0.3.1 (2021-04-01)

- Remove a bunch of superfluous dependencies

# rubbl_casatables 0.3.0 (2021-04-01)

- Add methods to iterate over particular rows (@cjordan, #146)
- Update versions of a variety of dependencies

# rubbl_casatables 0.2.2 (2020-12-15)

- No code changes from notional 0.2.1, but 0.2.1 *also* wasn't successfully
  published to Cargo.

# rubbl_casatables 0.2.1 (2020-12-15)

- No code changes from notional 0.2.0, but 0.2.0 wasn't successfully published
  to Cargo.

# rubbl_casatables 0.2.0 (2020-12-15)

- Fix retrieval of string values from table columns and cells, I hope.
  (#133, #134)
- Bump to the 2018 edition, which requires a small build tweak
