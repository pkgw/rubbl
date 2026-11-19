# `rubbl_casatables`

A Rust interface to the CASA table format.

See [the `rubbl_core` README on Crates.io][1] for a discussion of crate
duplication issues that may arise with key dependencies such as [`ndarray`][2].

[1]: https://crates.io/crates/rubbl_core/
[2]: https://crates.io/crates/ndarray/

## Dysco

Tables (typically MeasurementSets) whose columns are compressed with the
[Dysco](https://github.com/aroffringa/dysco) storage manager can be read and
written when the `dysco` Cargo feature is enabled:

```toml
rubbl_casatables = { version = "...", features = ["dysco"] }
```

This compiles the Dysco storage manager bundled with casacore into
`rubbl_casatables_impl` and links against [GSL](https://www.gnu.org/software/gsl/),
so the GSL headers and libraries must be installed (e.g. `apt install
libgsl-dev` or `brew install gsl`). Without the feature, opening a
Dysco-compressed column fails with an error saying that the `dyscostman`
library could not be found.
