#!/bin/sh
# PreToolUse guard for ORBIS (settings.json). Reads the tool call as JSON on
# stdin; answers with a permission decision where a CLAUDE.md rule needs the
# user's OK. Paths are matched, never hardcoded, so the repo can live anywhere.
input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
ask() {
    jq -n --arg r "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"ask",permissionDecisionReason:$r}}'
    exit 0
}
case $tool in
Bash)
    cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
    case $cmd in
    *git\ push*--force*|*git\ push*\ -f\ *|*git\ push*-f) ask "Force push: the workspace rules say always ask." ;;
    *octane-render.sh*|*octane-run.sh*|*daz-script.sh*|*daz-dump.sh*|*calib-variants.sh*|*calib-octane-batch.sh*|*make-skin-variants.py*|*make-skin-scenes.py*|*daz-export-batch.sh*)
        ask "Render/probe/headless DAZ run: ask the user whether Octane X / DAZ Studio is free (Working agreements: Ask before rendering)." ;;
    *daz-render.sh*)
        ask "Local CPU Iray takes the whole CPU. The user prefers Iray Server; ask first." ;;
    *iray-queue.sh*)
        ask "The Iray Server queue is the user's (paid time). Propose scenes and order, queue only after a yes; never reorder." ;;
    esac ;;
Edit|Write)
    f=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty')
    case $f in
    */src/iray.rs)
        ask "src/iray.rs reproduces DAZ's own .mi export and changes only with new .mi fixtures that prove it (#62). Is that the case here?" ;;
    */calib/scenes/*.duf)
        ask "calib/scenes holds only generated scenes (DAZ built-ins/primitives). Edit via the generator scripts, not by hand." ;;
    esac ;;
esac
exit 0
