#!/usr/bin/env bash
# Build the release zip into dist/ (git-ignored):
#
#   dist/orbis-<version>-macos.zip
#     orbis            (universal when both targets are installed)
#     README.md, LICENSE, docs/
#
# The binary carries the DAZ Studio script and icons; `./orbis install` sets
# up the plugin. Usage: scripts/package.sh
case ${1:-} in -h|--help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"orbis","version":"\([^"]*\)".*/\1/p')
name="orbis-${version}-macos"
stage="dist/${name}"

rm -rf "${stage}" "dist/${name}.zip"
mkdir -p "${stage}"

targets=()
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  if rustup target list --installed 2>/dev/null | grep -qx "$t"; then targets+=("$t"); fi
done
if [ ${#targets[@]} -eq 0 ]; then
  cargo build --release --bin orbis
  cp "$(scripts/dev/target-dir.sh)/release/orbis" "${stage}/orbis"
else
  bins=()
  for t in "${targets[@]}"; do
    cargo build --release --bin orbis --target "$t"
    bins+=("target/$t/release/orbis")
  done
  lipo -create -output "${stage}/orbis" "${bins[@]}"
fi
# Developer ID with a secure timestamp when the keychain has one, else ad hoc (#3).
scripts/sign.sh --release "${stage}/orbis"

cp README.md LICENSE "${stage}/"
mkdir -p "${stage}/docs"
rsync -a --exclude manual/ docs/ "${stage}/docs/"

(cd dist && ditto -c -k --norsrc --noextattr --keepParent "${name}" "${name}.zip")
lipo -info "${stage}/orbis"
echo "dist/${name}.zip"
