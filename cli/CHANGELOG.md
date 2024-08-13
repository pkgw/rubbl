# rc: micro bump

There are no code changes in this release of the Rubbl CLI. There are, however,
significant version bumps of the dependencies `anyhow`, `clap`, and `thiserror`.

Associated with this release is a new version of the `rubbl_casatables` crate,
which updates to casacore 3.5.0 and should add the ability to build on a wider
variety of platforms.

In tandem, the Rubbl crates have updated to [ndarray] 0.16 (#393, @pkgw). If
your code uses ndarray directly, you should make sure to update the version
requirement in your `Cargo.toml` files correspondingly.

[ndarray]: https://github.com/rust-ndarray/ndarray

Besides the above, the other crates in the Rubbl family have also received new
maintenance releases, but mostly don't contain comparably noteworthy changes.

The DOI of this release is [xx.xxxx/dev-build.rubbl.version][vdoi].

[vdoi]: https://doi.org/xx.xxxx/dev-build.rubbl.version


# rubbl 0.3.0 (2023-01-23)

This release of Rubbl adds automated DOI deposition to Zenodo (#230, @pkgw)!
This means that releases of the Rubbl suite are now formally published and
citable. Use `rubbl show version-doi` to print out the DOI of the Rubbl CLI tool
that you’re using.

Other changes include:

- Start using the more modern `anyhow` and `thiserror` crates for error handling,
  rather than `failure` (#220, @cjordan).
- Clean up dependency specifications, and document them somewhat more clearly
  (#220, @cjordan, @pkgw).
- Update to the 4.x series of clap, when it's used (#198, @pkgw).

The DOI of this release is [10.5281/zenodo.7563020][vdoi].

[vdoi]: https://doi.org/10.5281/zenodo.7563020


# rubbl 0.2.2 (2020-12-15)

- No code changes from notional 0.2.1, but 0.2.1 *also* wasn't successfully
  published to Cargo.

# rubbl 0.2.1 (2020-12-15)

- No code changes from notional 0.2.0, but 0.2.0 wasn't successfully published
  to Cargo.

# rubbl 0.2.0 (2020-12-15)

- Bump to the 2018 edition

# rubbl 0.1.2 (2020-09-28)

- Actually publish this to Crates.io so that people can install the CLI as
  documented.

# rubbl 0.1.1 (2020-09-28)

- Fix some compile warnings from old-style code
- New release powered by Cranko
