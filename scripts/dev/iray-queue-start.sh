#!/bin/sh
# Start, or quit, the queue-side DAZ Studio and its Iray Server queue (#194):
#   scripts/dev/iray-queue-start.sh           # DAZ up, Bridge connected, queue running
#   scripts/dev/iray-queue-start.sh --quit    # queue stopped, DAZ quit
# Start: launches DAZ Studio unless the queue-side one runs, connects the
# Bridge (Render Settings > Bridge: Connect, Rendering Enabled; the server,
# user and password DAZ remembers), checks it is connected (Connect greyed,
# Disconnect live) and only then clicks Render > ORBIS Iray Queue - once:
# a second click while it runs deadlocks DAZ (#170); a greyed item = it
# runs. Quit: refuses while a scene renders (active/ not empty), else stops
# the queue and quits DAZ, answering "don't save" to the scene it had open.
# A #94 restart: --quit, `iray-queue.sh -n --held ...`, then start.
# GUI through System Events (daz-ui.applescript): needs Accessibility for
# the terminal and an unlocked screen. Never touches a DAZ the user works in:
# only the default instance (headless orbis-render instances are skipped).
# Exit 0 = done, 1 = refused / failed (the message says why).
case ${1:-} in -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -u
here=$(cd "$(dirname "$0")" && pwd)
app="/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app"
queue="$HOME/Library/Caches/orbis/calib/queue"
ui() { osascript "$here/daz-ui.applescript" "$@" 2>/dev/null || echo missing; }
menu() { osascript -e "tell application \"System Events\" to tell process \"DAZStudio\" to $1" 2>/dev/null; }
say() { echo "iray-queue-start: $*"; }

# The queue-side (default) instance: DAZStudio without -instanceName.
queue_daz() {
    for p in $(pgrep -x DAZStudio); do
        ps -o args= -p "$p" | grep -q -- '-instanceName' || echo "$p"
    done | head -1
}
queue_item_enabled() { menu 'get enabled of menu item "ORBIS Iray Queue" of menu 1 of menu bar item "Render" of menu bar 1'; }
busy() { ls "$queue"/active/*.duf >/dev/null 2>&1; }

if [ "${1:-}" = --quit ]; then
    pid=$(queue_daz)
    [ -n "$pid" ] || { say "no queue-side DAZ running"; exit 0; }
    if busy; then say "a scene is rendering ($(ls "$queue"/active/*.duf | xargs -n1 basename)); quit after it"; exit 1; fi
    [ "$(queue_item_enabled)" = false ] && "$here/iray-queue.sh" --stop >/dev/null
    # The queue checks for "stop" between scenes, within a second when idle.
    t=0; while [ "$(queue_item_enabled)" = false ] && [ $t -lt 60 ]; do sleep 2; t=$((t + 2)); done
    osascript -e 'tell application "DAZStudio" to quit' >/dev/null 2>&1 &
    t=0
    while kill -0 "$pid" 2>/dev/null; do
        # The open queue scene was replaced in memory: never save it.
        for b in "Don't Save" "Discard" "No"; do
            [ "$(ui click AXButton "$b")" = clicked ] && break
        done
        [ $t -ge 300 ] && { say "DAZ ($pid) still running after 5 min; left as it is"; exit 1; }
        sleep 5; t=$((t + 5))
    done
    say "queue stopped, DAZ quit"
    exit 0
fi

# Start.
pid=$(queue_daz)
if [ -z "$pid" ]; then
    say "starting DAZ Studio"
    open -n -a "$app" || { say "could not launch $app"; exit 1; }
    t=0
    until [ "$(queue_item_enabled)" = true ] || [ "$(queue_item_enabled)" = false ]; do
        [ $t -ge 300 ] && { say "DAZ has no Render > ORBIS Iray Queue after 5 min (screen locked? menu missing: daz-add-queue-menu.dsa)"; exit 1; }
        sleep 5; t=$((t + 5))
    done
    # Let the main window finish building its panes.
    sleep 10
fi
if [ "$(queue_item_enabled)" = false ]; then
    say "the queue already runs in DAZ"
    exit 0
fi

# Bridge: the Render Settings pane's Bridge tab.
if [ "$(ui enabled AXButton Connect)" = missing ]; then
    menu 'click menu item "Render Settings..." of menu 1 of menu bar item "Render" of menu bar 1' >/dev/null
    sleep 3
    ui click AXRadioButton Bridge >/dev/null
    sleep 2
fi
[ "$(ui enabled AXButton Connect)" != missing ] || { say "no Bridge tab found in Render Settings"; exit 1; }
if [ "$(ui enabled AXButton Disconnect)" != true ]; then
    say "connecting the Bridge"
    ui click AXButton Connect >/dev/null
    t=0
    until [ "$(ui enabled AXButton Disconnect)" = true ]; do
        [ $t -ge 120 ] && { say "the Bridge did not connect in 2 min (server down, or its password changed: the user's)"; exit 1; }
        sleep 5; t=$((t + 5))
    done
fi
[ "$(ui value AXCheckBox Enabled)" = 1 ] || ui click AXCheckBox Enabled >/dev/null
[ "$(ui value AXCheckBox Enabled)" = 1 ] || { say "could not tick Rendering Enabled"; exit 1; }
say "Bridge connected, rendering enabled"

# The queue: one click, then its log must say it started.
mark=$(date +%s)
menu 'click menu item "ORBIS Iray Queue" of menu 1 of menu bar item "Render" of menu bar 1' >/dev/null
t=0
until [ "$(queue_item_enabled)" = false ] && [ "$(stat -f %m "$queue/queue.log" 2>/dev/null || echo 0)" -ge "$mark" ]; do
    [ $t -ge 60 ] && { say "the queue did not start (no greyed menu item / queue.log line in 60 s)"; exit 1; }
    sleep 2; t=$((t + 2))
done
tail -n 2 "$queue/queue.log"
say "queue running"
