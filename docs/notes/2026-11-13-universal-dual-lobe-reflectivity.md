# 2026-11-13 — Universal's dual lobe reflectivity as the IOR, its map per texel (#81)

Universal (any Uber surface without translucency, the Calib-Skin `-w0`
variants among them) wrote the dual lobe as uniform shares: `specular` =
W · min(2r, 1) × lobe 1's part, `coating` the rest, the IOR only for r >
0.5, the Dual Lobe Specular Reflectivity map dropped. The OpenPBR skin
path (`skin_gloss`) has mapped the lobes since b75304b as the MDL has
them; Universal now does the same. The sibling #133 (b65dcb7) had already
brought the Weight map to Universal.

## Docs first

- `irayubermaterial.mdl`, `add_dual_lobe_specular`: a `custom_curve_layer`
  with the diffuse as the layer, `normal_reflectivity` 1 − r · W and
  `grazing_reflectivity` 1 − W — the lobes reflect W · (F0 + (1 − F0)(1 −
  cos)^5), F0 = `dual_lobe_specular_reflectivity` (DAZ: 0.08 × the
  channel, × its map). The map sets F0 only; the grazing reflection is W
  whatever the map. A share W · min(2r, 1) dims grazing too.
- Octane probe (`octane-probe-pins.lua`, Universal 130): `ior` (Dielectric
  IOR, min 0.01) and `coatingIor` (min 1) are texture pins (type 5,
  default node float texture 31), like OpenPBR's. So `Ocs::ior_pin`'s
  baked n(F0 · map) / 2 at power 2 works on both.

No fit: the change is the MDL's curve on the pins Octane has.

## Renders

Octane 400 samples, Enhanced + Final, before = the installed binary
(master f0d1e9f), after = this change; Iray references: Calib-DualLobe
(5000, in git), the Calib-Skin `-w0` fronts (400, private scenes, local
only). Octane / Iray, linear R / G / B, median per #86's face regions;
the grey sphere 0.98 – 1.00 in both runs. Raw and denoised passes agree
to 0.01.

| | before | after |
|---|---|---|
| Monique forehead | 1.19 / 1.39 / 1.63 | 0.96 / 0.97 / 1.00 |
| Monique chest | 1.10 / 1.27 / 1.44 | 0.96 / 1.00 / 1.00 |
| Monique skin (median) | 1.00 / 1.10 / 1.16 | 0.96 / 0.97 / 1.00 |
| Twosret forehead | 1.05 / 1.26 / 1.49 | 0.94 / 0.96 / 0.95 |
| Twosret chest | 1.04 / 1.23 / 1.58 | 0.95 / 1.00 / 1.00 |
| Twosret skin (median) | 0.96 / 1.13 / 1.37 | 0.94 / 1.00 / 1.02 |
| Victoria skin (median) | 0.96 / 1.00 / 0.99 | 0.96 / 1.00 / 0.99 |

The blue excess (a white lobe over a dark red skin, #84's "base × 0.88 +
0.034 white") is gone where the gloss shines; Victoria, whose map is near
white, doesn't move.

Calib-DualLobe, mean luminance per sphere / Iray (highlight-spread.py):

| | D1 | D2 | D3 | D4 | D5 | D6 r 0.25 | D7 r 1 | D8 W 0.5 |
|---|---|---|---|---|---|---|---|---|
| before | 1.13 | 1.11 | 1.11 | 1.14 | 1.12 | 1.05 | 1.10 | 1.18 |
| after | 1.13 | 1.11 | 1.11 | 1.14 | 1.12 | 1.16 | 1.10 | 1.18 |

D6 (no map, r 0.25) was the one sphere the share form moved: it now sits
with its neighbours. Their common 1.10 – 1.18 is #137's (the dual lobe
alone 1.1 – 1.4 × Iray depending on the sphere's place), not this rule.
Spreads unchanged (D6 1.6 both).

## Process

- The first "after" export came out byte-identical to "before":
  `cargo test --release` doesn't rebuild `target/release/orbis`; run
  `cargo build --release` before exporting.
- A wait loop on `ls dir/*.png` in zsh fails on no-match and printed
  errors instead of waiting; wait on `pgrep -f octane-render.sh`.

## Open

- #100: its dual lobe half is this change; Glossy Reflectivity still
  takes the share form (`reflectivity_share`, `glossy_excess`) and its map
  is still ignored on Universal.
- #137: the dual lobe alone 1.1 – 1.2 × Iray on Calib-DualLobe.
