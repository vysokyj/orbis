---
name: iray-queue
description: Propose and run Iray Server renders through the ORBIS render queue. Use when Iray references are needed, or the queue seems stalled.
---

# Iray Server queue (`scripts/dev/iray-queue.sh`)

**The queue is the user's** (paid server time; another session may own a running batch).
1. Propose scenes and an order. Wait for a yes.
2. Only then: `scripts/dev/iray-queue.sh [-s samples] [-o out dir] <scene.duf>...` (default samples 400; references use 5000).
3. Never reorder, no backdated mtimes. `--stop` asks the queue to stop (only on request).
4. The user starts the queue-side DAZ themselves; Octane may render alongside it.
5. Stalled queue + DAZ log "Server refused connection: session handler rejected the session" = the server reset its password. **Tell the user**; they fix it and restart the queue script (it retries every 30 s). DAZ log: `~/Library/Application Support/DAZ 3D/Studio6/log.txt`.
6. Finished renders -> `iray-refs` skill. Status: `status` skill.
Principles: `docs/calibration.md`.
