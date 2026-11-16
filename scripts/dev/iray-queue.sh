#!/bin/sh
# Render scenes through the render queue of DAZ Studio (daz-render-queue.dsa,
# there connected to an Iray Server), capped by samples:
#   scripts/dev/iray-queue.sh [-n] [--held] [-s samples] [-o out dir] <scene.duf>...
#   scripts/dev/iray-queue.sh --collect
#   scripts/dev/iray-queue.sh --stop
# Each scene goes into ~/Library/Caches/orbis/calib/queue as a copy with Max
# Samples set and no time or quality stop (Boost for Daz: Max Time 0, or
# the streamed render is cut off); its render lands in <out dir>/<scene>.png
# (default ~/Library/Caches/orbis/calib/iray, beside the CPU references).
# Queue a batch BEFORE the user starts the queue script; without DAZ or a
# started queue the scenes wait; -n returns at once, --collect moves finished
# renders. A running queue picks a new scene up within a second.
# --held: a held-back scene (#94, needs a fresh queue-side DAZ session):
# refused while that DAZ runs. Any batch is refused while a running queue's
# DAZ has lost its Iray Server connection (#183: it would render on the CPU).
case ${1:-} in -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
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
held=0
while [ $# -gt 0 ]; do
    case $1 in
        -n) wait=0; shift ;;
        --held) held=1; shift ;;
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
# No item at all (#181: a test emptied customactions.dsx) is only a warning:
# the scenes may still wait in the queue, but the user can't start it.
if [ -z "$menu" ]; then
    echo "warning: DAZ has no Render > ORBIS Iray Queue item (customactions.dsx): quit DAZ Studio, then run daz-add-queue-menu.dsa (its header says how)" >&2
fi
# The batch goes in before the user starts the queue (the user's rule), so no
# running DAZ or queue is fine here - the scenes wait for the queue script's
# start. The "idle queue" freeze (#170) was a second click on the menu item
# while the script ran: DAZ deadlocks re-running a running custom action.
# The headless helper instance (orbis-render) is not the connected DAZ; by
# process name, not path (its QtWebEngine helpers live in DAZStudio.app too).
queue_daz=
for pid in $(pgrep -x DAZStudio); do
    ps -o args= -p "$pid" | grep -q "instanceName orbis-render" || queue_daz="$queue_daz $pid"
done
# Held-back scenes (#94: an NVIDIA preset / MDL module the running queue DAZ
# already imported) must wait for a fresh DAZ session; a running queue grabs
# a new scene within a second, in the old one. So --held queues only while
# the queue-side DAZ is down (the user said they restart it), else nothing.
if [ "$held" = 1 ] && [ -n "$queue_daz" ]; then
    echo "refused (--held): the queue-side DAZ Studio runs (pid$queue_daz); a held-back scene would render in its old session (#94). Queue it once the user has quit DAZ, before they start it again" >&2
    exit 1
fi
# A queue script that runs in a DAZ that lost its Iray Server connection
# renders the next scene on this Mac's CPU (#183: 2026-11-16, the server
# stopped, DAZ still open, a held-back scene went in). The queue script now
# checks that itself, but one started before that fix doesn't: refuse while
# its log says it runs and DAZ's log says the connection is gone.
if [ -n "$queue_daz" ] && [ "$(grep -E 'started, watching|  stopped' "$queue/queue.log" 2>/dev/null | tail -1 | grep -c 'started, watching')" = 1 ]; then
    if ! bridge=$(python3 "$(dirname "$0")/daz_log.py" --bridge); then
        echo "refused: the queue-side DAZ Studio runs its queue but lost its Iray Server connection (DAZ log: $(printf '%s' "$bridge" | cut -f2 | cut -c1-24)...); a scene queued now would render on this Mac's CPU. The user quits DAZ (or stops the queue: iray-queue.sh --stop), then queue" >&2
        exit 1
    fi
fi
if [ -n "$queue_daz" ]; then
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
