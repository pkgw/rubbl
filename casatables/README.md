# `rubbl_casatables`

A Rust interface to the CASA table format.


## Publishing to crates.io

Publishing this crate to crates.io is a bit of a pain because it needs the
header files that are stored in `../casatables_impl`. (The two crates are, in
turn, split to save on compilation time for the large CASA tables C++
codebase.) The header files are referenced via a symlink in `src/casacore`
that points to `../../casatables_impl/casacore`.

However, `cargo publish`
[can't handle such a symlink](https://github.com/rust-lang/cargo/issues/2748).
(It used to be able to …) The best solution I can identify is to manually copy
the files temporarily. You need to add them to Git because `cargo` uses Git's
listing of files to know what to publish. The procedure is:

1. `rm src/casacore`
2. `cp -r ../casatables_impl/casacore src/`
3. `git add src/casacore`
4. `cargo publish --allow-dirty`
5. `git reset HEAD src/casacore`
6. `rm -rf src/casacore`
7. `git checkout src/casacore`
