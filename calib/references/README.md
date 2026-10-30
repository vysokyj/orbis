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

`iray/renders.tsv` records how each kept render was made: samples, time cap
(0 = none), renderer (`DAZ Studio Iray (CPU)` through `calib-render.py`, or
`Iray Server (Boost for Daz)` through `iray-queue.sh`), date and render time.
Iray capped by samples with no time or quality stop runs exactly that many
iterations. `scripts/dev/keep-refs.py <render.png>...` copies renders in and
updates it; it takes only scenes that are in `calib/scenes` and never
replaces a render with one of fewer samples. Keep them as they come — commit
during a long run, not at its end.

Since 2026-10-30 every generated scene's reference is an Iray Server render
at 5000 samples except Calib-Tone-Gladiator (400, CPU). The 400-sample CPU
renders they replaced are kept in `~/Library/Caches/orbis/calib/iray-400`.
