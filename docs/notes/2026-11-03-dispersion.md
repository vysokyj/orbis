# 2026-11-03 — Refraction dispersion (Abbe)

Issue [#33](https://github.com/vysokyj/orbis/issues/33). Scenes:
`Calib-Dispersion` (Iray reference since 2026-10-30, 5000 samples) and the
new `Calib-Dispersion-Rough`. Variant enhanced-final; Octane at 1000 samples
(calib-variants) and 600 (probe 112).

## What the docs say

- DAZ's Uber MDL: `material_ior = base::abbe_number_ior(glossy_ior,
  abbe_number)` when dispersion and refraction are on; `refraction_base`
  splits the BSDF and makes the transmission `roughness_u: 0` with
  dispersion, whatever the glossy or refraction roughness.
- `src/iray.rs` already hands over `dispersion_enable` (refraction > 0 and
  Abbe > 0) and `abbe_number`, verified by the Calib-Dispersion `.mi`
  fixture. Unchanged.
- Octane Universal (probe 104's pin dump): `dispersion_coefficient_B`
  (pin type float, default node 6, range 0 – 100, default 0) and
  `dispersionMode` (1 Abbe number — the default — or 2 Cauchy). In Abbe
  mode the coefficient is the Abbe number; 0 is no dispersion, like Iray.

So the mapping is direct: the Abbe number as is, mode left at its default.

## Attempts

1. `dispersion_coefficient_B` written like the other float pins (node 31,
   a float texture). Rendered **identical** to the export without it
   (old binary built from HEAD in a scratch copy): pixel diff only noise.
2. Probe 112 (`scripts/dev/octane-probe-dispersion.lua`) imports the export
   and lists the pins: `ior`, `roughness` arrive, `dispersion_coefficient_B`
   is **nil** — Octane dropped a texture node on a plain float pin. Octane's
   own sample scenes (`caustic.orbx`, `vectron.orbx`) carry the pin as a
   float value node `type='6'` with a float3 attribute (`type='8'`).
3. As node 6 / attr 8 (`"20 0 0"`): the probe reads 60 / 40 / 20 / 10 / 20
   / 30 / 55 back and the render shows fringes. Same probe, overrides on
   the dispersive spheres: Abbe 3 and Cauchy B 0.05 roughly double the
   fringe metric again — the pin is live.

Lesson: a pin whose dump says `type=2` (float, not texture) needs node 6;
check the import with a probe before trusting an unchanged render. Other
`float_pin` uses may hide the same mistake (none found here: those pins are
texture pins).

## Numbers (Calib-Dispersion)

Luminance mean over each sphere's disc (linear) and a fringe metric:
RMS of the high-passed (R − B) inside the disc / its luminance × 100.

| sphere | Iray lum | Octane lum | ratio | Iray fringe | Octane, no disp. | Octane, Abbe |
|---|---|---|---|---|---|---|
| X1 Abbe 0 | 0.194 | 0.201 | 1.04 | 9.82 | 10.09 | 10.11 |
| X2 Abbe 60 | 0.221 | 0.228 | 1.03 | 9.22 | 9.76 | 10.02 |
| X3 Abbe 40 | 0.216 | 0.223 | 1.03 | 9.99 | 10.49 | 10.88 |
| X4 Abbe 20 | 0.211 | 0.218 | 1.03 | 10.70 | 10.88 | 11.86 |
| X5 Abbe 10 | 0.206 | 0.212 | 1.03 | 12.41 | 11.18 | 14.56 |
| X6 IOR 1.8, Abbe 20 | 0.219 | 0.226 | 1.03 | 12.91 | 12.25 | 14.37 |
| X7 IOR 2.4, Abbe 30 | 0.235 | 0.240 | 1.02 | 15.33 | 14.77 | 16.69 |
| X8 IOR 1.33, Abbe 55 | 0.115 | 0.118 | 1.03 | 17.27 | 17.66 | 18.42 |

Fringe excess over no dispersion (Iray's estimated as Iray − 0.97 ×
Octane's no-dispersion value, 0.97 being X1's Iray / Octane): X5 1.5 vs
Octane 3.4, X6 1.0 vs 2.1, X7 1.0 vs 1.9; the weak ones (Abbe ≥ 40, X8)
are within the metric's noise. Octane disperses ~2× Iray at the same
Abbe. Iray rendered with `iray_spectral_rendering` off (RGB), which can't
spread the spectrum like Octane's spectral kernel. Kept the number as is
(its physical meaning); the choice is asked in #33. Frame 1.04 as before.

Timings: probe 112, 3 renders × ~57 s at 600 samples; calib-variants
two scenes ~2 min.

## Smooth transmission

Rule from the MDL: a dispersive, mostly refractive (≥ 0.5), not thin body
gets Universal's `roughness` 0 (one roughness for reflection and
transmission; the transmission dominates glass). `Calib-Dispersion-Rough`
(generated, `make-calib-scenes.py`): pairs without / with Abbe 20 at a
shared glossy roughness 0.3 / 0.6 (Glossy Layered Weight 1) and at the
refraction's own 0.3 / 0.6. Octane: the Abbe spheres clear, gloss 0.6 and
own 0.6 frosted, the 0.3 ones nearly clear (× 0.3 refraction scale). Needs
its Iray reference (listed in #33, "Iray batch: needs").

## Open

- ~~Iray reference for Calib-Dispersion-Rough~~: done 2026-11-04, rule holds.
- ~~Dispersion strength~~: kept the Abbe number (decision 2026-11-03).

## 2026-11-04: Calib-Dispersion-Rough against Iray

Iray reference: Iray Server, 5000 samples (batch of 2026-11-04, fe12e27).
Octane: enhanced-final, frame 1.04. Per sphere disc (radius 36 px):
linear luminance and a detail metric — mean |blur 1.5 px − blur 4 px| /
luminance × 100 (a mid band; the plain high-pass measured Octane's noise
on the frosted spheres, R5 14.8 vs Iray 1.2, so it was dropped).

| sphere | Iray lum | Octane lum | ratio | Iray detail | Octane detail |
|---|---|---|---|---|---|
| R1 gloss 0.3, Abbe 0 | 0.270 | 0.276 | 1.02 | 8.6 | 11.4 |
| R2 gloss 0.3, Abbe 20 | 0.309 | 0.316 | 1.02 | 8.4 | 8.5 |
| R3 own 0.3, Abbe 0 | 0.292 | 0.301 | 1.03 | 5.6 | 5.9 |
| R4 own 0.3, Abbe 20 | 0.280 | 0.289 | 1.03 | 8.2 | 8.4 |
| R5 gloss 0.6, Abbe 0 | 0.256 | 0.303 | 1.19 | 1.2 | 1.2 |
| R6 gloss 0.6, Abbe 20 | 0.270 | 0.274 | 1.01 | 8.0 | 9.5 |
| R7 own 0.6, Abbe 0 | 0.229 | 0.261 | 1.14 | 1.3 | 2.6 |
| R8 own 0.6, Abbe 20 | 0.172 | 0.177 | 1.03 | 14.4 | 14.5 |

The MDL reading holds: Iray renders every dispersive sphere clear
whatever the roughness, like the export (roughness 0). No code change.

Aside, not dispersion: rough glass without dispersion at 0.6 (R5, R7) is
1.14 – 1.19 × Iray in Octane and R7 a little less frosted (detail 2.6 vs
1.3); R1 at gloss 0.3 is a bit sharper (11.4 vs 8.6). Filed as its own
issue (#72).

Timings: one enhanced-final render, ~3 min incl. export.
