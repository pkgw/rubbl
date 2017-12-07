#! /bin/sh
# Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
# Licensed under the MIT License.

set -e

cat >src/glue.rs <<'EOF'
#![allow(non_camel_case_types, non_snake_case, unused)]
EOF

exec bindgen \
     --rust-target=1.19 \
     src/glue.h \
     -- \
     -x c++ >>src/glue.rs
