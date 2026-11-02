---
name: octane-probe
description: Write and run Octane X Lua probes unattended. Use when an Octane behaviour must be measured (node values, render of a small scene) rather than guessed.
---

# Octane X probes

- Run: `scripts/dev/octane-run.sh <probe.lua> [scene.orbx]` (**ask the user first**; it takes the render lock, starts Octane X if needed and leaves it running; `QUIT_OCTANE=1` quits it). Templates: `scripts/dev/octane-probe*.lua`, `octane-render.lua`.
- Batch renders of exports as exported: `scripts/dev/octane-render.sh <out dir> [samples] <scene.orbx>...`.
- Octane X is **sandboxed**: it cannot open files from outside, you cannot launch it with a file. The script copies the probe to `~/Library/Containers/com.otoy.rndrviewer/Data/Documents/ORBIS Scripts/ORBIS probe.lua`. `os.getenv("HOME")` inside is the container, so probe logs/PNGs land in `~/Library/Containers/com.otoy.rndrviewer/Data/Documents/`.
- Needs an unlocked screen and Accessibility access for the terminal (System Events click).
- Failed probe/render: leave Octane open (error dialog, #50). No DAZ render beside it (shared lock `scripts/dev/render-lock.sh`; do not wait on `pgrep`).
- Octane manual mirror: `docs/octane/manual` (git-ignored; fetch with `fetch-octane-docs.py`). Read it before guessing.
- Clean up: `scripts/dev/clean-cache.sh` after a loop.
