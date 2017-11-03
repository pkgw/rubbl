#! /bin/sh

exec bindgen \
     --rust-target=1.19 \
     -o src/glue.rs \
     src/glue.h \
     -- \
     -x c++
