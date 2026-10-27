#!/bin/sh
# The calibration in one or more of the four export variants (Look × render
# mode, src/tuning.rs): export each scene from its cached dump in that
# variant, render it in Octane as exported and compare with Iray.
#   scripts/dev/calib-variants.sh [-s samples] <variant>... -- <scene>...
# <variant>: enhanced-final, faithful-final, enhanced-preview,
# faithful-preview, or "all". <scene>: a name in My Library/Scenes
# (Calib-Glossy) or a .duf path; the scene needs a cached dump
# (scripts/dev/daz-dump.sh). Writes ~/Library/Caches/orbis/calib/
# orbx/<variant>/, octane/<variant>/ and octane/<variant>/report.md.
# Needs an unlocked screen; never during a DAZ render.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root="$here/../.."
calib="$HOME/Library/Caches/orbis/calib"
library=${ORBIS_LIBRARY:-"/Volumes/Data/DAZ 3D/Studio/My Library"}
samples=400
if [ "${1:-}" = -s ]; then samples=$2; shift 2; fi
variants=
while [ $# -gt 0 ] && [ "$1" != -- ]; do
    case $1 in
        all) variants="$variants enhanced-final faithful-final enhanced-preview faithful-preview" ;;
        enhanced-final|faithful-final|enhanced-preview|faithful-preview) variants="$variants $1" ;;
        *) echo "unknown variant: $1" >&2; exit 2 ;;
    esac
    shift
done
[ "${1:-}" = -- ] && shift
[ -n "$variants" ] && [ $# -gt 0 ] || { sed -n '2,11p' "$0" >&2; exit 2; }
for v in $variants; do
    look=${v%-*}
    mode=$([ "${v#*-}" = preview ] && echo --preview || true)
    mkdir -p "$calib/orbx/$v"
    set -f
    for scene in "$@"; do
        case $scene in *.duf) duf=$scene ;; *) duf="$library/Scenes/$scene.duf" ;; esac
        name=$(basename "$duf" .duf)
        "$root/target/release/orbis" export "$duf" -o "$calib/orbx/$v/$name.orbx" --look "$look" $mode >/dev/null 2>&1 \
            || echo "$v $name: export FAILED (no cached dump?)" >&2
    done
    set +f
    "$here/octane-render.sh" "$calib/octane/$v" "$samples" "$calib/orbx/$v"/*.orbx >/dev/null
    python3 "$here/calib-compare-all.py" "$calib/octane/$v" > "$calib/octane/$v/report.md"
    echo "$v: $calib/octane/$v/report.md"
done
