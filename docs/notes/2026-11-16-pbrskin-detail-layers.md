# 2026-11-16 PBRSkin Detail Normal Map by layer (#140, round 3)

Resume of #140 after its third Iray batch: Calib-PBRSkin-DetailLayers,
5000 samples on the Iray Server (70453f2, Post Denoiser on@8). After the
2026-11-15 fix (lobe 2 on the detail normal) the dimples correlated
0.95 - 0.98 with Iray's but read 1.14 - 1.6 x Iray's contrast. This scene
puts the map (1 x 1, weight 1, mult 1: no footprint, no blend, the
lobe's roughness as without the map) under one layer at a time, each
beside its no-map twin.

Variant enhanced-final, 2000 samples, Octane's denoised pass (raw within
±0.02 but E1's grain). Measure as 2026-11-15: per sphere (discs from the
dump, `sphere_discs.py`) inside 0.85 R, band-pass detail = std / mean of
luminance minus its Gaussian blur at σ 2 px, Octane / Iray; correlation
of the band-passes at zero shift; spread p99 / median.

## Octane side re-rendered first

The export moved since the Octane side was rendered (2026-11-15): #174
(Uber's lobe 2 smooth again, PBRSkin's kept on the detail normal), #177
(normal maps at their strength, gated by Bump Enable), #162 (rough
dielectric lobes by Look). None reaches these spheres (lobe 1 α 0.16 is
under Beckmann's 0.25 limit, no Bump Enable, Detail Normal Map 1): the
re-render equals the old one to ±0.02.

## Each layer alone

| sphere | detail Iray | detail Octane | O / I | corr | spread I / O |
|---|---|---|---|---|---|
| E1 diffuse no map | 0.009 | 0.010 | (grain) | 0.33 | 1.85 / 1.87 |
| E2 diffuse + detail normal | 0.104 | 0.099 | **0.95** | 0.93 | 2.33 / 2.37 |
| E3 lobe 1 no map | 0.075 | 0.083 | 1.10 | 0.89 | 6.74 / 8.42 |
| E4 lobe 1 + detail normal | 0.346 | 0.475 | **1.37** | 0.95 | 5.65 / 7.14 |
| E5 lobe 2 no map | 0.766 | 0.772 | 1.01 | 0.98 | 8.17 / 7.03 |
| E6 lobe 2 + detail normal | 0.731 | 0.829 | 1.13 | 0.98 | 7.87 / 8.23 |
| E7 Top Coat no map | 0.551 | 0.367 | 0.67 | 0.91 | 9.58 / 14.92 |
| E8 Top Coat + detail normal | 0.800 | 0.902 | 1.13 | 0.95 | 10.15 / 15.08 |

- **The tilt is right** (resume's outcome a): the diffuse alone under the
  map is 0.95 x Iray's contrast at correlation 0.93. The OSL detail
  normal, its decode and tangent frame need nothing.
- **The excess is lobe 1's**: 1.37 with the map, and already sharper
  without it (spread 8.42 vs 6.74, correlation only 0.89). Lobe 2 (α
  0.026) is 1.13.
- **#166 holds**: Iray's coat carries the dimples (E8 0.80 against E7's
  0.55) and so does Octane's (corr 0.95). The smooth coat's own highlight
  is sharper in Octane (spread 14.9 vs 9.6) - a coat matter, not the
  normal.

## Lobe 1's model, not a constant

The resume said check the Phong pin against the MDL before anything.
`pbr_skin.mdl`: every PBRSkin lobe - lobe 1, lobe 2 (lobe 1 x mult), the
Top Coat (`pbr_add_clearcoat`), the flakes - is
`df::microfacet_ggx_smith_bsdf` at α = roughness², default
`multiscatter_tint` (0: no compensation). The export treats them as
Uber's Phong lobes: Universal `brdf` 1 (Beckmann) at `phong_pin(r²)`
(lobe 1 here pin 0.399, α 0.159 against Iray's GGX α 0.16). Same α,
different distribution: Beckmann's core is wider and its tails short,
so a tilt moves more of the lobe's energy.

Known input: the same export with Universal's model switched on every
sphere (`orbx-variants.py brdf=<n>`), nothing else:

| sphere, detail O / I (corr) | export, Beckmann | GGX (2) | GGX EP (6) | STD (7) |
|---|---|---|---|---|
| E2 | 0.95 (0.93) | 0.95 | 0.95 | 0.95 |
| E3 | 1.10 (0.89) | 1.07 (0.99) | 1.02 (0.99) | 1.05 (0.95) |
| E3 spread (Iray 6.74) | 8.42 | 7.12 | 6.66 | 7.82 |
| E4 | **1.37** (0.95) | **1.08** (0.97) | 1.03 (0.98) | 1.23 (0.97) |
| E5 | 1.01 | 0.92 | 0.92 | 0.99 |
| E6 | 1.13 | 1.10 | 1.08 | 1.13 |
| E7 / spread (Iray 9.58) | 0.67 / 14.9 | 0.75 / 12.5 | 0.77 / 11.8 | 0.70 / 13.4 |
| E8 | 1.13 | 1.05 | 1.03 | 1.10 |

GGX, the MDL's own distribution, takes lobe 1 to Iray's shape
(correlation 0.89 -> 0.99) and the dimples to 1.08. GGX EP is closer
still on these spheres, but its multiple-scattering compensation is not
in the MDL (`multiscatter_tint` 0). So #140's remaining contrast is not
the detail normal; it is how the export draws every PBRSkin lobe.

Not changed here: the model is material-wide (all PBRSkin lobes, the
metal Mixes, the coat) and #167's note has a contrary number (`brdf=2`
on Calib-PBRSkin: dual lobe rim 1.19 -> 1.27, P6's spread 1.08 -> 1.39),
not yet decomposed. Filed as #186 with these tables.

## Where it stands

#140 done: the Detail Normal Map is `pbr_detail_normal` (procedural OSL,
tiles, weight, factor; `.mi` checked), lobe 2 and the Top Coat take it
as in Iray, the tilt matches Iray's (0.95), the pattern correlates 0.93 -
0.98 under every layer. What is left is the lobes' distribution (#186)
and the smooth Top Coat's sharper highlight (in #186's table).

Timings: Octane 2000 samples ~2.5 min a render (export + 3 variants,
~11 min with the lock beside the running Iray queue).

## Process

- One layer per sphere settled in one batch what three rounds of
  whole-material spheres hadn't: the diffuse twin separates the normal
  from the lobes' response at once.
- `orbx-variants.py brdf=` on the existing export is the cheap
  decomposition of a lobe's model: three renders, no export change.
