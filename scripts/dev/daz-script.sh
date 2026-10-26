#!/bin/sh
# Run a DAZ Script in its own headless DAZ Studio instance and wait:
#   scripts/dev/daz-script.sh <script.dsa> [args...]
# The script gets the arguments, then the main instance's content
# directories, in App.scriptArgs; it should end with App.delayedExit().
# Never alongside an Octane render (Iray / DAZ take the whole CPU).
set -eu
script=$1; shift
app="/Applications/DAZ 3D/DAZStudio6 64-bit/DAZStudio.app/Contents/MacOS/DAZStudio"
if pgrep -q -f "Octane X.app/Contents/MacOS/Octane X"; then
    echo "Octane X is running — quit it first" >&2
    exit 1
fi
for a in "$@"; do set -- "$@" -scriptArg "$a"; shift; done
i=0
while dir=$(defaults read com.daz.Studio6 "ContentDir$i" 2>/dev/null); do
    set -- "$@" -scriptArg "$dir"
    i=$((i + 1))
done
defaults write com.daz.Studio6 NSAppSleepDisabled -bool YES
"$app" -instanceName orbis-render -noPrompt "$@" "$script"
