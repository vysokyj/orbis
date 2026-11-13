# 2026-11-13 Denoised pass in the compares (#139)

Goal: the compare scripts compared Octane's raw beauty with Iray references
rendered with DAZ's Post Denoiser on (found in #47: Enhanced skin's raw
grain read as 3 - 8 x Iray's fine texture). Make every compare read what
the export actually produces, say which pass it read, and measure which
recorded numbers move between the passes. No render, no rule changed.

## What the passes are

- **Octane**: `octane-render.sh` saves the raw beauty as `<out>/<scene>.png`
  and, since 4efd231, the denoiser's output (`BEAUTY_DENOISER_OUTPUT`) as
  `<out>/denoised/<scene>.png` for an export with the denoiser on.
  `src/tuning.rs`: the denoiser is on for enhanced-final and faithful-final,
  off for both Preview variants. So for a Preview variant the raw image
  *is* the product.
- **Iray**: `renders.tsv` now has a `denoiser` column, read from the capped
  copy each reference was rendered from (beside the cached png): all 269
  references are `on@8` (Post Denoiser Enable, start iteration 8), both the
  223 Iray Server and the 46 local CPU ones. `keep-refs.py` records it for
  new references. At 5000 samples the reference's grain in a flat dome
  patch is at the 8-bit floor (high-pass std / mean 0.0019 on
  Calib-Glossy's background, Octane raw 0.0025, denoised 0.0021), so whether
  the Iray Server applies the Post Denoiser can't be told from the pixels
  there; the 400 / 1000 / 2000-sample references are where it matters.

## The rule (scripts/dev/octane_pass.py)

`pick(png)` -> (file, label), used by calib-compare-all.py (so
calib-variants.sh's report), highlight-spread.py, skin-compare.py,
gloss-split.py, furnace-rings.py, point-highlight.py and
refr-volume-mc.py --measure:

| render directory | denoised/ twin | reads | label |
|---|---|---|---|
| variant, denoiser off (tuning.rs) | any | raw | `raw` |
| variant, denoiser on | fresh | twin | `denoised` |
| variant, denoiser on | missing / stale | raw | `raw*` |
| not a variant (`p117`, `skin-r5`...) | fresh | twin | `denoised` |
| not a variant | missing | raw | `raw*` |
| `iray` (cache or references) | - | as is | `Iray, denoiser on@8` |

A twin is stale when its mtime is more than 120 s off the raw image's
(both come from the same render; a later raw-only render beside an old twin
must not pick the old twin up). `raw*` = grain-sensitive numbers are not
comparable with a denoised reference. `ORBIS_PASS=raw | denoised` forces a
pass - that is how the table below was measured.

## Raw against denoised on what is rendered

Every pair with a fresh twin in the cache (no new render): enhanced-final
19 (Calib-Dispersion-Rough, -DualLobe, -DualLobeGlossy{,-bare,-dl,-gloss},
-Furnace-ArchGlass{,2,3}, -Furnace-Resin2, -Furnace-UberGlass, -Glossy,
-NVIDIA2, -PBRSkin-Occlusion, -Point-ArchAniso,
-Strand-UberM-Edge-T2{,-Dome}, and the private Hair-Teens renders), faithful-final 5
(Calib-G9Eye, the strand and hair ones), p117 / uber118 / uber118-base 3.
All 400 samples.

| metric (script) | largest change denoised vs raw |
|---|---|
| sphere / floor / frame means (calib-compare-all) | <= 1 % (Calib-Glossy, -NVIDIA2 one sphere each), 0 % on the rest |
| p99 / median spread (highlight-spread) | <= 0.1 (<= 3 %), / Iray means <= 0.01 |
| ring profiles (furnace-rings, 7 furnace renders) | <= 3 % (a 0.033 ring of ArchGlass2 S8), the rest < 1 % |
| gloss split a / g (gloss-split, DualLobeGlossy family) | <= 1 %; the bare scene's O / I 1.084 -> 1.079; blurred peak O / I <= 2 % |
| point peaks / contours (point-highlight, Point-ArchAniso) | peak <= 0.6 %, contours within a pixel |
| volume share (refr-volume-mc --measure, Dispersion-Rough) | 0 % |
| Calib-PBRSkin-Occlusion (no Iray reference; objects' pixels) | mean / median 1.000, p99 / p99.5 0.999, 3 x 3 high-pass std 0.92 x |

So for these scenes - direct-lit spheres, furnaces, glass - Octane's 400
samples are converged enough that the denoiser moves nothing over a few
per cent - a sun-lit PBRSkin sphere row too; no spec number recorded on them changes (Calib-Glossy's and
-DualLobe's p99 / median, #95; NVIDIA2's A5 streak, #114; the furnace
rings of #117 / #118 / #147 - #149; #137's gloss split).

## Not rechecked (no denoised twin, needs a render)

The grain lives in Octane's random walk (SSS skin, translucent props), and
those renders all predate 4efd231, so they read `raw*`:

- The skin faces' highlight p99.5 (G9 figures, where #47 saw the grain) (spec, dual lobe / Top Coat section:
  Victoria 0.87, Kayo 0.84 ... the full faces 0.91 - 1.11 x Iray), measured
  on raw renders of private G9 figures at 400 samples - a p99.5 over a
  grainy face is the most exposed number in the spec. Under the #57 / #19
  hold; issue #151 asks for the recheck when #57 resumes.
- Calib-PBRSkin, Calib-SSS, Calib-Skin-* region medians: medians, so
  little exposure; the texture contrast is already recorded on the denoised
  pass (2026-11-12 skin texture contrast).

## Process lessons

- The two older readers (`gloss-split.py`, `furnace-rings.py`) took any
  `denoised/` twin without looking at its age or the variant; one rule in
  one place now.
- A report must name its pass per row: calib-compare-all's table has an
  `Octane pass` and an `Iray denoiser` column and a summary line, the
  other scripts put the label beside the file name.
- The disc finder shared by highlight-spread.py and calib-compare-all.py
  (#146) is a separate fix; octane_pass.py only picks the file.
