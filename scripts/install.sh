#!/usr/bin/env bash
# Build and install the DAZ Studio plugin into your DAZ library ("My Library"
# from DAZ Studio's content directories, or the one given).
# Usage: scripts/install.sh [--library DIR]
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --bin orbis
target/release/orbis install "$@"
