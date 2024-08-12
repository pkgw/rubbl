# `rubbl_core`

This crate defines some core types used by the Rubbl framework:

- I/O helpers
- Error handling
- Numeric array types

# Crate Duplication

This crate depends on several foundational crates that your upstream project may
also explicitly depend on:

- [`anyhow`] 1.0 (optionally)
- [`byteorder`] 1.4
- [`clap`] 4.0 (optionally)
- [`ndarray`] 0.16
- [`num-complex`] 0.4
- [`termcolor`] 1.1 (optionally)
- [`thiserror`] 1.0

[`anyhow`]: https://crates.io/crates/anyhow/
[`byteorder`]: https://crates.io/crates/anyhow/
[`clap`]: https://crates.io/crates/anyhow/
[`ndarray`]: https://crates.io/crates/anyhow/
[`num-complex`]: https://crates.io/crates/anyhow/
[`termcolor`]: https://crates.io/crates/anyhow/
[`thiserror`]: https://crates.io/crates/anyhow/

If your project depends on a version of one of these crates that is not
compatible with the version required by *this* crate, Cargo will include
multiple versions of it in your build. These duplicates have the same name but
cannot be intermixed. (Note that, according to [semver], versions `0.X` and
`0.(X+1)` of a crate are *not* considered compatible.)

Unfortunately, due to [limitations in Cargo's current resolver][1], this crate
has to specify a narrow compatibility range for these dependencies. So, be
careful to match your version requirements to the ones appropriate to the
version of `rubbl_core` that you are using. The command [`cargo tree -d`][2]
will display any duplicates in your dependency tree. If absolutely necessary,
this crate re-exports some of its key dependencies, which can be used to “name”
them in a reliable way even if they are duplicated elsewhere.

[semver]: https://semver.org/
[1]: https://github.com/rust-lang/cargo/issues/9029
[2]: https://doc.rust-lang.org/cargo/commands/cargo-tree.html