# 2026-10-27 – 28: hair widths, four variants, the eyes, thin walls

Commits `6b5dbc9` … `46fdf7b`; issues #16, #20, #25, #42, #44, #45.
Enhanced + Final unless stated.

## 1. Strand-hair width (#16, `6b5dbc9`)

**Symptom.** In the Calib-Eye close-up the strands rendered 3–4× thicker
and lighter than Iray's.

**Cause.** DAZ's channel is *Line Start Width (mm)* (Calib-Eye's hair:
0.12 → 0.01 mm), but `LINE_WIDTH_SCALE` made it 4× that. The 4× dated from
dumps that carried only the guide strands. It made up for the missing PR
hairs. Dump format 3 carries the render strands (36 448 on Calib-Eye), so
the 4× had become pure excess. The colour fit done at 4× had hidden it.

**Width sweep** (k × DAZ's mm, 400 samples, crown / fringe / side):

| scene | k = 1 | k = 2 | k = 4 |
|---|---|---|---|
| Calib-Hair | 2.20 / 1.43 / 1.65 | 1.56 / 0.98 / 1.13 | 0.93 / 0.66 / 0.82 |
| Calib-Hair-dark | 2.33 / 1.40 / 1.65 | — | 0.85 / 0.49 / 0.69 |
| Calib-Hair-darktr | 3.38 / 1.49 / 1.60 | — | 1.13 / 0.39 / 0.63 |

- **Coverage** of the Calib-Eye close-up is the share of pixels more than
  15 % off the background. Iray 0.43; k = 1 0.28, **k = 2 0.47**, k = 4
  0.63. k = 1 is sub-pixel, so the sky shows through the crown.
- At k = 1 the hair looked grey-blue (b/g 1.03 vs Iray 0.78). This was
  first suspected to be sky glints (specular 0.25 = Glossy Layer Weight).
  A specular {0.25, 0.1, 0.03} × albedo {1, 0.6} grid ruled that out:
  specular 0.25 → 0.03 changed the crown by 3 % only. It is coverage.
- Octane's Hair material is linear in its albedo: albedo × 0.6 gives
  exactly × 0.60.

**Result.** `LINE_WIDTH_SCALE` 0.002 (2× DAZ's mm). Colour refit at 2× on
the crown: `HAIR_COLOUR_SHARE` 0.046, `HAIR_TRANSMISSION_SHARE` 0.110.

| | blond | dark | darktr |
|---|---|---|---|
| crown | 0.91 | 0.96 | 1.02 |

Fringe 0.65–0.76, sides 0.75–0.93.

**Left open.**
- Hue (#42): the blonds are less saturated (r/g 1.16 vs Iray 1.47), and
  darktr is more saturated (1.18 vs 1.08).
- **Backlit glow** through the strands (Octane's TT lobe, a sunset through
  the hair) is kept on purpose as an Enhanced feature. DAZ/Iray doesn't do
  it (the user's call). Faithful may damp it.

**Tool.** `orbx-variants.py … hair=<albedo×>,<specular>`.

## 2. Four variants (#44, `1a1b1ee`, `9d37057`, `84c0e0d`)

**Decision (user).** An export is Look (Enhanced / Faithful) × mode
(Final / Preview). Calibrate Enhanced + Final first, then run the same scene
set for each of the other three and tune each on its own. Every difference
is a named field of one table (`src/tuning.rs`, `Variant::tuning()`, four
full rows), so any exception can be traced back.

**Refactor check.** 16 exports (4 scenes × 4 variants) were byte-identical
before and after.

**Calibration.** 17 scenes, `scripts/dev/calib-variants.sh all -- …`.

| finding | before | fix | after |
|---|---|---|---|
| Preview exposure 1.0 (from the Octane 1 era) | floors 0.71–0.74 | `base_exposure` 1.4 | 0.99–1.04 |
| Faithful sun: Octane's model barely dims a low sun | 18:00 1.86, noon 1.08 | power × exp(−0.2001 / max(sun.y, 0.05)), `daylight_power` 2.864 | 18:00 0.95, noon 1.00, node-steered 0.98 |
| Preview (direct lighting) renders a fully transmissive surface black | translucency-1 sphere black | `max_transmission` 0.75 | 0.66, in line with its neighbours |

- A grey sunset colour (0.5 / 0.3 / 0.15) did nothing for the Faithful
  sun. Octane's response to power near the fit is quite non-linear, so it
  took several rounds.
- Spheres in Preview are about 10 % darker: direct lighting has only two
  bounces, while the amount of light is right.
- Left, in all variants: Calib-Glass R4 (absorbing) 2.8×. Preview only:
  glass sphere 7 0.81, Mixing sphere 8 0.76.

## 3. Cornea (#20, `59159dc`)

**Symptom.** A dark band and the room mirrored over the iris, plus a sharp
highlight.

**Causes.**
1. The cornea's Top Coat is in mode 0 (Reflectivity 0.3). It was written as
   Fresnel IOR 1.5 with roughness 0.0016. Mode 0 means F0 = 0.08 × Reflectivity
   = 0.024, which gives IOR (1 + √F0) / (1 − √F0) ≈ 1.37 (`top_coat_ior`).
2. The cornea and tear film are thin-walled with their own refraction. They
   took the sharp refraction's roughness 0 instead of the gloss's 0.07. A
   thin wall doesn't blur what is seen through it, so it now keeps the gloss
   roughness, and the Top Coat takes that minimum roughness.

**Result** (Calib-Eye):

| region | before | after |
|---|---|---|
| pupil | 0.43 | 0.89 |
| iris | 0.57–0.74 | 0.79–0.83 |
| sclera L / R | 0.97 / 1.13 | 0.97 / 1.13 |

The highlight is now Iray's soft glow.

**By-product (#25, Calib-TopCoat):**

| sphere | Iray look | before | after |
|---|---|---|---|
| T1 Reflectivity 0.5 | glossy coat | 1.11 | 1.11 |
| T2 Reflectivity 1 | stronger coat | 0.75 | **1.09** |
| T3 Weighted 1 | a mirror | 0.10 | 0.10 |
| T4 Weighted 0.4 | 40 % mirror over the base | 0.15 | 0.15 |
| T5 / T6 Fresnel 1.5 / 2 | hardly any coat | 2.18 / 4.16 | same |
| T7 / T8 Custom Curve | hardly any coat | 2.49 / 2.54 | same |

Taking F0 from the curve's normal value made T8 10.7×, so that was
reverted. Weighted, Fresnel and Curve stay open in #25.

## 4. The caruncle and Base Color Effect 1 (#20, #45, `37aaf04`, `46fdf7b`)

**Symptom.** The caruncle (inner eye corner) rendered 4.8× Iray: light,
yellowish and pore-textured, where Iray's is dark and smooth. The rest of
the EyeSocket was 1.06×.

### 4.1 First, wrong fix (`37aaf04`, since removed)

- A black EyeSocket albedo blacked the caruncle out, so the caruncle is part
  of the EyeSocket material: thin-walled, Translucency Weight w 0.65,
  Top Coat 0.45 / roughness 0.7.
- Albedo × (1 − w) = 0.35 got it to 1.8× only. Coat and gloss alone, with a
  black albedo, gave 0.27.
- Iray renders of Calib-Eye with the EyeSocket's w at 0 / 0.3 / 0.65 gave
  the caruncle at 0.287 / 0.143 of its w 0 render. Both points fit
  (1 − w) × 0.41. The tear film made no difference (1.00).
- Fix: `THIN_SKIN_DIFFUSE` 0.41, applied **by the name EyeSocket**, with
  figures' thin walls excluded from transmission. Caruncle 0.94.

**User's objection.**
- "We write the spec ourselves, so it needn't be true." The spec's
  justification ("nothing comes through, the head is behind it") had been
  an argument, never a measurement.
- "Iray doesn't know what it renders." A rule keyed on an object name or on
  figure vs prop has no physical meaning. Iray sees only the material and
  the geometry.

### 4.2 Looking for a material-only rule on the eye (dead end)

Rules tried, all for the same material class (thin-walled, translucent,
Base Mixing 0):

| rule | caruncle | Calib-SSS thin spheres w 0.5 | Calib-Leaf backlit card |
|---|---|---|---|
| albedo (1 − w), transmission w (props) | 1.87 | **0.93–0.97** | **0.89** |
| no transmission, albedo (1 − w) × 0.41 (EyeSocket rule) | **0.94** | — | — |
| albedo (1 − w) × 0.41, transmission w | 1.00 | 0.47–0.50 | — |
| albedo (1 − w) × 0.44, transmission min(2w, 1) | ~1.0 (est.) | 0.84–1.08 | 1.75 |
| albedo mix(0.41 (1 − w) D, 0.59 w Tc), transmission w | 1.09 (Tc white 3.11) | = props | — |

No fixed shares fitted all three. The conclusion "something the spheres
don't have" was right.

**Iray bisection on the eye.** Calib-Eye without hair at 100 samples, about
228 s each. Caruncle relative to the base render:

| change | ratio |
|---|---|
| w 0 | 5.17 |
| w 0.3 | 2.0 |
| Invert Transmission Normal off | 1.00 |
| Top Coat off | 1.00 |
| Translucency Color white, no map | 2.09 |
| tear film hidden | 1.00 |

All 23 caruncle faces face the camera. The Translucency Color moves the
*reflected* part as well. This pointed to a channel that couples the two,
but the eye was too slow to bisect further.

### 4.3 Calib-Thin: the primitives that decided it (`84370ee`)

`make-calib-scenes.py` builds these from Calib-Leaf: three vertical
thin-walled cards on a floor, a distant light, diffuse D green
(0.25, 0.55, 0.1). The geometry equals Calib-Leaf's, so its dump exports
every variant.

| scene | cards (w, Translucency Color) |
|---|---|
| `-A` | w 0.5: Tc white / Tc = D / Tc dark (0.05, 0.02, 0.02) |
| `-B` | w 0 / w 0.3 Tc dark / w 0.65 Tc dark |
| `back-` / `front-` | light rotation y 180 (backlit) / 0 |
| `-bce` | Base Color Effect 1 instead of 0 |

Iray at 100 samples takes **22 s** a scene; the set takes under 3 minutes.

**Export at the default rule, Base Color Effect 0** (mean |log| 0.044):

| | A: Tc white | A: Tc = D | A: Tc dark | B: w 0 | B: w 0.3 | B: w 0.65 |
|---|---|---|---|---|---|---|
| back | 0.89 | 0.89 | 1.06 | 1.00 | 1.01 | 0.95 |
| front | 0.96 | 0.97 | 0.99 | 1.00 | 1.01 | 0.99 |

So Iray's plain thin translucent wall is what the export already did. Rough
grids of albedo × a, transmission × t on these scenes only got worse
(a 1 t 1.5: 0.074; a 1 t 2: 0.121; a 0.7: ~0.29; a 0.44: ~0.60).

**Iray bisection on the cards** (front-B with one EyeSocket channel copied
over each time, ~28 s each; w 0.3 / w 0.65 relative to w 0):

| copy | w 0.3 | w 0.65 |
|---|---|---|
| base | 0.697 | 0.350 |
| SSS on | 0.697 | 0.350 |
| Transmitted Color | 0.697 | 0.350 |
| gloss | 0.722 | 0.393 |
| **Base Color Effect 1** | **0.281** | **0.140** |

The eye's caruncle gave 0.287 / 0.143, so this was the match. Top Coat and
Invert Transmission Normal had already been ruled out on the eye.

**What Base Color Effect 1 does in Iray** (bce / base, front-lit):

| card | luminance | r / g / b |
|---|---|---|
| B w 0 | 1.000 | 1 / 1 / 1 |
| B w 0.3 Tc dark | 0.404 | 0.403 / 0.404 / 0.410 |
| B w 0.65 Tc dark | 0.401 | 0.401 / 0.401 / 0.388 |
| A w 0.5 Tc dark | 0.397 | 0.418 / 0.396 / 0.390 |
| A w 0.5 Tc = D | 0.540 | flat |
| A w 0.5 Tc white | 0.956 | 1.49 / 0.86 / 1.88 |

- With a dark Tc it is a flat ×0.40 of the diffuse reflection at any w.
- With w 0 it does nothing.
- A brighter Tc adds back light passed through the wall (from the floor and
  the sky). A white Tc shifts the hue toward Tc.
- Backlit, it passes more: Tc = D 1.77×, Tc white 1.23×.
- A linear fit of the effective albedo to (1 − w) D and w Tc failed
  (a −0.30, b 1.77, predictions off).

**Octane hypotheses on the -bce scenes** (`orbx-variants.py`, mean |log|):

| hypothesis | score |
|---|---|
| export at default shares | 0.679 (front 1.8–2.5, back up to 2.38) |
| albedo × 0.4 | 0.184 |
| albedo × 0.4, transmission Tc → D × 2 | 0.193 |
| albedo × 0.4, transmission Tc → D | 0.389 |
| albedo × 0.4, transmission × 1.6 | 0.053 |
| **albedo × 0.4, transmission × 2** | **0.010** |

All cards came out 0.98–1.02. The w 0 card read 0.39 because the variant
script scaled its albedo too. The rule only applies where there is
transmission.

**Rule (`46fdf7b`).** A surface with transmission > 0 and Base Color Effect
1 gets diffuse × `SCATTER_TRANSMIT_DIFFUSE` 0.4 and transmission share ×
`SCATTER_TRANSMIT_TRANSMISSION` 2. It depends on the material only. The
EyeSocket name rule, `is_eye_socket`, `THIN_SKIN_DIFFUSE` and the
figures-only `thin && !mat.prop` exclusion are gone.

**Verification:**

| scene | Base Color Effect 0 | Base Color Effect 1 |
|---|---|---|
| Calib-Thin front-A | 0.96 / 0.97 / 0.99 | 1.01 / 1.01 / 1.01 |
| Calib-Thin front-B | 1.00 / 1.01 / 0.99 | 1.00 / 0.98 / 1.01 |
| Calib-Thin back-A | 0.89 / 0.89 / 1.06 | 1.00 / 1.00 / 1.00 |
| Calib-Thin back-B | 1.00 / 1.01 / 0.95 | 1.00 / 0.98 / 1.02 |

- Calib-SSS spheres (unchanged): 1.04 0.97 0.99 0.97 0.96 0.95 0.95 0.93;
  floor 1.04.
- Calib-Leaf frame (unchanged): 0.98.
- Calib-Eye: caruncle 1.00 (was 4.6–4.8), pupil 0.88, iris 0.79, sclera
  0.97 / 1.13.
- 101 materials in the user's library set Base Color Effect 1, so the fix
  reaches beyond eyes.

**Not checked.**
- Hue under a white Tc: Iray tints toward Tc there, and the rule is a
  scalar. Only luminance was fitted.
- Base Color Effect 2 (Scatter & Transmit Intensity).
- Faithful and Preview on Calib-Thin.

## 5. Process lessons

- **Material first on primitives.** Calib-Thin answered in 22 s a render
  what hours on Calib-Eye could not. Use a figure only to confirm.
- **Same samples as the reference.** Eye variants rendered at 400 samples
  against a 100-sample reference took 1280 s instead of 318 s. The 36 k PR
  hairs cost little by comparison: 228 s without them, 318 s with them.
  Measure and print each render's time.
- **Rules depend on the material, never on names or object kinds.** If two
  surfaces with "the same" material differ in Iray, a channel is missing
  from the comparison. Bisect channels on primitives.
- **The spec is our claim, not ground truth.** Re-measure a spec sentence
  before building on it.
- **The dump cache:** `orbis export --dump X` moves X into the cache under
  the scene's name, so pass a copy. Rebuild with `cargo build --release
  --bins --examples`. After a tar restore, touch the sources, because cargo
  trusts mtimes.
- **Octane X** crashed at start-up three times in one evening (21:11, 21:40,
  22:16). `octane-run.sh` now relaunches it up to twice.
- **Push over ssh** hung waiting for a prompt. Use `GIT_SSH_COMMAND="ssh -o
  BatchMode=yes"`.
- **Tools added:**
  - `calib-variants.sh`
  - in `orbx-variants.py`: `set=<mat>.<pin>=<f|c|p|x>…` (x scales the
    existing value per channel), `label~hyp` file names, nested-node pin
    matching by depth, and pins a material lacks are skipped.
