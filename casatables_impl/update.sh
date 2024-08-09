#! /bin/bash
#
# Copy pristine casacore source files into our source tree.
#
# The only reason that we need to look at a casacore build directory, as
# opposed to a source directory, is to get the version.h file.
#
# This script has no intelligence to handle things like renamed or deleted
# files. If you're updating to a new casacore version and changes of that
# nature have been made, you'll have to manually copy/move/delete casacore
# source files into place within the rubbl tree.

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
        casacore/casa/version.h)
            cp "$builddir/$path" "$impldir/$path"
            ;;
        *)
            cp "$srcdir/$path" "$impldir/$path"
            ;;
    esac
done
