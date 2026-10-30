#!/bin/sh
# Render a DAZ scene with Iray from the command line, capped in time:
#   scripts/dev/daz-render.sh <scene.duf> <out.png> [max seconds] [max samples]
#   scripts/dev/daz-render.sh --stop
# Renders a copy of the scene (beside the output) with Max Time / Max Samples
# lowered in one DAZ Studio instance that stays open between renders: its
# render queue (daz-render-queue.dsa, folder ~/Library/Caches/orbis/calib/
# queue-local) is started on the first render and kept; --stop quits it.
# App Nap is off for DAZ Studio (in the background it crawled, 18 of 36
# iterations taking 17 minutes).
set -eu
here=$(cd "$(dirname "$0")" && pwd)
app="/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/MacOS/DAZStudio"
queue="$HOME/Library/Caches/orbis/calib/queue-local"
instance="DAZStudio -instanceName orbis-render"
mkdir -p "$queue"
if [ "${1:-}" = --stop ]; then
    if pgrep -f "$instance" >/dev/null; then
        : > "$queue/stop"
        echo "asked the render instance to quit"
    fi
    exit 0
fi
scene=$1
out=$2
max_time=${3:-120}
max_samples=${4:-2000}
copy="${out%.*}.duf"
# One renderer at a time (Octane X may stay open): wait for the others.
. "$here/render-lock.sh"
render_lock "daz-render.sh $(basename "$scene")"
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
rm -f "$out" "$out.log"

# The render instance: started once, with the queue folder and the main
# instance's content directories (its preferences, in order) as arguments.
pid=$(pgrep -n -f "$instance" || true)
if [ -n "$pid" ] && [ ! -f "$queue/ready" ]; then
    # An instance from before the queue (or one stuck starting): replace it.
    kill -9 "$pid" 2>/dev/null || true
    pid=
fi
if [ -z "$pid" ]; then
    rm -f "$queue/ready" "$queue/stop" "$queue"/*.duf "$queue"/active/*.duf
    set -- -scriptArg "$queue"
    i=0
    while dir=$(defaults read com.daz.Studio6 "ContentDir$i" 2>/dev/null); do
        set -- "$@" -scriptArg "$dir"
        i=$((i + 1))
    done
    # App Nap off (passed as an argument, DAZ takes it for a scene).
    defaults write com.daz.Studio6 NSAppSleepDisabled -bool YES
    # In the background (open -g): a new instance's window would take the
    # focus from whatever the user is doing.
    open -g -n -a "${app%/Contents/MacOS/DAZStudio}" --args -instanceName orbis-render -noPrompt "$@" "$here/daz-render-queue.dsa"
    for _ in $(seq 180); do
        pid=$(pgrep -n -f "$instance" || true)
        [ -n "$pid" ] && [ -f "$queue/ready" ] && break
        sleep 1
    done
    [ -f "$queue/ready" ] || { echo "the DAZ Studio render instance did not start" >&2; exit 1; }
fi

name=$(basename "$out" .png)
rm -f "$queue/$name.png" "$queue/$name.log" "$queue/done/$name.duf"
cp "$copy" "$queue/$name.duf.part"
mv "$queue/$name.duf.part" "$queue/$name.duf"
while [ ! -f "$queue/done/$name.duf" ]; do
    kill -0 "$pid" 2>/dev/null || { echo "the DAZ Studio render instance quit during $name" >&2; break; }
    sleep 1
done
rm -f "$queue/done/$name.duf"
[ -f "$queue/$name.log" ] && mv "$queue/$name.log" "$out.log"
[ -f "$queue/$name.png" ] && mv "$queue/$name.png" "$out"
cat "$out.log" 2>/dev/null || true
test -f "$out"
