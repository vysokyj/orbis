# 2026-11-15 — Morgan Hair on fibres: Iray's cutout and coat shares, the strand taper (#161)

Resume of [2026-11-14](2026-11-14-morgan-fibre-base.md) after its Iray
batch: Calib-Hair-Teens-nogloss-fib-op1 / -nocoat / -op1-nocoat, 2000
samples, Iray Server (local only: store hair, numbers only here). Same
patches (2026-11-06 note), linear means over pixels opaque in both Iray
fibre renders, R / G / B. Octane enhanced-final, 256 samples, denoised
pass, 640 × 360 (`orbx-variants.py res=640,360`, Iray box-reduced 2 × 2).
My patch script reads the 2026-11-14 tables to 0.01 – 0.02.

## Iray's own shares on fibres

| Iray variant / Iray -nogloss-fib | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| -op1 (Cutout Opacity 1) | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 / 1.00 | 1.00 / 1.00 / 1.00 | 1.05 / 1.06 / 1.06 | 1.11 / 1.18 / 1.11 |
| -nocoat (Top Coat Weight 0) | 1.22 / 1.59 / 1.37 | 1.18 / 1.55 / 1.33 | 1.19 / 1.57 / 1.34 | 1.18 / 1.55 / 1.33 | 1.19 / 1.54 / 1.36 |
| -op1-nocoat | 1.22 / 1.59 / 1.37 | 1.19 / 1.56 / 1.33 | 1.20 / 1.57 / 1.35 | 1.24 / 1.63 / 1.41 | 1.31 / 1.80 / 1.50 |

- **The cutout costs Iray's fibres nothing in the dense pile** and 5 – 18 %
  where the hair thins out. The TR2 map's v runs around Iray's fibre
  (MDL spec, hair texture spaces); a ray through a faded part of one fibre
  hits another of the same radiance, so only sparse patches show the
  background. Octane takes 1.21 – 1.27 × off every patch (below), also in
  the crown: on the uv-less curves it samples the map at an effective
  ~0.6 everywhere (2026-11-14). That part is the export's.
- **The coat's filter on the base is Iray's too**: Iray 1.18 – 1.22 (R) /
  1.54 – 1.59 (G) / 1.33 – 1.37 (B), Octane (Top Coat Weight 0 / export,
  after #163) 1.30 / 1.65 / 1.58 on the crown - G within 4 %, R 7 %, B
  15 % more filter in Octane. Not the gap.

## Octane, re-exported after #163

#163 divides translucency's transmission by (1 − albedo); Morgan's strands
have translucency, so the four exports were redone:

| Octane / Iray | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| export / -nogloss-fib | 0.57 / 0.60 / 0.55 | 0.57 / 0.61 / 0.58 | 0.60 / 0.63 / 0.61 | 0.61 / 0.64 / 0.61 | 0.64 / 0.74 / 0.61 |
| -op1 / Iray -op1 | 0.69 / 0.74 / 0.70 | 0.71 / 0.77 / 0.76 | 0.72 / 0.79 / 0.77 | 0.71 / 0.77 / 0.75 | 0.71 / 0.80 / 0.70 |
| -nocoat / Iray -nocoat | 0.61 / 0.62 / 0.63 | 0.58 / 0.59 / 0.60 | 0.60 / 0.62 / 0.63 | 0.62 / 0.63 / 0.64 | 0.69 / 0.79 / 0.69 |
| -op1-nocoat / Iray -op1-nocoat | 0.72 / 0.75 / 0.78 | 0.70 / 0.72 / 0.76 | 0.71 / 0.74 / 0.77 | 0.72 / 0.74 / 0.78 | 0.75 / 0.84 / 0.79 |

#163 lifted every row 0.04 – 0.08 (before: export 0.51 – 0.59, bare
0.63 – 0.79). With the cutout and the coat both out of the way the bare
base is **0.70 – 0.84 × Iray, grey** (R / G / B within 0.08): not a
colour channel.

Probes on the bare base (-op1-nocoat):
- **Path depth** (diffuse and glossy depth 128 instead of 64 / 24): within
  0.01, on the bare base and on the export with its cutout. No bounce
  limit, and the cutout's passes don't run out of depth.
- **Transmission 0** (as Iray's fibres drop it, #79): 0.54 – 0.67. Like
  for like Octane's bare fibre base is ~0.6 × Iray's; the export's
  transmission covers a quarter of the gap that Iray doesn't have.

## The strand taper: DAZ's is quadratic

`.mi` of -op1-nocoat (headless DAZ, local): 68 776 fibres in three
objects (35 861 / 4 369 / 28 546; the dump has 68 773), B-spline, 26 / 16 /
20 control points (the strand's points + one at each end), and a
per-fibre `hair t 2` (the root's UV). The radii are not linear in the
point index: median over fibres 0.0100 → 0.0086 at mid-strand → 0.0050
cm, root + (tip − root) · s². The same on Morgan's ribbons (`.mi` of
-ng-tw001: across-edge width by the strand's v, 0.0200 → 0.0175 at v 0.5
→ 0.0111 at 0.95 – 1) and on a plain polyline of Calib-Strand (8 points,
0.2 → 0.1: 0.01, 0.0099, 0.00959, 0.00908, 0.00837, 0.00745, 0.00633,
0.005 cm - the quadratic to 1e-5). The export tapered linearly: up to 17 %
thinner mid-strand, 10 % less area. Fixed (`strand_width`, spec Strand
hair). On Morgan's pile it moves nothing: every patch +0.00 – 0.02
(bare 0.70 – 0.84 → 0.70 – 0.84, export 0.57 – 0.74 → 0.58 – 0.74).

## Against the ribbon references

The ribbon scene (Calib-Hair-Teens-nogloss, sides 2) re-exported at half
with all of the above, against Iray's ribbons (5000 samples): crown
1.16 / 1.27 / 1.18, hilite 0.79 / 0.85 / 0.81, middle 1.10 / 1.22 / 1.15,
lengths 0.82 / 0.88 / 0.83, tips 0.81 / 0.96 / 0.78. The same Octane
render against Iray's fibres: 0.58 – 0.74. Octane's round curves sit
between Iray's flat ribbons and its round fibres.

## Where it stands

Of the 0.57 – 0.74 against Iray's fibres: the cutout ~0.8 (Octane only),
the coat's filter ~0.9 on B (Octane slightly more), the rest the bare
base at 0.70 – 0.84 (0.55 – 0.67 like for like, without transmission) -
which synthetic fibre patches of Morgan's material don't show (#79:
0.91 – 1.07 dome). What is left is the deep pile of the real hair. Both
next steps depend on which reference Morgan (ribbon hair, Line
Tessellation 2) follows: the cutout on curves should follow the ribbons
(v along the strand: the ends fade) or the fibres (no net effect); a
deep-pile synthetic batch only matters if the fibres govern. Asked on
#161 (`question`).

## Process

- Read the decomposition table before deciding which branch of a resume
  applies: here the cutout took the first branch, the coat the second
  and the bare base neither.
- The `.mi` carries what a geometry rule needs: three `.mi` files read
  the taper in minutes; nobody had checked the linear one since #16.
- `.mi` hair objects are binary: big-endian floats, per fibre an optional
  `hair t` pair then x y z radius per control point; `hair [ n ]` the
  fibres' offsets. Ribbon `trilist` vertices are sorted by x, so widths
  come from the triangles' edges whose two ends share v.
- Waiting on an Octane batch from a foreground command: take the render
  lock (`render_lock` in a `sh -c`), it returns when the batch frees it.
