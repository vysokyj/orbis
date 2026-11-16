# Take turns through a named lock (sourced by daz-render.sh, daz-script.sh
# and octane-run.sh). The "render" lock: local CPU Iray (daz-render.sh) takes
# the whole CPU, so Octane (octane-run.sh) never renders beside it. The "daz"
# lock (ORBIS_LOCK_NAME=daz, daz-script.sh): one headless DAZ instance at a
# time. Headless DAZ and Octane may run at once (the user's rule, 2026-11-16).
#   [ORBIS_LOCK_NAME=daz] . "$here/render-lock.sh"; render_lock "<what>"
# waits for the lock (a directory, with "<pid> <what>" in it) and frees it
# when the script exits. A lock whose process is gone is taken over. A script
# run by the holder (ORBIS_RENDER_LOCK=<its pid>) shares its lock.
render_lock_dir="$HOME/Library/Caches/orbis/${ORBIS_LOCK_NAME:-render}.lock"

render_lock() {
    if [ -n "${ORBIS_RENDER_LOCK:-}" ] && [ "$(cut -d' ' -f1 "$render_lock_dir/owner" 2>/dev/null)" = "$ORBIS_RENDER_LOCK" ]; then
        return 0
    fi
    mkdir -p "${render_lock_dir%/*}"
    said=
    until mkdir "$render_lock_dir" 2>/dev/null; do
        holder=$(cat "$render_lock_dir/owner" 2>/dev/null || true)
        render_lock_pid=${holder%% *}
        # Gone, or killed between taking the directory and naming itself.
        if { [ -n "$render_lock_pid" ] && ! kill -0 "$render_lock_pid" 2>/dev/null; } || { [ -z "$render_lock_pid" ] && [ -n "$(find "$render_lock_dir" -maxdepth 0 -mmin +1 2>/dev/null)" ]; }; then
            rm -rf "$render_lock_dir"
            continue
        fi
        [ -n "$said" ] || { echo "waiting for the render lock (${holder:-being taken}): $render_lock_dir" >&2; said=1; }
        sleep 5
    done
    echo "$$ $1" > "$render_lock_dir/owner"
    export ORBIS_RENDER_LOCK=$$
    trap render_unlock EXIT
    trap 'exit 130' INT TERM
}

render_unlock() {
    [ "$(cut -d' ' -f1 "$render_lock_dir/owner" 2>/dev/null)" = "$$" ] && rm -rf "$render_lock_dir"
    return 0
}
