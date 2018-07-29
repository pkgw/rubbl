#! /bin/bash
# Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
# Licensed under the MIT License.

if [ "$1" = "" ] ; then
    echo >&2 "usage: $0 <path-to-casacore-build-directory>"
    exit 1
fi

impldir=$(dirname $0)
builddir="$1"

cur_branch="$(cd $impldir && git rev-parse --abbrev-ref HEAD)"

if [ "$cur_branch" != vendor-casacore ] ; then
    echo >&2 "error: check out the \"vendor-casacore\" branch before running this script"
    exit 1
fi

if [ ! -f "$builddir/CMakeCache.txt" ] ; then
    echo >&2 "error: build directory \"$builddir\" does not look right: no CMakeCache.txt file"
    exit 1
fi

srcdir="$(grep CMAKE_HOME_DIRECTORY "$builddir/CMakeCache.txt" |sed -e 's/CMAKE_HOME_DIRECTORY:INTERNAL=//')"

(cd "$impldir" && git ls-files) |while read path ; do
    case "$path" in
        casacore/casa/config.h|update.sh)
            ;; # skip
        *)
            cp "$srcdir/$path" "$impldir/$path"
            ;;
    esac
done
