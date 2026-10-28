#!/bin/sh
# The installed ORBIS script's whole export, unattended (#1): a headless DAZ
# Studio opens the scene, dumps it and runs orbis through DzProcess as the
# File menu does, with the saved options in place of the dialog:
#   scripts/dev/daz-export-batch.sh <scene.duf> <out dir>
# Writes <out dir>/<scene>.orbx and <out dir>/orbis-batch.txt (ok / error,
# orbis's report). Install first (scripts/install.sh). Never alongside an
# Octane render.
set -eu
scene=$1
out=$2
here=$(cd "$(dirname "$0")" && pwd)
lib=$(defaults read com.daz.Studio6 ContentDir1 2>/dev/null || defaults read com.daz.Studio6 ContentDir0)
script="$lib/Scripts/ORBIS/Export to Octane.dsa"
[ -f "$script" ] || { echo "not installed: $script (scripts/install.sh)" >&2; exit 1; }
mkdir -p "$out"
start=$(date +%s)
DONE_FILE="$out/orbis-batch.txt" "$here/daz-script.sh" "$script" --batch "$scene" "$out" >/dev/null 2>&1 || true
echo "$(basename "$scene" .duf): $(( $(date +%s) - start )) s"
cat "$out/orbis-batch.txt" 2>/dev/null || { echo "no result" >&2; exit 1; }
