#!/bin/sh
# Run a DAZ Script in its own headless DAZ Studio instance and wait:
#   [DONE_FILE=<file>] scripts/dev/daz-script.sh <script.dsa> [args...]
# The script gets the arguments, then the main instance's content
# directories, in App.scriptArgs; it should end with App.delayedExit().
# With DONE_FILE the instance is killed as soon as that file appears:
# quitting a big scene took DAZ 2 minutes of cleanup (intake.duf).
# Never alongside an Octane render (Iray / DAZ take the whole CPU).
# The user's custom actions (customactions.dsx) are put back after the run;
# KEEP_ACTIONS=0 keeps what the script changed there.
case ${1:-} in -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
set -eu
# Absolute: DAZ Studio starts through `open`, in another directory.
script=$(cd "$(dirname "$1")" && pwd)/$(basename "$1"); shift
app="/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/MacOS/DAZStudio"
# One DAZ instance or render at a time (Octane X may stay open).
. "$(dirname "$0")/render-lock.sh"
render_lock "daz-script.sh $(basename "$script")"
for a in "$@"; do set -- "$@" -scriptArg "$a"; shift; done
i=0
while dir=$(defaults read com.daz.Studio6 "ContentDir$i" 2>/dev/null); do
    set -- "$@" -scriptArg "$dir"
    i=$((i + 1))
done
defaults write com.daz.Studio6 NSAppSleepDisabled -bool YES
# -headless: no window (the same flag Dazzle uses; #63). HEADLESS=0 starts a
# normal instance for a script that needs the GUI. -noDefaultScene skips the
# default scene's load.
flags="-instanceName orbis-render -noPrompt"
[ "${HEADLESS:-1}" = 0 ] || flags="-headless -noDefaultScene $flags"
# Custom actions (the user's Render > ORBIS Iray Queue, File > Export to
# Octane...) live in one file for every instance: -instanceName gives this
# one its own menus.dsx, but it loads and, quitting with a window, saves the
# user's Studio6/customactions.dsx. A script that changes its in-memory list
# changes the user's menus (#181: the #170 menutest emptied it). So the file
# is put back as it was after the run. KEEP_ACTIONS=0 lets a script's change
# stay (daz-add-queue-menu.dsa, run to change it).
actions="$HOME/Library/Application Support/DAZ 3D/Studio6/customactions.dsx"
actions_copy=
if [ "${KEEP_ACTIONS:-1}" != 0 ] && [ -f "$actions" ]; then
    actions_copy=$(mktemp "${TMPDIR:-/tmp}/orbis-customactions.XXXXXX")
    cp -p "$actions" "$actions_copy"
fi
restore_actions() {
    [ -n "$actions_copy" ] || return 0
    if ! cmp -s "$actions" "$actions_copy"; then
        # Contested: restoring also undoes a save by the user's own DAZ Studio
        # if they quit it during this run - rare (the render lock keeps our
        # runs apart, not the user's), and a DAZ still open writes its own
        # list again when it quits, so its menus win anyway.
        cp -p "$actions_copy" "$actions"
        echo "daz-script.sh: put customactions.dsx back as it was before the run (KEEP_ACTIONS=0 keeps a change)" >&2
    fi
    rm -f "$actions_copy"
}
trap 'restore_actions; render_unlock' EXIT
if [ -z "${DONE_FILE:-}" ]; then
    # Not exec: the restore above runs after DAZ quits.
    "$app" $flags "$@" "$script"
    exit
fi
rm -f "$DONE_FILE"
# In the background, started directly (not through open): a headless instance
# has no window to take the focus from whatever the user is doing.
"$app" $flags "$@" "$script" >/dev/null 2>&1 &
pid=$!
while [ -n "$pid" ] && kill -0 $pid 2>/dev/null && [ ! -f "$DONE_FILE" ]; do sleep 1; done
[ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
