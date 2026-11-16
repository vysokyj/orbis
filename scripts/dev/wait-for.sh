#!/bin/sh
# Block until a file exists (and, with a pattern, contains it), then print
# its last lines. The one sanctioned way for an agent to wait on a render it
# started in the background: an inline `until ...; do sleep; done` or a
# `f=...` variable is a new ad hoc command, so the user gets a permission
# prompt each time; this script is covered by the scripts/dev allow rule.
#   scripts/dev/wait-for.sh <file> [pattern] [timeout_s]
#   scripts/dev/wait-for.sh <bg task .output> 'exited with code'
#   scripts/dev/wait-for.sh --pid <pid> [timeout_s]
# pattern = fixed string (grep -F); timeout_s default 570 (fits a 600 s
# foreground call). Exit 0 = there, 124 = timed out (call it again).
case ${1:-} in -h|--help|'') sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
if [ "$1" = "--pid" ]; then
  pid=$2; timeout=${3:-570}; t=0
  while kill -0 "$pid" 2>/dev/null; do
    [ "$t" -ge "$timeout" ] && { echo "wait-for: pid $pid still running after ${timeout}s"; exit 124; }
    sleep 5; t=$((t + 5))
  done
  echo "wait-for: pid $pid exited"; exit 0
fi
file=$1; pattern=${2:-}; timeout=${3:-570}; t=0
while :; do
  if [ -f "$file" ]; then
    if [ -z "$pattern" ] || grep -qF -- "$pattern" "$file"; then
      tail -n 20 "$file"; exit 0
    fi
  fi
  [ "$t" -ge "$timeout" ] && { echo "wait-for: '$pattern' not in $file after ${timeout}s"; exit 124; }
  sleep 5; t=$((t + 5))
done
