#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
    echo "usage: tools/tcl-diff.sh CASE.tcl" >&2
    exit 2
fi

case_file=$1
tclsh_bin=${TCLSH:-tclsh8.6.18}
if ! command -v "$tclsh_bin" >/dev/null 2>&1; then
    echo "Tcl 8.6.18 shell not found; set TCLSH=/path/to/tclsh8.6.18" >&2
    exit 2
fi

case_source=$(sed -e '$a\' "$case_file")
scratch=$(mktemp -d "${TMPDIR:-/tmp}/molt-tcl-diff.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

MOLT_DIFF_SCRIPT=$case_source "$tclsh_bin" tools/tcl-diff-driver.tcl >"$scratch/tcl.out"
MOLT_DIFF_SCRIPT=$case_source cargo run --quiet -p molt-app -- shell tools/tcl-diff-driver.tcl \
    >"$scratch/molt.out"

if cmp -s "$scratch/tcl.out" "$scratch/molt.out"; then
    echo "match"
else
    diff -u "$scratch/tcl.out" "$scratch/molt.out"
    exit 1
fi
