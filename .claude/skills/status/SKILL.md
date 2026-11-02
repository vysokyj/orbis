---
name: status
description: State of ORBIS's long-running processes - Iray Server queue (waiting/active/done/ETA, stalled server), Octane X last job, DAZ/Octane processes, render lock, git/CI, cache size. Use when the user writes "status" or "stav", asks what is running / how far the render is, or before starting any render.
---

# Status

Run `python3 scripts/dev/status.py` (read-only; `--sets` adds the per-set table of the full Iray run when `~/Library/Caches/orbis/calib/sets-status.py` exists) and show it as a compact table in Czech: what runs, what waits, how far, ETA, what is odd.

Add one line for each `!!` warning, nothing else:
- STALLED queue -> the user restarts the queue script in DAZ (we never start or reorder their queue).
- "session handler rejected the session" -> the Iray Server reset its password; the user fixes it, the queue script retries every 30 s.
- Tiny PNGs -> black renders.
- Leftover `.orbx` -> `scripts/dev/clean-cache.sh`.

Run it also before proposing any render (lock free? queue busy? app running?). When the user wrote "status"/"stav" during a long run, keep answering that way until they say stop.
