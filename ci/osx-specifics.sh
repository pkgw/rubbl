#! /usr/bin/env bash
# Copyright 2019 Peter Williams and collaborators
# Licensed under the MIT License

# The `openat` dependency currently needs patching to build on macOS.

set -ex

git clone -b mac https://github.com/pkgw/openat

cat <<EOF >>Cargo.toml
[patch.crates-io]
openat = { path = "./openat" }
EOF
