#! /bin/sh
# Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
# Licensed under the MIT License.

exec bindgen \
     --rust-target=1.19 \
     -o src/glue.rs \
     src/glue.h \
     -- \
     -x c++
