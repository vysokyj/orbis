#!/bin/sh
# Delete the .orbx exports in the cache (they filled 107 GB); keeps Iray
# references, dumps, PNG renders and reports (re-export from cached dumps).
#   scripts/dev/clean-cache.sh
case ${1:-} in -h|--help) sed -n '2,4p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;; esac
cache="$HOME/Library/Caches/orbis"
n=$(find "$cache" -name '*.orbx' 2>/dev/null | wc -l | tr -d ' ')
find "$cache" -name '*.orbx' -delete 2>/dev/null
echo "removed $n .orbx exports from $cache"
