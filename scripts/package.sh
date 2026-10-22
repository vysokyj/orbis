#!/usr/bin/env bash
# Build the DAZ Studio plugin zip into dist/ (git-ignored):
#
#   dist/orbis-<version>-macos.zip
#     Content/Scripts/ORBIS.png                    (folder thumbnail)
#     Content/Scripts/ORBIS/Export to Octane.dsa   (+ .png thumbnail)
#     Content/Scripts/ORBIS/bin/orbis              (universal when both targets are installed)
#     README.md, LICENSE, docs/
#
# `Content/` is a DAZ content-package root: its contents go into a DAZ
# library. Staged by `orbis install`, like a local install.
# Usage: scripts/package.sh
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*"name":"orbis","version":"\([^"]*\)".*/\1/p')
name="orbis-${version}-macos"
stage="dist/${name}"
plugin="${stage}/Content/Scripts/ORBIS"
bin="dist/orbis"

rm -rf "${stage}" "dist/${name}.zip" "${bin}"
mkdir -p "${stage}"

targets=()
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  if rustup target list --installed 2>/dev/null | grep -qx "$t"; then targets+=("$t"); fi
done
if [ ${#targets[@]} -eq 0 ]; then
  cargo build --release --bin orbis
  cp target/release/orbis "${bin}"
else
  bins=()
  for t in "${targets[@]}"; do
    cargo build --release --bin orbis --target "$t"
    bins+=("target/$t/release/orbis")
  done
  lipo -create -output "${bin}" "${bins[@]}"
fi
# Ad hoc signature until a Developer ID one replaces it (#3).
codesign --force --sign - "${bin}"
"${bin}" install --library "${stage}/Content"
cp README.md LICENSE "${stage}/"
mkdir -p "${stage}/docs"
rsync -a --exclude manual/ docs/ "${stage}/docs/"

(cd dist && ditto -c -k --norsrc --noextattr --keepParent "${name}" "${name}.zip")
lipo -info "${plugin}/bin/orbis"
echo "dist/${name}.zip"
