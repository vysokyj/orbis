# 2026-11-11 — The scene's camera in Iray references (#97)

Goal: make Calib-NVIDIA2's Iray reference comparable — it showed the
spheres low and close while every Octane render shows the row in the
middle of the frame (#95 couldn't check its spheres; `calib-compare-all.py`
read 0.12 – 3.3).

## Cause

- The scene source didn't change: `calib/scenes`, the My Library copy and
  the queue's capped copy (`queue/done/`) have the same Camera-1; the
  capped copy differs only in Max Time / Rendering Quality, as
  `iray-queue.sh` sets them.
- The scene has no `scene.current_camera`. DAZ saved it while the
  viewport looked through the Perspective View, and a DAZ that opens it
  keeps its own Perspective View; `daz-render-queue.dsa`'s `doRender`
  renders the active viewport's camera. `orbis` takes the first camera
  node (Camera-1). Already diagnosed on 2026-11-05
  ([note](2026-11-05-nvidia-mdl-direct.md), a fitted view on a scratch
  copy) and `daz-apply-presets.dsa` fixed (d77767d), but the scene was
  never rebuilt and nothing guarded the queue.
- 12 of 253 generated scenes lacked it: Calib-NVIDIA2, -BumpGamma-NVIDIA,
  -CastShadows-Off, -Shell-G9, -Mi-Priority (the last three from
  `daz-build-node-flags.dsa`, the second from `make-calib-scenes.py`'s
  DAZ-saved input) and the `.mi`-only Calib-Mi-AoA … AoA5, -Mi-Legacy,
  -Legacy2. Three of them had references from the oblique Perspective
  View too — by eye against their Octane renders.

## Fix (a47b777)

- `scripts/dev/duf_camera.py`: `missing` / `ensure` (the only camera
  current; several are refused, not guessed), CLI `--fix`.
- The 12 scenes got `current_camera "#Camera-1"` and nothing else
  (checked: the scene minus the key equals the old one); My Library copies
  replaced (same content).
- `make-calib-scenes.py`, `make-mi-shader-scenes.py`,
  `make-node-flag-scenes.py` call `ensure` before saving.
- `iray-queue.sh` refuses a batch with such a scene (nothing queued);
  `keep-refs.py` refuses a render made from such a copy (it refused the old
  Calib-NVIDIA2 render); test `calibration_scenes_render_through_their_camera`
  checks `calib/scenes`.

## Re-render (c76e35c)

Iray Server, 5000 samples, one queue DAZ session (started 22:55, no NVIDIA
module loaded before; no module shared within the batch, #94): Calib-NVIDIA2
133 s, -BumpGamma-NVIDIA 86 s, -CastShadows-Off 108 s, -Shell-G9 177 s. No
MDL import errors. All four now framed as Octane's.

## Calib-NVIDIA2 against Iray (enhanced-final, Octane 1024 samples)

Disk r 0.75 / centre r 0.35 of each sphere (discs from Calib-Glossy's
reference, same layout), spread = p99 / median over the disk:

| sphere | disk | centre | spread O / I | g/r O v I | b/r O v I |
|---|---|---|---|---|---|
| A1 asphalt | 1.05 | 1.07 | 4.1 / 4.1 | 1.04 v 1.07 | 1.18 v 1.18 |
| A2 ceramic tiles | 1.04 | 1.04 | 1.5 / 1.8 | 1.12 v 1.11 | 1.25 v 1.24 |
| A3 architectural g 0.3 | 1.02 | 1.09 | 8.9 / 9.7 | 1.14 v 1.14 | 1.41 v 1.39 |
| A4 architectural g 0 | 0.95 | 0.90 | 1.8 / 1.7 | 1.12 v 1.13 | 1.33 v 1.33 |
| A5 architectural aniso 4 | 0.99 | 1.03 | **4.1 / 6.4** | 1.19 v 1.18 | 1.56 v 1.52 |
| A6 architectural glass | 1.04 | 1.04 | 4.5 / 4.6 | 1.18 v 1.17 | 1.54 v 1.53 |
| A7 resin grey | **1.28** | **1.20** | 1.8 / 1.6 | 1.12 v 1.12 | 1.32 v 1.34 |
| A8 resin r 0.02 | 1.01 | 1.00 | 10.8 / 10.6 | 0.43 v 0.38 | 0.58 v 0.52 |

Floor 1.03. #95's Phong-lobe rule holds on the dielectrics here (A1, A2,
A6, A8 within 5 %, spreads alike). Two real gaps:
- A7: `RESIN_LOSS` (1 − 0.64 r) was fitted to bring A7 to 1.02 × against
  the 2026-11-05 scratch copy with the *fitted* view — another view of the
  HDRI, so that fit doesn't carry over. Open as #113 (decompose first).
- A5: level right, highlight broader than Iray's (spread 0.64 ×): #114.

## The other three references

- Calib-BumpGamma-NVIDIA: `bump-gamma.py` on old / new reference gives the
  same verdict (card 1 RMS vs a 2.2 decode 0.032 / 0.030, cards 2 – 3 vs 1.0
  0.018): #91 stands.
- Calib-CastShadows-Off, Calib-Shell-G9: frame 0.99 × against the existing
  Octane renders; #36's conclusions were drawn by eye and hold.

## Process lessons

- A finding recorded only in a lab note ("check `current_camera` before
  queueing") doesn't stop the next batch: the 2026-11-05 note said it and
  three more references were rendered wrong afterwards (2026-11-06, -09).
  Put it in the queue script, keep-refs and a test.
- A fit against a reconstructed view is a fit against another scene;
  re-check such constants when the true reference arrives.
