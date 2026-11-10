# 2026-11-10 — Calib-NVIDIA-BumpG1's concrete: a failed MDL import, not a material (#94)

Question: in Calib-NVIDIA-BumpG1 (Calib-NVIDIA with the presets' bump
maps at `map_gamma` 1, #91) the concrete sphere renders 0.52 × Iray, in
plain Calib-NVIDIA 1.00 – 1.06 (#90); old and new binaries agree (#93).
What does the `map_gamma` 1 copy change for concrete beyond the bump?
Decompose before touching the export.

## What DAZ hands Iray (.mi)

The fixtures `Calib-NVIDIA.json` and `Calib-NVIDIA-BumpG1.json` differ for
N3 concrete in one argument: `bump_texture` (`concrete_plain_bump.png`)
gamma "auto" → 1. The diffuse (`concrete_plain.png`) stays "auto", tiling
1. The raw `.mi` agrees: one texture element per file
(`…concrete_plain_bump….texture.g_1.000`), bound to `bump_texture` only.

## What the MDL does with it

`nvidia/concrete.mdl`: the bump texture drives two things — the gloss
tint (`base::file_texture(bump_texture)`, `simple_glossy_bsdf` under a
`fresnel_layer` at `glossy_weight` 0.2) and the whole surface's normal
(`file_bump_texture`, factor 6). The diffuse comes from the colour map
alone. So at gamma 1 the gloss tint gets brighter (map mean 0.34 → 0.61)
and the bump's slope changes with the decode — #90 measured the gloss at
a few percent of concrete's disk (+0.014 at 2.2, +0.025 at 1.0), and a
bump changes the slope, not the mean. Neither explains 1.9 ×.

## The references themselves

Iray, Calib-NVIDIA-BumpG1 against Calib-NVIDIA (sphere centre box,
linear luminance / band-passed detail, box blur r 2 − r 8, over the mean):

| sphere | Calib-NVIDIA | -BumpG1 |
|---|---|---|
| metal brushed | 0.121 / 0.241 | 0.121 / 0.241 |
| cast iron | 0.137 / 0.223 | 0.142 / 0.209 |
| **concrete** | 0.075 / **0.030** | 0.145 / **0.014** |
| polished concrete | 0.098 / 0.065 | 0.096 / 0.062 |
| porcelain | 0.270 / 0.011 | 0.270 / 0.011 |

Iray's BumpG1 concrete has no texture at all: its detail is porcelain's
noise floor, though the colour map alone has a contrast of 0.18 (linear
std / mean). Smooth, bluish grey (RGB 0.129 0.146 0.184). Polished
concrete, whose palette bump PNG went to gamma 1 the same way, kept its
detail. Not a gamma effect — a different material.

## The cause: DAZ's log

The queue's DAZ log (`Studio6/log.txt`) at the BumpG1 load, 2026-11-09
23:25:40:

- `MDL Import Error (4004): The module "::nvidia::concrete" was not
  imported because at least one of the required DB elements is already
  in use`
- `MDL Import Error (4006): DB name for material definition
  "mdl::nvidia::concrete::concrete(…)" already in use`

and none of concrete's images loaded (polished's, granite's, the metals'
did). The scene before it in the same queue session,
Calib-BumpGamma-NVIDIA, had imported the concrete module. DAZ imports an
MDL module into Iray's database once per session; a later scene with the
same preset fails the import and Iray renders that material as a plain
fallback.

Reproduced headless, no render (`openFile` of three scenes in one DAZ
instance): Calib-BumpGamma-NVIDIA → Calib-NVIDIA-BumpG1 logs the concrete
error, then Calib-NVIDIA logs it for porcelain and granite_black — any
NVIDIA preset met a second time in a session. The gamma has nothing to do
with it.

Other references: the main and the `orbisrender` instance logs (since
2026-11-02 / -05) show the error for no other render. Calib-NVIDIA,
-NVIDIA2, -NVIDIA3 rendered without it. BumpG1's other spheres are valid
(their modules loaded once), so #91's Cast Iron finding from it (detail
0.94 × between the decodes) stands.

## Octane against both references

Octane enhanced-final (the #93 renders), disk r 0.75 of the sphere:

| | brushed | cast iron | concrete | polished | porcelain | granite | resin | arch |
|---|---|---|---|---|---|---|---|---|
| BumpG1 / Iray BumpG1 | 0.96 | 0.94 | **0.53** | 0.99 | 1.11 | 1.07 | 0.95 | 1.06 |
| BumpG1 / Iray Calib-NVIDIA | 0.96 | 0.97 | **1.05** | 0.98 | 1.11 | 1.07 | 0.95 | 1.06 |

Against the valid concrete (plain Calib-NVIDIA's, which differs only by
the bump's decode and the few-percent gloss tint) Octane's BumpG1
concrete is 1.05 ×. No export change; no Octane gloss-off / bump-off
variants needed — the gap isn't on the Octane side.

## Guard

`scripts/dev/daz_log.py` lists every scene load that logged an MDL
import error in any DAZ instance's log. `keep-refs.py` refuses such a
render ("re-render it in a fresh DAZ session"), `status.py` flags one in
the queue's log of the last three days. Run on BumpG1's render it skips
it; Calib-NVIDIA's passes.

Iray batch: Calib-NVIDIA-BumpG1 again, 5000 samples, in a queue-side DAZ
started fresh (the running one has the concrete module from 11-09), to
replace the reference and check concrete at gamma 1 (expected within a
few percent of Calib-NVIDIA's).

## Process

- The 0.52 sat in #93's table as "not #93" — the reference image showed
  the untextured sphere at a glance. Look at both references side by
  side before decomposing a ratio.
- The DAZ log is the first stop for a reference that looks like a
  different material: DAZ warns (not errors) and renders anyway.
