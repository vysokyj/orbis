#!/bin/sh
# Run an Octane X Lua probe unattended:
#   scripts/dev/octane-run.sh <probe.lua> [scene.orbx]
# Links the scene into Octane's sandbox as Documents/orbis_scene.orbx (the
# probe opens it with octane.project.load), installs the probe as the script
# menu's "ORBIS probe", starts Octane X if it isn't running, clicks the menu
# item (System Events; the terminal needs Accessibility access), waits for
# the probe to write Documents/orbis_probe.done and quits Octane X again —
# killed if it doesn't quit in 30 s. Output lands in the sandbox's Documents.
# Never run it while a DAZ Studio render is going: Iray takes the whole CPU.
set -eu
probe=$1
scene=${2:-}
docs="$HOME/Library/Containers/com.otoy.rndrviewer/Data/Documents"
scripts="$docs/ORBIS Scripts"
bin="Octane X.app/Contents/MacOS/Octane X"
if pgrep -q -f "instanceName orbis-render"; then
    echo "a DAZ Studio render is running — wait for it" >&2
    exit 1
fi
# Octane opens in its simple mode and switches to the standalone one (with
# the Script menu) a few seconds later — not at all behind a locked screen.
if ioreg -n Root -d1 -a | grep -A1 CGSSessionScreenIsLocked | grep -q true; then
    echo "the screen is locked — Octane X may not open its Script menu" >&2
fi
if [ -n "$scene" ]; then
    # A hard link: the sandbox reads it, and a 1 GB export isn't copied.
    ln -f "$scene" "$docs/orbis_scene.orbx"
fi
cp "$probe" "$scripts/ORBIS probe.lua"
rm -f "$docs/orbis_probe.done"
pgrep -q -f "$bin" || open -a "Octane X"
menu() {
    osascript -e "tell application \"System Events\" to tell process \"Octane X\" to $1" 2>/dev/null
}
# Behind a locked screen it stayed in the simple mode for 5+ minutes
# (2026-10-26): give up after 2.
waited=0
until menu 'get name of every menu item of menu 1 of menu bar item "Script" of menu bar 1' | grep -q "ORBIS probe"; do
    sleep 2
    waited=$((waited + 2))
    if [ $waited -ge 120 ]; then
        echo "no Script menu after 2 minutes (a locked screen keeps Octane X in its simple mode)" >&2
        pkill -f "$bin" || true
        exit 1
    fi
done
# It switches modes a few seconds after the menu first shows: let it settle,
# then click, and again if the probe hasn't started its log within 30 s.
sleep 10
started() { [ -n "$(find "$docs" -maxdepth 1 -name 'orbis_probe*.txt' -newer "$scripts/ORBIS probe.lua" 2>/dev/null)" ]; }
tries=0
until started; do
    menu 'click menu item "ORBIS probe" of menu 1 of menu bar item "Script" of menu bar 1' >/dev/null || true
    i=0
    while ! started && [ $i -lt 15 ]; do sleep 2; i=$((i + 1)); done
    tries=$((tries + 1))
    [ $tries -lt 3 ] || { echo "the probe didn't start" >&2; exit 1; }
done
until [ -f "$docs/orbis_probe.done" ]; do sleep 2; done
osascript -e 'tell application "Octane X" to quit' >/dev/null 2>&1 || true
i=0
while pgrep -q -f "$bin" && [ $i -lt 15 ]; do sleep 2; i=$((i + 1)); done
pkill -f "$bin" || true
echo "done: $docs"
