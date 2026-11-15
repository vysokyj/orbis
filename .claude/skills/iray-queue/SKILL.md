---
name: iray-queue
description: Prepare and run Iray Server renders through the ORBIS render queue - big batch first, one call to the user to start the server. Use when Iray references are needed, or the queue seems stalled.
---

# Iray Server queue (`scripts/dev/iray-queue.sh`)

**The queue and the server are the user's** (paid time; another session may own a running batch). Iray renders go **only** to the Iray Server, never local CPU Iray (`daz-render.sh`) without asking. The server's time is the scarce resource, ours is cheap: the user starts the server once, so one call must cover as much work as possible.

## Agreement (2026-11-02)
1. **Prepare the whole batch first.** Before asking, have everything ready: generated scenes (`make-*.py`, `daz-build-*.dsa`; DAZ built-ins/primitives or free DAZ add-ons only for calib/scenes), checked that they open and that their dumps/references exist. Add what will probably be needed next, not only what is needed now. Write the ordered list (scene, samples, why) and an ETA from `python3 scripts/dev/status.py` (avg s/scene).
2. **Queue it first, then one call** (the user's rule, 2026-11-15): `scripts/dev/iray-queue.sh -n [-s samples] [-o out dir] <scene.duf>...` (default 400; references use 5000), one call per sample count, in the batch's order. DAZ, the server and the queue script need not run: the scenes wait in the queue folder. Then: "Batch queued: N scenes, ~X min. Start DAZ, the Iray Server and the queue script." A scene dropped into a running, idle queue froze DAZ until a restart (#170) - never queue into an idle running queue and then ask the user to wait; queue before they start, or while the queue is still rendering.
3. A scene that needs a queue-side DAZ restart before it (#94, below) is **held back**: say so in the call, and queue it when the user says they restart (before the new queue script starts). `iray-queue.sh --collect` moves the finished renders to their out dir (`-n` doesn't wait for them).
4. **While it runs, work on Octane** (Octane may render beside the Iray Server queue; never beside a local CPU Iray render) and on anything else; check `progress` instead of polling.
5. Never reorder (no backdated mtimes). The next batch goes after the running one, or ask. `--stop` only on request.
6. Finished renders -> `iray-refs` skill.

## Parked batches (2026-11-11)
In the `issue-loop` skill step 2 is not a call: the batch is written into the issue as a `## Iray batch` comment under the label `needs-iray`, and work goes on with the next issue. The user's "server runs" then covers every batch parked at that moment; collecting and queueing them is described there.

## One NVIDIA preset per DAZ session
DAZ imports an MDL module into Iray once per session. A later scene in the same queue session using the same NVIDIA preset (concrete, porcelain, granite, ...) logs `MDL Import Error (4004)` and Iray renders that material as an untextured fallback (#94). Before a batch: list the presets each scene uses; a repeat (within the batch, or of a scene the running queue DAZ already rendered) needs the user to restart the queue-side DAZ in between - ask for it in the one call. `keep-refs.py` refuses such renders, `status.py` flags them (`scripts/dev/daz_log.py`).

## The scene's camera
DAZ renders through the viewport's camera. A scene saved through the Perspective View has no `current_camera`, and the queue then renders DAZ's own view, not the camera `orbis` exports (#97: four references were unusable). `iray-queue.sh` refuses such a batch, `keep-refs.py` such a render; `python3 scripts/dev/duf_camera.py --fix <scene.duf>` (or `duf_camera.ensure` in a generator) sets it.

## Stalls
- `progress` shows `STALLED` or the DAZ log (`~/Library/Application Support/DAZ 3D/Studio6/log.txt`) shows "Server refused connection: session handler rejected the session" = the server reset its password. **Tell the user**; they fix it and restart the queue script in DAZ (it retries every 30 s). We never start the queue-side DAZ.
- `iray-queue.sh` queues even when DAZ Studio or the queue script isn't running (the scenes wait), and notes a rejected session in the DAZ log - tell the user. It refuses only scenes without a camera and a stale menu path.
Principles: `docs/calibration.md`.
