#!/bin/sh
# Render ORBIS exports in Octane X unattended, each as exported:
#   scripts/dev/octane-render.sh <out dir> [samples] <scene.orbx>...
# Writes <out dir>/<scene>.png. Needs an unlocked screen (octane-run.sh).
set -eu
out=$1; shift
samples=400
case ${1:-} in *[!0-9]*|'') ;; *) samples=$1; shift ;; esac
here=$(cd "$(dirname "$0")" && pwd)
docs="$HOME/Library/Containers/com.otoy.rndrviewer/Data/Documents"
mkdir -p "$out"
: > "$docs/orbis_jobs.txt"
# Job files named per run: with the same orbis_job_<i>.orbx every run, the
# first import of a batch now and then came out with the previous run's
# job 0 left over (default-grey spheres, an empty frame; #58).
run=$(date +%s)
i=0
for scene in "$@"; do
    name=$(basename "$scene" .orbx)
    job="orbis_job_${run}_$i"
    ln -f "$scene" "$docs/$job.orbx"
    # The tone LUT, if any, beside it (octane-render.lua loads it by path).
    "$here/../../target/release/examples/orbx_ocs" file "$scene" assets/tone.cube "$docs/$job.cube" 2>/dev/null || true
    printf '%s\t%s\t%s\n' "$job.orbx" "$name" "$samples" >> "$docs/orbis_jobs.txt"
    rm -f "$docs/orbis_job_$name.png"
    i=$((i + 1))
done
"$here/octane-run.sh" "$here/octane-render.lua"
cat "$docs/orbis_probe_render.txt"
for scene in "$@"; do
    name=$(basename "$scene" .orbx)
    [ -f "$docs/orbis_job_$name.png" ] && mv "$docs/orbis_job_$name.png" "$out/$name.png"
done
# Nothing of the run stays in Octane's sandbox.
rm -f "$docs"/orbis_job_*.orbx "$docs"/orbis_job_*.cube
