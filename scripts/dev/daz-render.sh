#!/bin/sh
# Render a DAZ scene with Iray from the command line, capped in time:
#   scripts/dev/daz-render.sh <scene.duf> <out.png> [max seconds] [max samples]
# Renders a copy of the scene (beside the output) with Max Time / Max Samples
# lowered, in its own DAZ Studio instance (App Nap off: in the background it
# crawled, 18 of 36 iterations taking 17 minutes), and waits for it to quit.
set -eu
scene=$1
out=$2
max_time=${3:-120}
max_samples=${4:-2000}
here=$(cd "$(dirname "$0")" && pwd)
app="/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/MacOS/DAZStudio"
copy="${out%.*}.duf"
# One renderer at a time: Iray takes the whole CPU (scripts/dev/octane-run.sh
# quits Octane X when its probe is done).
if pgrep -q -f "Octane X.app/Contents/MacOS/Octane X"; then
    echo "Octane X is running — quit it first" >&2
    exit 1
fi
python3 - "$scene" "$copy" "$max_time" "$max_samples" <<'EOF'
import gzip, json, sys
src, dst, max_time, max_samples = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
raw = open(src, 'rb').read()
d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
def cap(v):
    if isinstance(v, dict):
        if v.get('id') == 'Max Time': v['current_value'] = max_time
        if v.get('id') == 'Max Samples': v['current_value'] = max_samples
        # Stop on the caps alone: with no quality-based stop, a sample cap
        # makes Iray deterministic (tone variants share their noise).
        if v.get('id') == 'Rendering Quality Enable': v['current_value'] = False
        for x in v.values(): cap(x)
    elif isinstance(v, list):
        for x in v: cap(x)
for e in d['scene'].get('extra', []):
    if e.get('type') == 'studio_render_settings':
        cap(e)
open(dst, 'wb').write(gzip.compress(json.dumps(d).encode()))
EOF
# The main instance's content directories (its preferences), in order.
set -- -scriptArg "$copy" -scriptArg "$out"
i=0
while dir=$(defaults read com.daz.Studio6 "ContentDir$i" 2>/dev/null); do
    set -- "$@" -scriptArg "$dir"
    i=$((i + 1))
done
rm -f "$out" "$out.log"
# App Nap off for DAZ Studio (passed as an argument, DAZ takes it for a scene).
defaults write com.daz.Studio6 NSAppSleepDisabled -bool YES
# In the background (open -g): a new instance's window would take the focus
# from whatever the user is doing. open returns at once; follow the instance
# by its name.
open -g -n -a "${app%/Contents/MacOS/DAZStudio}" --args -instanceName orbis-render -noPrompt "$@" "$here/daz-render.dsa"
pid=
for _ in $(seq 60); do
    pid=$(pgrep -n -f "DAZStudio -instanceName orbis-render" || true)
    [ -n "$pid" ] && break
    sleep 1
done
# Kill it once the render is saved: quitting cleans up for minutes.
while [ -n "$pid" ] && kill -0 $pid 2>/dev/null && ! grep -q '^done$' "$out.log" 2>/dev/null; do sleep 1; done
[ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
cat "$out.log" 2>/dev/null || true
test -f "$out"
