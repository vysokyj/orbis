#!/bin/sh
# Run a DAZ Script in its own headless DAZ Studio instance and wait:
#   [DONE_FILE=<file>] scripts/dev/daz-script.sh <script.dsa> [args...]
# The script gets the arguments, then the main instance's content
# directories, in App.scriptArgs; it should end with App.delayedExit().
# With DONE_FILE the instance is killed as soon as that file appears:
# quitting a big scene took DAZ 2 minutes of cleanup (intake.duf).
# Never alongside an Octane render (Iray / DAZ take the whole CPU).
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
if [ -z "${DONE_FILE:-}" ]; then
    exec "$app" $flags "$@" "$script"
fi
rm -f "$DONE_FILE"
# In the background, started directly (not through open): a headless instance
# has no window to take the focus from whatever the user is doing.
"$app" $flags "$@" "$script" >/dev/null 2>&1 &
pid=$!
while [ -n "$pid" ] && kill -0 $pid 2>/dev/null && [ ! -f "$DONE_FILE" ]; do sleep 1; done
[ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
