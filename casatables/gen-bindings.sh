#! /bin/sh
# Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
# Licensed under the MIT License.

# Requires bindgen: "cargo install bindgen-cli"
# and libclang headers: "sudo apt install libclang-dev"
# https://rust-lang.github.io/rust-bindgen/requirements.html

set -e

cat >src/glue.rs <<'EOF'
#![allow(non_camel_case_types, non_snake_case, unused)]
EOF

exec bindgen \
     --rust-target=1.47 \
     --rustified-enum '.*' \
     src/glue.h \
     -- \
     -x c++ >>src/glue.rs
