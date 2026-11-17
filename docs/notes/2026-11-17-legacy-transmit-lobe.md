# 2026-11-17 — NVIDIA's `mdl 1.0` transmitting lobes are cosᵉ too (#191)

Question (#191, from #184): #184 found Iray's legacy `simple_glossy_bsdf`
(modules below MDL 1.3, every NVIDIA preset DAZ ships) reflect-only lobe
is a Phong of exponent e = 2 / r² (D ∝ cosᵉ), where `libbsdf`'s (1.3, the
Uber) is cos^(e − 1). The transmitting lobes — resin's
`scatter_reflect_transmit` base, architectural's `scatter_transmit` glass,
the coat over them — kept `phong_pin`, because `RESIN_LOBE`,
`LEGACY_LOBE` and the transmission Falloff Mixes were fitted on
`libbsdf`'s lobe. Their head-on shape was unmeasured.

## 1. Scenes (generated, `make-mdl-scenes.py`)

Our own modules (`calib/mdl/orbis_rtlobe_v10 / _v13`, `orbis_tlobe_v10 /
_v13`), Calib-Furnace-Metal's spheres, per roughness r 0.354 / 0.5 / 0.7:
the 1.0 lobe, the 1.3 lobe, and a 1.3 **twin** at r′ = r √(2 / (2 + r²)),
whose cos^(e′ − 1) is cosᵉ — the null test: if the legacy lobe is cosᵉ,
the 1.0 sphere and its twin draw the same highlight (r 0.7 without its
plain 1.3 sphere: 8 slots).

- **Calib-Point-MdlReflTrans**: resin's base lobe (rt, closed, IOR 1.49)
  with an opaque interior (`absorption_coefficient` 1000), so only the
  front reflection is seen, under Calib-Point-MdlAniso's point light at
  the camera, EV 3 (the white 1.0 lobe at r 0.354 peaked 0.018 at EV 11.5,
  ArchAniso P1; F 0.039 of it).
- **Calib-Back-MdlTransmit**: the transmit lobe (closed, IOR 1.5), the
  camera level with the spheres' centres (0, 30, 450), and behind each
  sphere a spot light on its line of sight, 60 cm past the centre, aimed
  at the camera (Y turn only), flat beam (Beam Exponent 0), 40° spread
  (covers its own sphere, R 15, no other, 50 cm apart), 5000 lm (`back_lit`).
  Seen through the ball lens each lamp is a blob whose radius maps the
  deviation the two rough refractions must make. A thin wall can't show
  the lobe this way (its straight-through deviation doesn't change across
  the disc), so the closed sphere only.

Iray Server (home), 2000 samples, post denoiser: 65 s / 90 s. Two failed
tries first, both at 64 samples into a scratch dir: the spots aimed at y
15, the node translation — Calib-Furnace-Metal's spheres are R 15 about y
15 in their geometry, translated 15 up, so the centres are at y 30 (the
blobs sat in the upper half); then EV 16.5 clipped r 0.354's blob ~6×
(read off the edge through the model profile below) → EV 20.

## 2. Reflection of the rt lobe (Calib-Point-MdlReflTrans)

`point-highlight.py`, width × height of the run through the peak above
0.3 / 0.5 of it (px; vertical half-axis 45.9 in every slot, horizontal 46 –
49 by slot, so compare heights). Model: head-on L ∝ D(θ) G / cos θ, x =
R sin θ, v-cavities G, **no Fresnel term** (with the light at the camera h
= k, so F(k · h) is the head-on value everywhere; with the macro F(θ) the
model was 4 – 5 px wide at r 0.7 for the 1.3 twin as well — the first
model was wrong, not Iray). 1.0 spheres also × `legacy_loss(r, θ)` (the
#117 per-event loss, which falls to the rim).

| sphere | Iray w × h 0.3 / 0.5 | cosᵉ | cosᵉ × g | cos^(e − 1) |
|---|---|---|---|---|
| S1 1.0 r 0.354 | 37×35 29×27 | 35.4 27.3 | 35.4 27.3 | 36.5 28.2 |
| S2 1.3 r 0.354 | 37×36 29×28 | | | 36.5 28.2 |
| S3 1.3 twin 0.3434 | 35×36 27×28 | 35.4 27.3 | | |
| S4 1.0 r 0.5 | 49×48 37×38 | 49.5 38.9 | 49.2 38.6 | 52.4 41.4 |
| S5 1.3 r 0.5 | 53×52 41×42 | | | 52.4 41.4 |
| S6 1.3 twin 0.4714 | 49×49 39×39 | 49.5 38.9 | | |
| S7 1.0 r 0.7 | 67×64 55×52 | 66.6 55.3 | 65.0 52.4 | 69.7 60.1 |
| S8 1.3 twin 0.6274 | 71×66 59×54 | 66.6 55.3 | | |

Every 1.0 sphere is cosᵉ × the legacy loss, every 1.3 one cos^(e − 1),
every twin cosᵉ, to about a pixel; at r 0.7 the legacy loss's fall to the
rim narrows the 1.0 lobe 2 px below its twin, as the model says.

## 3. Transmission (Calib-Back-MdlTransmit)

Model (scratch Monte Carlo): pinhole camera at 30 R, lamp 4 R behind,
the entry refraction sampled from the Phong D, the exit evaluated toward
the lamp (Walter's BTDF, v-cavities, no Fresnel), profile along the
vertical through the centre, px at R 46.8.

| sphere | Iray height 0.3 / 0.5 | model cosᵉ (× g) | model cos^(e − 1) (× g) |
|---|---|---|---|
| T1 1.0 r 0.354 | 41 32 | 43 34 (43 34) | 45 35 (45 35) |
| T2 1.3 r 0.354 | 42 33 | | 45 35 |
| T3 1.3 twin | 42 32 | 42 33 | |
| T4 1.0 r 0.5 | 54 44 | 59 47 (58 47) | 63 51 (62 50) |
| T5 1.3 r 0.5 | 58 46 | | 63 51 |
| T6 1.3 twin | 56 44 | 56 45 | |
| T7 1.0 r 0.7 | 70 56 | 78 66 (73 60) | 82 75 (79 67) |
| T8 1.3 twin | 72 58 | 72 59 | |

The model runs ~7 % wide on every 1.3 sphere (0.88 – 0.98 of it; the
v-cavities transmission term, no Fresnel), so read the null test: the
1.0 sphere is its twin's height within 0 – 2 px at every r, and 2 – 4 px
below the 1.3 lobe at r 0.5, where the model expects 4 – 5. Against the
model, scaled by its 1.3 error, cosᵉ × g fits (0.93 – 0.96), cos^(e − 1) ×
g doesn't (0.84 – 0.91). **The transmitting legacy lobe is cosᵉ too**, in
both scatter modes.

## 4. Consequence: the loss table on the cosᵉ lobe

`libbsdf`'s lobe at r′ keeps more in the white furnace than at r (closed
rt IOR 1.49, disk: r 0.25 0.977 v 0.976, 0.5 0.839 v 0.820, 0.75 0.706 v
0.654, 1 0.610 v 0.527; `furnace-rings.py model`), so `LEGACY_LOBE` —
fitted as the per-event loss over `libbsdf` at r — held part of the shape
change. Refitted each row on `libbsdf` at r′ against the same spheres'
measured rings (Calib-Furnace-MdlTransmit T1 – T4, -MdlTir X1 / X3,
-MdlLegacy K1 – K4, -MdlLegacy2 L1 – L4, L7, -Resin2 U3 – U6; A2 left out,
its r is architectural's equivalent), the paths traced once per sphere
with each event's incoming cosine, g reweighting them, Nelder – Mead on
the rings centre – 0.9 (equal weights; area weights moved r 1 to a worse
compromise). The harness first reproduced the old rows on the old lobe
(r 0.5 0.940 / 0.755 / 2.33 v 0.939 / 0.769 / 2.21; r 1 0.848 / 0.608 /
1.04 v 0.840 / 0.609 / 1.11). Worst ring (any sphere of the row):

| r | old row on cosᵉ | refit | head / grazing / skew |
|---|---|---|---|
| 0.05 – 0.3 | 0.008 – 0.017 | 0.006 – 0.011 (loosely held, skew to its bound) | kept |
| 0.4 | 0.017 | 0.012 | 0.955 / 0.870 / 2.352 |
| 0.5 | 0.041 | 0.015 | 0.927 / 0.789 / 2.047 |
| 0.6 | 0.098 | 0.058 | 0.907 / 0.735 / 1.522 |
| 0.7 | 0.091 | 0.030 | 0.885 / 0.628 / 1.454 |
| 0.75 | 0.096 | 0.026 | 0.862 / 0.629 / 1.374 |
| 1 | 0.229 | 0.110 | 0.790 / 0.551 / 1.209 |

(r 1 on the old lobe: 0.105 — one row for IOR 1.05 – 2 and rt.) Rows ≤
0.3 stay: r′ is within 2 % of r there and either fit is within the 8-bit
references. The Rust port (`mdl_layer::trace` with `legacy_rough` for
base and coat) meets U4's measured rings 0.783 → 0.517 within 0.02
(test).

## 5. Export change and Octane (enhanced-final, 400 samples, denoised)

Resin's base and coat take `legacy_phong_pin` (Octane's α in
`resin_lobe_albedo` the same); architectural's glass keeps its swept √r
pins (only its Iray model moves, through `mdl_layer`). Before = the
renders of 2026-11-16 14:28 – 14:33 (#117's falloff), after = this
change.

Furnace disks, Octane / Iray before → after: Resin2 U3 1.02 → 1.02, U4
1.04 → 1.03, U5 1.12 → 1.10, U6 1.17 → 1.17, U7 1.10 → 1.09, U8 1.03 →
1.02; Calib-Furnace-Resin (coated) R1 – R8 1.05 / 1.03 / 1.03 / 1.00 /
1.07 / 1.02 / 1.05 / 1.03 → 1.02 / 1.01 / 1.00 / 0.98 / 1.05 / 1.01 /
1.08 / 1.01; ArchGlass G4 1.05 → 1.04, G5 1.01 → 1.03, G8 0.96 → 0.99,
the rest unchanged; ArchGlass2 H2 1.05 → 1.05, H3 1.02 → 1.03, H6 1.06 →
1.05, H7 0.98 → 1.00, thin walls unchanged. Lit (`calib-compare.py
regions`): Calib-Resin cube1 / cube2 / cube3 1.07 / 1.29 / 1.06 → 1.08 /
1.28 / 1.05; Calib-ArchGlass within 0.003. U5 / U6 (α ≥ 0.5) stay Octane
over its Smith model, #117's open residue.

Not measured: resin's own highlight shape in Octane under a point light
(no NVIDIA-resin point-lit reference); the pin follows the measured Iray
lobe as #184's did for the reflect-only presets, verified there on
Calib-Point-ArchAniso.

## Where it stands

Export changed (resin pins, the legacy lobe's Iray model, `LEGACY_LOBE`
0.4 – 1). Architectural glass's √r pins (swept on renders, #117) against
`legacy_phong_pin`: #198.

## Process lessons

- A null twin (the 1.3 lobe at the roughness whose exponent is the
  hypothesis's) made the transmission readable without trusting the
  model's absolute widths, which ran 7 % wide.
- With the light at the camera the microfacet Fresnel is the head-on value
  everywhere; putting the macro F(θ) in the profile model made it wide at
  high r and almost read as "Iray narrower than cosᵉ".
- Read a primitive's geometry centre (vertex centroid + translation), not
  the node translation, before aiming lights at it.
- A refit harness should first reproduce the table it replaces on the old
  model; here it did (r 0.5 / 1), which also fixed the objective (equal
  ring weights).
