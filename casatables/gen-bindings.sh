#! /bin/sh
# Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
# Licensed under the MIT License.
#
# Keep the magic "55555" synchronized with src/glue.h.

set -e

cat >src/glue.rs <<'EOF'
#![allow(non_snake_case)]
const STRING_SIZE: usize = include!("casa_string_size.txt");
EOF

exec bindgen \
     --rust-target=1.19 \
     src/glue.h \
     -- \
     -x c++ |sed -e 's/55555usize/STRING_SIZE/g' >>src/glue.rs
