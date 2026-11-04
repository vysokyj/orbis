#!/bin/sh
# Render scenes through the render queue of a running DAZ Studio
# (daz-render-queue.dsa, there connected to an Iray Server), capped by samples:
#   scripts/dev/iray-queue.sh [-s samples] [-o out dir] <scene.duf>...
#   scripts/dev/iray-queue.sh --stop
# Each scene goes into ~/Library/Caches/orbis/calib/queue as a copy with Max
# Samples set and no time or quality stop (Boost for Daz: Max Time 0, or
# the streamed render is cut off); its render lands in <out dir>/<scene>.png
# (default ~/Library/Caches/orbis/calib/iray, beside the CPU references).
case ${1:-} in -h|--help) sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
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
while [ $# -gt 0 ]; do
    case $1 in
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
if ! grep -q started "$queue/queue.log" 2>/dev/null; then
    echo "no queue running: open daz-render-queue.dsa in the connected DAZ Studio" >&2
    exit 1
fi
# A "started" line outlives the DAZ that wrote it: the queue only runs while
# the connected DAZ Studio does. Tell the user instead of filling a dead queue.
# The headless helper instance (orbis-render) is not the connected DAZ.
if ! pgrep -fl "DAZStudio" | grep -qv "instanceName orbis-render"; then
    echo "the queue script was started, but DAZ Studio is not running: ask the user to start DAZ and the Iray Server queue" >&2
    exit 1
fi
if tail -n 300 "$HOME/Library/Application Support/DAZ 3D/Studio6/log.txt" 2>/dev/null | grep -q "session handler rejected the session"; then
    echo "the Iray Server rejects the session (it reset its password): tell the user, they fix it and restart the queue script" >&2
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
    # Whole, then visible to the queue.
    mv "$queue/$name.duf.part" "$queue/$name.duf"
    sleep 1   # distinct modification times: the queue takes the oldest first
done
# All queued; collect the renders as they come.
for scene in "$@"; do
    name=$(basename "$scene" .duf)
    start=$(date +%s)
    until [ -f "$queue/done/$name.duf" ]; do sleep 2; done
    if [ -f "$queue/$name.png" ]; then
        mv "$queue/$name.png" "$out/$name.png"
        echo "$name: $(( $(date +%s) - start )) s"
    else
        echo "$name: FAILED (see $queue/queue.log)"
    fi
done
