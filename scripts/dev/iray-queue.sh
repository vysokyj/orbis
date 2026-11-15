#!/bin/sh
# Render scenes through the render queue of DAZ Studio (daz-render-queue.dsa,
# there connected to an Iray Server), capped by samples:
#   scripts/dev/iray-queue.sh [-n] [-s samples] [-o out dir] <scene.duf>...
#   scripts/dev/iray-queue.sh --collect
#   scripts/dev/iray-queue.sh --stop
# Each scene goes into ~/Library/Caches/orbis/calib/queue as a copy with Max
# Samples set and no time or quality stop (Boost for Daz: Max Time 0, or
# the streamed render is cut off); its render lands in <out dir>/<scene>.png
# (default ~/Library/Caches/orbis/calib/iray, beside the CPU references).
# Queue a batch BEFORE the user starts the queue script; without DAZ or a
# started queue the scenes wait; -n returns at once, --collect moves finished
# renders. A running queue picks a new scene up within a second.
case ${1:-} in -h|--help) sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -eu
queue="$HOME/Library/Caches/orbis/calib/queue"
out="$HOME/Library/Caches/orbis/calib/iray"
samples=400
mkdir -p "$queue"
if [ "${1:-}" = --stop ]; then
    : > "$queue/stop"
    echo "asked the queue to stop"
    exit 0
fi
# Each queued scene keeps its out dir in <queue>/<scene>.out (the queue
# script only reads *.duf).
collect() {
    for job in "$queue"/done/*.duf; do
        [ -f "$job" ] || continue
        name=$(basename "$job" .duf)
        [ -f "$queue/$name.out" ] || continue
        dir=$(cat "$queue/$name.out")
        if [ -f "$queue/$name.png" ]; then
            mkdir -p "$dir"
            mv "$queue/$name.png" "$dir/$name.png"
            echo "$name: $dir/$name.png"
        else
            echo "$name: FAILED (see $queue/queue.log)"
        fi
        rm -f "$queue/$name.out"
    done
}
if [ "${1:-}" = --collect ]; then
    collect
    exit 0
fi
wait=1
while [ $# -gt 0 ]; do
    case $1 in
        -n) wait=0; shift ;;
        -s) samples=$2; shift 2 ;;
        -o) out=$2; shift 2 ;;
        *) break ;;
    esac
done
mkdir -p "$out"
# DAZ keeps the Render > ORBIS Iray Queue item's script path as it was added;
# a moved repo leaves it on a stale copy (or a symlink someone made for it).
own="$(cd "$(dirname "$0")" && pwd -P)/daz-render-queue.dsa"
menu=$(sed -n 's/.*File="\([^"]*\)".*Text="ORBIS Iray Queue".*/\1/p' "$HOME/Library/Application Support/DAZ 3D/Studio6/customactions.dsx" 2>/dev/null)
if [ -n "$menu" ] && [ "$menu" != "$own" ]; then
    echo "DAZ's ORBIS Iray Queue menu runs $menu, not $own: quit DAZ Studio, then run daz-add-queue-menu.dsa with that path" >&2
    exit 1
fi
# The batch goes in before the user starts the queue (the user's rule), so no
# running DAZ or queue is fine here - the scenes wait for the queue script's
# start. The "idle queue" freeze (#170) was a second click on the menu item
# while the script ran: DAZ deadlocks re-running a running custom action.
# The headless helper instance (orbis-render) is not the connected DAZ.
if pgrep -fl "DAZStudio" | grep -qv "instanceName orbis-render"; then
    busy=$(ls "$queue"/*.duf "$queue"/active/*.duf 2>/dev/null | head -1)
    if [ -z "$busy" ] && grep -q started "$queue/queue.log" 2>/dev/null; then
        echo "note: DAZ runs and its queue may be idle; it should pick the scenes up within a second. If nothing starts, the user must NOT click ORBIS Iray Queue again while it runs (DAZ deadlocks, #170): check queue.log, restart DAZ if needed" >&2
    fi
    if tail -n 300 "$HOME/Library/Application Support/DAZ 3D/Studio6/log.txt" 2>/dev/null | grep -q "session handler rejected the session"; then
        echo "note: the Iray Server rejected the session (it reset its password): tell the user, they fix it and restart the queue script" >&2
    fi
else
    echo "queued for later: DAZ Studio is not running; the scenes render once the user starts DAZ, the Iray Server and the queue script" >&2
fi
# A scene without current_camera renders DAZ's Perspective View, not the
# camera orbis exports (#97): refuse the whole batch, nothing queued.
if ! python3 "$(dirname "$0")/duf_camera.py" "$@" >&2; then
    echo "fix the scenes first (python3 scripts/dev/duf_camera.py --fix, or the generator), then queue" >&2
    exit 1
fi
for scene in "$@"; do
    name=$(basename "$scene" .duf)
    rm -f "$queue/$name.png" "$queue/done/$name.duf"
    python3 - "$scene" "$queue/$name.duf.part" "$samples" <<'EOF'
import gzip, json, sys
src, dst, samples = sys.argv[1], sys.argv[2], int(sys.argv[3])
raw = open(src, 'rb').read()
d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)
def cap(v):
    if isinstance(v, dict):
        if v.get('id') == 'Max Time': v['current_value'] = 0
        if v.get('id') == 'Max Samples': v['current_value'] = samples
        if v.get('id') == 'Rendering Quality Enable': v['current_value'] = False
        for x in v.values(): cap(x)
    elif isinstance(v, list):
        for x in v: cap(x)
for e in d['scene'].get('extra', []):
    if e.get('type') == 'studio_render_settings':
        cap(e)
open(dst, 'wb').write(gzip.compress(json.dumps(d).encode()))
EOF
    echo "$out" > "$queue/$name.out"
    # Whole, then visible to the queue.
    mv "$queue/$name.duf.part" "$queue/$name.duf"
    sleep 1   # distinct modification times: the queue takes the oldest first
done
echo "queued $# scene(s) at $samples samples"
[ "$wait" = 1 ] || exit 0
# All queued; collect the renders as they come.
for scene in "$@"; do
    name=$(basename "$scene" .duf)
    until [ -f "$queue/done/$name.duf" ]; do sleep 2; done
done
collect
