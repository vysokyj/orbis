#!/bin/sh
# Print cargo's target directory (the repo's own target/, or wherever the
# user's cargo config / CARGO_TARGET_DIR puts it): scripts must not assume
# <repo>/target.
#   target=$(scripts/dev/target-dir.sh)   # then "$target/release/orbis"
case ${1:-} in -h|--help) sed -n '2,5p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
cd "$(dirname "$0")/../.." || exit 1
cargo metadata --no-deps --format-version 1 2>/dev/null | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
