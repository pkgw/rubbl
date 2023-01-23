# rubbl_core 0.4.0 (2023-01-23)

- Start using the more modern `anyhow` and `thiserror` crates for error
  handling, rather than `failure` (#220, @cjordan).
- Clean up dependency specifications, and document them somewhat more clearly
  (#220, @cjordan, @pkgw). In particular, this version of the core crate now
  requires `ndarray 0.15.x`. We previously attempted to have a more open version
  requirement, but due to the behavior of the Cargo dependency resolver, it
  didn't actually buy us anything. See discussion in #220 for more details.
- Relatedly, remove the dependency on the `approx` crate.
- Update to the 4.x series of `clap`, when it's used (#198, @pkgw).



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
