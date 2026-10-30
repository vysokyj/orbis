# Iray references

`iray/<scene>.png`: DAZ Studio's Iray render of each generated scene in
`calib/scenes` — the references the calibration measures Octane against
(`scripts/dev/calib-compare-all.py` reads them from
`~/Library/Caches/orbis/calib/iray`; copy them there to compare without
rendering in DAZ).

Rendered by `scripts/dev/calib-render.py` (headless DAZ Studio, Iray capped by
samples so every variant has the same noise; `--auto-ev` set the scene's
exposure, saved in the scene). Only the generated scenes' renders belong
here — never a render of a user's own scene or of DAZ Store content.

Calib-DualLobe2 and Calib-SSSColor were rendered on an Iray Server at 5000
samples (`scripts/dev/iray-queue.sh -s 5000`); the others are being
re-rendered that way (2026-10-30).
