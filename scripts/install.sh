#!/usr/bin/env bash
# Build and install the DAZ Studio plugin into your DAZ library ("My Library"
# from DAZ Studio's content directories, or the one given).
# Usage: scripts/install.sh [--library DIR]
case ${1:-} in -h|--help) sed -n '2,4p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --bin orbis
"$(scripts/dev/target-dir.sh)/release/orbis" install "$@"
