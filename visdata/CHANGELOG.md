# rc: micro bump

No code changes; just bumping the `anyhow` version.


# rubbl_visdata 0.3.1 (2024-08-13)

- Various internal cleanups, including dependency updates and fixes for Clippy
  complaints (#394, @pkgw). Functionality should not be changed.


# rubbl_visdata 0.3.0 (2023-01-23)

- Start using the more modern `anyhow` and `thiserror` crates for error
  handling, rather than `failure` (#220, @cjordan).
- Clean up dependency specifications, and document them somewhat more clearly
  (#220, @cjordan, @pkgw).
- Update to the 4.x series of clap, when it's used (#198, @pkgw).
