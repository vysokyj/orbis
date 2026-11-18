# 2026-11-19 — Cutout maps on strand hair (#202)

Athena Hair (store, Uber fibres, local Calib-Hair-Athena) vanished in
Octane in both Looks under its Cutout Opacity maps (2026-11-17 hair-set
note). Goal: read the map as Iray does on strands, or a neutral value -
never one texel.

## What the export did

`BasisCurves` carry no UVs, and the Uber's opacity went out as an image
texture as on a mesh. Octane then reads every strand at the image's
corner: Athena's two maps (2048², v-only fades: top 20 / 35 % of the
image 0 → 1, constant across u; means 0.907 / 0.659 decoded) are 0 at
(0, 0), so the hair went. On the synthetic scene below Octane read a map
opaque for u < 0.5 as 0.5 (bilinear across the wrap at u = 0): the
mapped columns 0.42 - 0.63 of the plain one in the sparse rows. Morgan's
TR2 (2026-11-14) read ~0.6 against its mean 0.847 - the same mechanism.

## The docs

- Iray programmer's manual 8.1.4: on fibre geometry texture space 0 is
  x along the fibre (0 root → 1 tip), y around it, z the radius; space 1
  per-fibre coordinates, space 2 per-vertex. Fibres under a regular BSDF
  are single-sided and opaque (transmission discarded). MDL spec: the
  same for hair shading.
- DAZ's `.mi` of the scene below: the cutout map goes through
  `base::transform_coordinate` with no `coordinate` argument, i.e. the
  default `coordinate_source(texture_space: 0)`, as on every surface;
  the fibre objects carry no per-hair or per-vertex texture coordinates
  (the strands' UV set is all 0; Morgan's `.mi` had a per-fibre `hair t
  2`, the root's UV).

So the reading was: the map's u along the strand (W in Octane), its v
around the fibre. First implementation: a Gradient on the W coordinate
from the map's column means (v averaged), a constant when it doesn't
vary. Athena's maps then give 0.907 / 0.659 (no variation along u).

## Calib-Strand-Cutout-Dome (Iray, 2000 samples, home server, 1139 s)

`make-strand-scenes.py`: grey 0.5 fibres under Calib-Aniso's dome, rows
G (one thick strand), H (300 sparse), I (3000 dense), J (300 vertical);
columns: no map, opaque for u < 0.5, for v < 0.5, but for v 0.75 - 1
(our own 0 / 1 PNGs, so no gamma question). Cell sums, linear:

| | u-half / plain | v-half / plain | v-band / plain |
|---|---|---|---|
| Iray G / H / I / J | 0.99 / 1.01 / 1.01 / 0.93 | 0.99 / 1.01 / 1.00 / 0.89 | 0.99 / 1.02 / 1.02 / 0.84 |
| Octane as exported before | 0.42 / 0.63 / 1.03 / 0.57 | 0.42 / 0.63 / 1.03 / 0.55 | 0.43 / 0.63 / 1.04 / 0.52 |

(J's columns differ by their strands' placement: Octane's uniform 0.5
before shows the same 0.57 / 0.55 / 0.52 order.) **Iray drew every
mapped fibre whole**: no tip fade under the u map, so it does not read
the cutout along the fibre - the manual's space 0 x is not what the
Uber's lookup sees, or the cutout does not reach fibres. All three maps
are opaque at uv (0, 0), where every strand's UVs sit, so "Iray reads
the strand's own UV" fits too.

The along-the-strand rule, rendered anyway (both Looks identical):
u-half 0.51 - 0.53 × Iray (half the strand cut), v-half 0.71 - 0.87,
v-band 0.85 - 0.89. Octane's curve is a tube with two faces: a constant
opacity 0.5 keeps 0.69 / 0.82 / 0.93 / 0.71 of the plain strand (single
/ sparse / dense / vertical), 0.75 keeps 0.83 - 0.93. Dropped: it
contradicts the u-half column.

## Interim rule: the map's mean on strands

Every strand material (fibres, ribbons, prisms; both Looks) writes the
cutout as the map's mean (decoded as Iray reads the channel, scale /
offset / invert applied) × the value. Neutral in the issue's sense - the
hair's coverage as a whole, never one texel.

- Calib-Strand-Cutout-Dome, Octane / Iray: plain 0.94 - 1.02; u-half and
  v-half 0.71 - 0.87, v-band 0.85 - 0.89 (Iray draws them opaque here).
- **Athena Hair is back**, both Looks. Patches (hair-set-compare.py's
  boxes, denoised) Octane / Iray, luminance: Enhanced fringe 0.65,
  temple 0.84, lengths 0.94, lower 1.00 (B 0.66 - 0.92); Faithful 0.61 /
  0.81 / 0.92 / 1.00. Against Octane's -op1 (no map at all): 0.96 -
  1.03 - in the dense pile the mean costs nothing, as Iray's cutout cost
  Morgan's fibres nothing there (2026-11-15).

## Open: what Iray's fibres read (Calib-Strand-Cutout2-Dome)

Same rows; columns a plain Cutout Opacity 0.5, the u map inverted (opaque
for u ≥ 0.5), the v map inverted, an all-black map - each transparent at
uv (0, 0). Octane side rendered (mean rule: 0.69 - 0.93 of the plain
strand for the three 0.5 columns, the black one gone). The Iray Server
stopped answering (connection timed out) before it rendered; parked in
#202:
- plain 0.5 drawn whole → Iray ignores the cutout on fibres: write
  nothing (opacity 1) on fibres, keep the mean on ribbons / prisms (DAZ
  hands those its tessellated UVs);
- the inverted maps and black gone, 0.5 thinned → Iray reads the map at
  the strand's own UV (DAZ's per-fibre root UV): the mean stays as the
  hair's average over its roots until the dump carries strand UVs;
- u-inverted with tips kept → along the fibre after all (then Cutout-Dome
  needs another look).

## Process

- Two `calib-variants.sh` runs started back to back raced the
  `cargo build` of the first: the second exported Athena with the old
  binary (the hair still gone), which looked like the fix failing. Build
  once, then start the renders.
- A synthetic map test needs maps that differ at the strands' UVs: the
  generated strands' UV set is all 0, so Cutout-Dome's maps all read
  opaque there and could not tell "ignored" from "read at the UV".
