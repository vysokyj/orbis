#!/bin/sh
# SessionStart: orient a fresh session (stdout goes into the context).
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
echo "ORBIS session start. Project skills: see .claude/skills (calib-loop, ship-fix, lab-note, iray-refs, iray-queue, mi-fixture, octane-probe, daz-script-dev, docs-first, progress)."
git status -sb | head -5
orbx=$(find "$HOME/Library/Caches/orbis" -name '*.orbx' 2>/dev/null | wc -l | tr -d ' ')
[ "$orbx" -gt 0 ] && echo "Cache has $orbx leftover .orbx exports: run scripts/dev/clean-cache.sh after the compare loop."
command -v gh >/dev/null && gh issue list -L 8 2>/dev/null
exit 0
