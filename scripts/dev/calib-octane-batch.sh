#!/bin/sh
# The whole Octane side of the calibration in one go (needs an unlocked
# screen; never during a DAZ render): every calibration export and the
# hypothesis variants, rendered as exported, then compared with the Iray
# references.
#   scripts/dev/calib-octane-batch.sh [samples]
set -eu
here=$(cd "$(dirname "$0")" && pwd)
calib="$HOME/Library/Caches/orbis/calib"
"$here/octane-render.sh" "$calib/octane" "${1:-400}" "$calib"/orbx/*.orbx "$calib"/orbx/variants/*.orbx
python3 "$here/calib-compare-all.py" "$calib/octane" > "$calib/octane/report.md"
echo "report: $calib/octane/report.md"
