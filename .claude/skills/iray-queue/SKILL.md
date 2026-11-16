---
name: iray-queue
description: Queue Iray renders on the home Iray Server (free, 4 GB VRAM) through the ORBIS render queue - generic Calib scenes at Claude's discretion, Demo/private scenes only after asking. Use when Iray references are needed, or the queue seems stalled.
---

# Iray Server queue (`scripts/dev/iray-queue.sh`)

## Home Iray Server (the user's rule, 2026-11-16, until they say otherwise)
The Iray Server is now a home machine: **free, but its GPU has only 4 GB of VRAM**. The old rules for a paid server (one big batch, one call, parked until the user starts it) are gone.
- **Generic scenes go there on Claude's own judgment, no question**: everything in `calib/scenes` and freshly generated `Calib-*` scenes (DAZ built-ins, primitives, free DAZ add-ons). Send what the work needs when it needs it; small batches are fine.
- **Ask first** for `Demo-*` and private scenes (store content, the teens scene, big figures with 4K maps): they are what 4 GB may not hold.
- **4 GB VRAM**: keep scenes small (primitives, few and small textures). A render that takes far longer than its siblings, a black / tiny PNG, or an out-of-memory / fallback line in DAZ's log (`~/Library/Application Support/DAZ 3D/Studio6/log.txt`) = the scene did not fit: don't resend it, tell the user, make it smaller.
- Still never local CPU Iray (`daz-render.sh`) without asking.
- The queue-side DAZ and its queue script are the user's to start; we never start them. Without them the scenes simply wait in the queue folder.

## Queueing
1. Prepared scenes: generated (`make-*.py`, `daz-build-*.dsa`; DAZ built-ins/primitives or free DAZ add-ons only for calib/scenes), checked that they open, camera set (below), NVIDIA presets listed (below).
2. `scripts/dev/iray-queue.sh [-n] [-s samples] [-o out dir] <scene.duf>...` (default 400; references use 5000), one call per sample count. Without `-n` it waits for the renders and moves them to the out dir - run it as a background command and block with `scripts/dev/wait-for.sh` if you must; `-n` returns at once and `--collect` moves finished renders later. Queueing into a running, idle queue froze DAZ once (#170, fixed); if DAZ hangs on a new scene, tell the user.
3. A scene that needs a queue-side DAZ restart before it (#94, below) is **held back**: tell the user, and queue it when they say they restart (before the new queue script starts) with `iray-queue.sh -n --held ...`: it refuses (nothing queued) while the queue-side DAZ still runs (`pgrep -x DAZStudio`, the `orbis-render` helper excluded) - a running queue would grab it at once, in the old session, and on 2026-11-16 one rendered it on the CPU (#183). Refused: wait for the user to quit DAZ, never work around it.
4. **While it runs, work on Octane** (Octane and DAZ may run at once; never beside a local CPU Iray render) and on anything else; check `progress` instead of polling.
5. Never reorder (no backdated mtimes); another session's scenes go first. `--stop` only on request.
6. Finished renders -> `iray-refs` skill.

## Inside the issue loop
A worker queues its own generic scenes and waits for them when the queue runs (`status.py`: queue script started, log fresh, no lost connection); without a running queue it parks the issue (`needs-iray`) as described in the `issue-loop` skill and the orchestrating session queues the batch.

## One NVIDIA preset per DAZ session
DAZ imports an MDL module into Iray once per session. A later scene in the same queue session using the same NVIDIA preset (concrete, porcelain, granite, ...) logs `MDL Import Error (4004)` and Iray renders that material as an untextured fallback (#94). Before a batch: list the presets each scene uses; a repeat (within the batch, or of a scene the running queue DAZ already rendered) needs the user to restart the queue-side DAZ in between - hold it back and tell them. `keep-refs.py` refuses such renders, `status.py` flags them (`scripts/dev/daz_log.py`).

## The scene's camera
DAZ renders through the viewport's camera. A scene saved through the Perspective View has no `current_camera`, and the queue then renders DAZ's own view, not the camera `orbis` exports (#97: four references were unusable). `iray-queue.sh` refuses such a batch, `keep-refs.py` such a render; `python3 scripts/dev/duf_camera.py --fix <scene.duf>` (or `duf_camera.ensure` in a generator) sets it.

## Never on the CPU (#183)
When the Iray Server stops while the queue-side DAZ stays open, DAZ logs "Received error from server on cluster manager connection" and drops the connection without a word; the next render runs "Initializing local rendering" on this Mac's CPU. No script API shows the connection (probed), so the guards go by DAZ's log:
- `daz-render-queue.dsa`, before each scene: a lost connection in DAZ's log since this DAZ session / the queue's last stop / the last good render -> the queue **ends** (`queue.log`: `stopped: DAZ lost its Iray Server connection ...`, a dialog in DAZ), the scenes stay queued; the user reconnects and starts the queue again. The server not answering (`nc` to its address) -> it waits, nothing renders. A render that ran locally anyway -> image discarded, scene back in the queue, the queue ends.
- `iray-queue.sh` refuses any batch while the queue-side DAZ runs a queue and its log says the connection is lost (`python3 scripts/dev/daz_log.py --bridge`); `status.py` flags it. Tell the user; don't queue until they restart DAZ or reconnect.
- Residual hole: a DAZ that never connected (no bridge event this session) renders its first scene on the CPU before the verdict ends the queue - the user connects before starting the queue.

## Stalls
- `progress` shows `STALLED` or the DAZ log (`~/Library/Application Support/DAZ 3D/Studio6/log.txt`) shows "Server refused connection: session handler rejected the session" = the server reset its password. **Tell the user**; they fix it and restart the queue script in DAZ (it retries every 30 s). We never start the queue-side DAZ.
- `iray-queue.sh` queues even when DAZ Studio or the queue script isn't running (the scenes wait), and notes a rejected session in the DAZ log - tell the user. It refuses only scenes without a camera and a stale menu path.
Principles: `docs/calibration.md`.
