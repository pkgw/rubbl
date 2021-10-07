# rubbl_core 0.3.0 (2021-10-07)

- Add a dependency on the `approx` crate so that we provide implementations
  of its traits on our Complex type. If we don't do this, it's not possible
  for downstream users to do so. This extra dep could be gated behind a Cargo
  feature, but it's lightweight so we just add it unconditionally to keep
  things simple (#169, #170, @derwentx)
- Increase the flexibility of allowed versions of the `ndarray` and
  `num-complex` dependencies (#154, @cjordan)


# rubbl_core 0.2.1 (2021-04-01)

- Update versions of core dependencies


# `rubbl_core` 0.2.0 (2020-12-15)

- Bump to the 2018 edition
- Bump ndarray to the 0.14 series


# `rubbl_core` 0.1.3 (2020-09-28)

- Start using Cranko for releases
- Fix some compile warnings


# `rubbl_core` 0.1.2 (2019 Jun 10)

- Use `Error.iter_chain()` rather than the deprecated `.causes()`
- Update a bunch of dependencies
- `rustfmt` all the source
