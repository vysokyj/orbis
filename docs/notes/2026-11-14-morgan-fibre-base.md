# 2026-11-14 — Morgan Hair's base on round strands, first decomposition (#161)

#161 (from #99): against Iray's round-fibre render of Morgan Hair
(Calib-Hair-Teens-fib / -nogloss-fib, local: store hair, numbers only
here) Octane's base is about 0.55 ×, with or without the gloss, though the
same material on synthetic fibre patches matched (#79). Goal: find which
part of the real hair separates the two. No level fitted. The first run
stopped when Octane X failed a probe; the resume (same day, after the
user restarted Octane X) rendered the cutout and transmission probes and
the real Top Coat Weight 0 export, and parked Iray's side of both
(below, from "Resume").

Patches of the [2026-11-06 note](2026-11-06-morgan-hair-strands.md),
linear means over pixels opaque in both Iray fibre renders, R / G / B,
Octane / Iray -nogloss-fib (2000 samples). Octane enhanced-final, 256
samples, denoised pass.

## Half resolution is the same measurement

The -nogloss-fib export at 640 × 360 (`orbx-variants.py res=640,360`,
Iray box-reduced 2 × 2) reads the full-resolution table of the fibre
compare to 0.01: crown 0.51 / 0.56 / 0.53, hilite 0.52 / 0.57 / 0.57,
middle 0.53 / 0.59 / 0.59, lengths 0.55 / 0.60 / 0.59, tips 0.58 / 0.70 /
0.59. 61 - 68 s a render instead of 123 - 154. The fibre scene's export
(Line Tessellation 1) is Enhanced's ribbon export unchanged; only Faithful
+ Final drops Uber translucency on fibres.

## What DAZ hands Iray agrees with the export

Morgan's Strands in DAZ's .mi (Calib-Hair-Teens-ng-tw001, 2026-11-06):
`diffuse_color` 0.306 / 0.259 / 0.172 = the colour 0.708 / 0.632 / 0.500
× SSS Reflectance Tint 0.430 / 0.410 / 0.344 (Base Color Effect 2 with
translucency, the channel shown). The export's albedo 0.258 / 0.219 /
0.145 is that × (1 − 0.059 translucency) × (1 − 0.106 refraction), its
thin-wall transmission 0.39 / 0.35 / 0.28. The only map on the three
Morgan materials is Cutout Opacity (Morgan Hair TR2); the base colour is
a plain value. So "Morgan's maps" in #161 is the cutout alone.

## The top coat in Octane: not it

`coat-off` (the coat layer's specular 0, base untouched): crown 0.51 /
0.56 / 0.53, hilite 0.51 / 0.55 / 0.54, middle 0.53 / 0.57 / 0.57,
lengths 0.54 / 0.59 / 0.58, tips 0.58 / 0.68 / 0.58 - within 0.03 of the
export. Octane's coat reflection is not the missing half; Iray's coat on
fibres is not measured (no -fib -nocoat reference).

## Iray: synthetic ribbons are brighter than fibres, Morgan's darker

Iray only, per cell (linear G sums), Calib-Strand-UberM columns 5 / 14 /
15 / 16 (whole / no gloss / no translucency / no refraction), rows G
(thick) / H (sparse) / I (dense) / J (vertical):

| Iray ribbons / fibres, dome, no gloss (column 14) | G | H | I | J |
|---|---|---|---|---|
| face-on (-T2-Dome) | 2.50 | 3.07 | 3.42 | 2.60 |
| 65° (-Edge-T2-Dome) | 1.38 | 1.51 | 2.21 | 2.12 |

On Morgan the same quotient is 0.50 - 0.75 (fibres 1.33 - 2.00 × the
ribbons without gloss, fibre-compare note). The synthetic strands carry
Morgan's material without its coat and its cutout; so in Iray something
of the real hair lifts the fibres against the ribbons, or darkens the
ribbons, by 2 - 4 ×. Candidates left: the cutout, the coat on fibres, the
depth of the pile.

## The cutout is the lead

Morgan Hair TR2 (350², gray): constant across the image's width, along
its height 0.12 at the top row → 0.33 / 0.67 / 0.91 at 10 / 20 / 30 % →
1 from 40 % down; mean 0.847. On DAZ's ribbons (UV v along the strand)
that fades one end of every strand. On Iray's fibres
`state::texture_coordinate(0)` is (along the fibre, around it, radius)
(Iray programmer's manual, hair materials and fibre geometry), so the
map's v runs *around* the fibre: no strand end fades, each fibre's
circumference spans 0.12 - 1. The manual also says that without a hair
BSDF a fibre is shaded single-sided and opaque, its transmission dropped
(the Faithful rule of #79). The export writes `BasisCurves` without
texture coordinates, so what Octane samples from the TR2 map on curves is
unknown: from the 2026-11-06 note on, "the export sees what Iray does"
was assumed, not measured.

Probes written, not rendered: opacity 1 (`op1`) and 0.847 (`op085`) on
the three Morgan materials, and their transmission off (`notr`, Iray's
fibres drop it).

## Octane failed

The batch (half, nocoat, notr, op085, op1) failed on `notr` with "render
engine failure" (octane-render.lua line 64) after half and nocoat; Octane
X is left open for its dialog (#50), not retried, the two cutout probes
not rendered.

## Resume: the Octane side split

After the user restarted Octane X: the same half-resolution export,
probes `op1`, `op085` first and `notr` last (it had failed). All three
rendered (30 / 30 / 71 s); `notr` did not fail again, so the earlier
"render engine failure" was not the probe itself. Then the real exports
of three local scene variants (`scene-channels.py`, dumped headless):
-op1 (Cutout Opacity map dropped, value 1), -nocoat (Top Coat Weight 0),
-op1-nocoat. Octane / Iray -nogloss-fib (the Iray side unchanged:
these say what each part does in Octane, not yet what Iray does), my
patch script re-reads the export within 0.02 of the table above:

| Octane variant | crown | hilite | middle | lengths | tips |
|---|---|---|---|---|---|
| export (half) | 0.52 / 0.57 / 0.55 | 0.52 / 0.58 / 0.58 | 0.53 / 0.60 / 0.60 | 0.55 / 0.61 / 0.61 | 0.59 / 0.72 / 0.61 |
| `op085` (cutout = its mean 0.847) | 0.59 / 0.65 / 0.63 | 0.60 / 0.67 / 0.68 | 0.61 / 0.68 / 0.69 | 0.63 / 0.71 / 0.71 | 0.67 / 0.83 / 0.70 |
| `op1` (probe) = -op1 (export) | 0.64 / 0.71 / 0.69 | 0.65 / 0.74 / 0.75 | 0.66 / 0.75 / 0.76 | 0.69 / 0.78 / 0.79 | 0.73 / 0.90 / 0.77 |
| `notr` (transmission 0) | 0.41 / 0.48 / 0.47 | 0.42 / 0.49 / 0.51 | 0.43 / 0.50 / 0.52 | 0.45 / 0.52 / 0.53 | 0.46 / 0.57 / 0.52 |
| -nocoat (export) | 0.64 / 0.89 / 0.83 | 0.60 / 0.84 / 0.77 | 0.62 / 0.87 / 0.81 | 0.65 / 0.90 / 0.83 | 0.73 / 1.11 / 0.89 |
| -op1-nocoat (export) | 0.78 / 1.10 / 1.03 | 0.74 / 1.05 / 0.99 | 0.75 / 1.07 / 1.01 | 0.80 / 1.12 / 1.06 | 0.90 / 1.40 / 1.12 |

What each says:
- **The cutout in Octane** takes 1.23 - 1.25 × (op1 / export, every patch). With
  the map Octane is darker than with the map's mean (0.847: 1.13 ×), so
  on the uv-less curves Octane samples the map lower than its average
  (an effective ~0.6 by interpolation; where on the image is unknown).
  The probe and the real -op1 export agree to 0.01.
- **Transmission** is no missing part: off, Octane drops to 0.78 - 0.86 ×. If
  Iray's fibres drop it (the manual, #79), Octane is further off, not
  closer.
- **The top coat filters the base.** `coat-off` (specular 0) moved
  nothing, but the real Top Coat Weight 0 export lifts G / B 1.33 - 1.56 ×
  and R 1.15 - 1.24 ×: the export puts the coat's absorption on the base
  albedo and transmission (a falloff mix, 0.93 / 0.75 / 0.81 face-on,
  0.69 / 0.56 / 0.46 at grazing). That share was never in the coat-off
  probe; it is the coat's colour on the base, and it carries most of the
  hue gap (G and B lower than R in every row above).
- Cutout and coat together: 0.74 - 1.12 (tips G 1.40). So both parts,
  if Iray's fibres don't apply them, would explain the whole gap - no
  constant left. Whether Iray's fibres do is not measured.

## What Iray must say

Three local Iray variants of -nogloss-fib (store hair: names only),
2000 samples like the reference: -op1, -nocoat, -op1-nocoat. Iray variant
/ Iray -nogloss-fib = Iray's own cutout and coat share on fibres; Octane
variant / the matching Iray variant = what is left. Uber only, no NVIDIA
preset (no `dual_lobe_hair`, no #94 restart). Parked `needs-iray`.

## Process

- Cheap first check of a "the export sees what Iray does" claim: which
  texture coordinates each side really has. Curves without `st` and Iray
  fibres with (along, around) were both unexamined.
- In zsh a `$M` list doesn't split in a `for` loop: the first variant
  batch silently set nothing ("0 pins set, changed" = only the resolution).
- `orbx-variants.py` gained `res=<w>,<h>` and `coat-off`;
  `scene-channels.py` writes channel variants of a scene (Calib-Hair-Teens
  -fib ones: local only).
