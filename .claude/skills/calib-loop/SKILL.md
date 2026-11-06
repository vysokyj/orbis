---
name: calib-loop
description: Run the ORBIS calibration loop - export a calibration scene from its cached dump, render it in Octane X, compare with the Iray reference, report, clean the cache. Use when checking or tuning an Octane-side rule against Iray, or when asked to "calibrate", "render and compare", "run the variants".
---

# Calibration loop (export -> Octane -> compare -> clean)

Principle and scene families: `docs/calibration.md`. Winning rules go in `docs/spec/octane-export.md`.
All paths below are relative to the repo root; scripts find each other via their own location, so the repo can live anywhere.

## Before anything
1. **Ask the user, one short question**: is Octane X / DAZ Studio free? If they say they work in an app, hands off. "It's free, don't ask" lasts for that app for that session only.
2. Octane may render beside the Iray Server queue, **never beside a local CPU Iray render**.
3. Prefer small `Calib-*` scenes or half resolution. Big scenes (teens, 1.2 M faces) in back-to-back full renders froze the Mac.
4. Needs an unlocked screen (Octane X has no Script menu behind a lock).

## Steps
1. `cargo build --release` (scripts find the binary through `scripts/dev/target-dir.sh`, wherever cargo builds).
2. A scene needs a cached dump: `scripts/dev/daz-dump.sh <out dir> <scene.duf>...` (headless DAZ, ask first).
3. One or all variants (Look x mode: enhanced-final, faithful-final, enhanced-preview, faithful-preview). Start with **enhanced-final**:
   `scripts/dev/calib-variants.sh [-s samples] enhanced-final -- <scene>...`
   Whole pass: `scripts/dev/calib-octane-batch.sh`.
   Output: `~/Library/Caches/orbis/calib/{orbx,octane}/<variant>/` + `report.md`.
4. Compare: `scripts/dev/calib-compare.py`, `calib-compare-all.py`; highlights: `highlight-spread.py [--vs <iray.png>] <render.png>...`.
   Hand large result tables to the `calib-analyst` agent so they stay out of the main context.
5. **Octane failed?** Leave Octane X open so the user can read the error dialog (#50). Do not retry in a loop.
6. `scripts/dev/clean-cache.sh` - deletes the `.orbx` exports (they filled 107 GB). Keeps Iray refs, dumps, PNGs, reports.
7. Findings -> `lab-note` skill; winning rule -> spec in the same commit.

## Rules that outrank the loop
- **A real material off? `.mi` first** (`mi-fixture` skill): see what DAZ hands Iray before any Iray batch or MDL theory. DAZ also reads channel visibility (`"visible"`), so generated spheres (hidden channels) can miss what product content triggers (#52).
- `src/iray.rs` (DAZ -> Iray conversion) is verified by `.mi` fixtures. A render that contradicts it points at the **Octane side**; never change `iray.rs` to fit a render (#62).
- Octane over Iray: match light amounts and scene meaning, keep Octane where it is more plausible.
- Read `docs/octane/manual/` (local mirror) and `docs/nvidia/` before guessing; see the `docs-first` skill.
