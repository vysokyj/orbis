#!/bin/sh
# Delete the .orbx exports in the cache (they filled 107 GB); keeps Iray
# references, dumps, PNG renders and reports (re-export from cached dumps).
#   scripts/dev/clean-cache.sh              # all of ~/Library/Caches/orbis
#   scripts/dev/clean-cache.sh calib/s79    # only under that cache subdir
#   scripts/dev/clean-cache.sh -n [dir]     # dry run: count, delete nothing
# The only sanctioned way to delete .orbx: it checks the target is inside the
# cache (an empty or escaping path would otherwise delete too much) and
# always uses find, so no shell glob with a variable is needed.
case ${1:-} in -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
dry=0
if [ "${1:-}" = "-n" ]; then dry=1; shift; fi
cache="$HOME/Library/Caches/orbis"
sub=${1:-}
case $sub in
  "") target=$cache ;;
  /*) target=$sub ;;
  *)  target=$cache/$sub ;;
esac
case $target in
  *..*) echo "refusing path with '..': $target" >&2; exit 1 ;;
  "$cache"|"$cache"/*) ;;
  *) echo "refusing path outside $cache: $target" >&2; exit 1 ;;
esac
if [ ! -d "$target" ]; then echo "no such directory: $target (0 removed)"; exit 0; fi
n=$(find "$target" -name '*.orbx' -type f 2>/dev/null | wc -l | tr -d ' ')
if [ "$dry" = 1 ]; then echo "$n .orbx exports under $target (dry run)"; exit 0; fi
find "$target" -name '*.orbx' -type f -delete 2>/dev/null
echo "removed $n .orbx exports under $target"
