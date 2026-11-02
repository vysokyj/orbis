#!/bin/sh
# PreToolUse guard for ORBIS (settings.json). Reads the tool call as JSON on
# stdin; answers with a permission decision where a CLAUDE.md rule needs the
# user's OK. Paths are matched, never hardcoded, so the repo can live anywhere.
input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
session=$(printf '%s' "$input" | jq -r '.session_id // "none"')
# Render/probe/headless-DAZ scripts ask once per session: the marker is written
# by `guard.sh post` (PostToolUse) after the user let the first one run.
marker="$HOME/Library/Caches/orbis/guard/$session.render"
is_render() {
    case $1 in
    *octane-render.sh*|*octane-run.sh*|*daz-script.sh*|*daz-dump.sh*|*calib-variants.sh*|*calib-octane-batch.sh*|*make-skin-variants.py*|*make-skin-scenes.py*|*daz-export-batch.sh*) return 0 ;;
    esac
    return 1
}
if [ "${1:-}" = post ]; then
    if [ "$tool" = Bash ]; then
        cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
        if is_render "$cmd"; then mkdir -p "${marker%/*}" && : > "$marker"; fi
    fi
    exit 0
fi
allow() {
    jq -n --arg r "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"allow",permissionDecisionReason:$r}}'
    exit 0
}
deny() {
    jq -n --arg r "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"deny",permissionDecisionReason:$r}}'
    exit 0
}
ask() {
    jq -n --arg r "$1" '{hookSpecificOutput:{hookEventName:"PreToolUse",permissionDecision:"ask",permissionDecisionReason:$r}}'
    exit 0
}
case $tool in
Bash)
    cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
    if is_render "$cmd"; then
        [ -f "$marker" ] && allow "Render/probe run already confirmed in this session."
        ask "Render/probe/headless DAZ run: ask the user whether Octane X / DAZ Studio is free (Working agreements: Ask before rendering). Asked once per session."
    fi
    case $cmd in
    *git\ commit*)
        # Scenes and DAZ content never enter git, except the generated calib/scenes;
        # demo/ keeps only the manifest, the README and the reference renders.
        staged=$(cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null && git diff --cached --name-only 2>/dev/null)
        bad=$(printf '%s\n' "$staged" | grep -E '\.(duf|dsf|dsa|dbz|mi)$|(^|/)Demo-.*\.(duf|mi|usdz|orbx)$' | grep -vE '^(calib/scenes/|demo/scenes/Demo-[^/]+\.duf$|calib/references/mi/|scripts/|daz/Scripts/)' | head -3)
        [ -z "$bad" ] || deny "Staged DAZ scene/content files outside calib/scenes (store content must not enter this public repo): $bad"
        bad=$(printf '%s\n' "$staged" | grep -E '^demo/' | grep -vE '^demo/(README\.md|scenes\.toml|scenes/Demo-[^/]+\.duf|references/iray/(renders\.tsv|[^/]+\.png))$' | head -3)
        [ -z "$bad" ] || deny "demo/ may hold only README, scenes.toml, scenes/Demo-*.duf and reference PNGs + renders.tsv: $bad" ;;
    *git\ push*--force*|*git\ push*\ -f\ *|*git\ push*-f) ask "Force push: the workspace rules say always ask." ;;
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
    */Demo-*.duf)
        deny "Demo- scenes are hand-made by the user and read-only for scripts and Claude." ;;
    */calib/scenes/*.duf)
        ask "calib/scenes holds only generated scenes (DAZ built-ins/primitives). Edit via the generator scripts, not by hand." ;;
    esac ;;
esac
exit 0
