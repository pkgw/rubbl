# rc: minor bump

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
