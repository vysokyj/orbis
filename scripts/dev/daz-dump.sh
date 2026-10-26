#!/bin/sh
# Dump scenes in a headless DAZ Studio (the ORBIS script's --dump mode) and
# export each with the fresh dump:
#   scripts/dev/daz-dump.sh <out dir> <scene.duf>...
# Writes <out dir>/<scene>.orbx; the dump becomes the scene's cached one.
# Never alongside an Octane render.
set -eu
out=$1; shift
here=$(cd "$(dirname "$0")" && pwd)
root="$here/../.."
mkdir -p "$out"
tmp=$(mktemp -d)
for scene in "$@"; do
    name=$(basename "$scene" .duf)
    start=$(date +%s)
    DONE_FILE="$tmp/$name.json.done" "$here/daz-script.sh" "$root/daz/Scripts/ORBIS/Export to Octane.dsa" \
        --dump "$scene" "$tmp/$name.json" >/dev/null 2>&1 || true
    if [ "$(cat "$tmp/$name.json.done" 2>/dev/null)" = ok ]; then
        "$root/target/release/orbis" export "$scene" --dump "$tmp/$name.json" -o "$out/$name.orbx" | grep -E "meshes|skipped" || true
        echo "$name: ok in $(( $(date +%s) - start )) s"
    else
        echo "$name: dump FAILED"
    fi
done
rm -rf "$tmp"
