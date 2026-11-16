# 2026-11-16 The skin faces' dual lobe: not the bump, GGX's shape (#175)

After #174 the Calib-Skin faces' dual lobe (dlflat − gloss0, p99.5 of the
2-pass binomial-blurred difference over the face box, Octane's denoised
pass / Iray, enhanced-final 400) sat at Victoria 0.749, Kayo 0.797,
Gabriela 0.840, Nishi 0.739, where the plain Calib-DualLobe spheres read
1.06 - 1.12. Goal: find the component that differs, no constant fitted
(#19 / #57).

Scenes: the private Calib-Skin-<figure>-front twins without bump
(dlflatnb / gloss0nb) and without normal map (dlflatnn / gloss0nn) of
Victoria, Kayo, Gabriela, Nishi (Iray references from #19; Octane side
rendered here, `octane/t175-twins`, 400 samples, ~80 s a scene, 16 scenes
~25 min); the #174 renders (`t174-fix`, `f151-400`) for the full twins.
`face-highlight.py --sub` now pairs the twins too (-dlflatnb − -gloss0nb).

## 1. The bump: Octane follows Iray's

| dual lobe, Octane / Iray (blurred p99.5, denoised) | Victoria | Kayo | Gabriela | Nishi |
|---|---|---|---|---|
| both maps (#174 fix) | 0.749 | 0.797 | 0.840 | 0.739 |
| no bump (nb) | 0.798 | 0.852 | 0.838 | 0.782 |
| no normal map (nn) | 0.749 | 0.807 | 0.836 | 0.781 |

Iray's peak rises without the bump (Victoria +17 %, Kayo +45 %, Gabriela
+3 %, Nishi +10 %); Octane's rises with it (Kayo's Octane peak 0.0412 ->
0.0638, +55 %). The ratio moves 0 - 7 %: the baked bump spreads the lobe
as Iray's `file_bump_texture` does, and the gap stays without it. Not the
bump, not the pores.

The nn twins are no evidence about the normal map in Octane: the export
ignores the Normal Map channel's value (bakes every normal map at factor
1), so its nn renders still carry the map. Filed as #176 (MDL SDK
`base.mdl`: n = normalize(f x T_u + f y T_v + (|f z| + 1 − f) N)). Iray
says the normal map hardly moves the lobe (nn − both +0.4 - 5 %).

## 2. Where in the highlight: the skin sphere's quantiles

Each Calib-Skin scene has a sphere in the torso's skin beside the figure
(disc centre 785, 381, R 111 px; 0.9 R used): a smooth, known shape with
the same material and light. Octane / Iray of the dual lobe difference at
quantiles of the disc (blurred, denoised):

| quantile | 99.99 | 99.9 | 99.5 | 98 | 95 | 90 | 80 | sum |
|---|---|---|---|---|---|---|---|---|
| Victoria nb | 0.950 | 0.914 | 0.849 | 0.896 | 0.879 | 0.871 | 1.123 | 1.147 |
| Kayo nb | 0.957 | 0.943 | 0.908 | 0.836 | 0.824 | 0.870 | 1.234 | 1.022 |
| Gabriela nb | 0.960 | 0.948 | 0.901 | 0.825 | 0.811 | 0.896 | 1.277 | 1.051 |
| Nishi nb | 1.129 | 1.118 | 1.040 | 0.846 | 0.756 | 1.164 | 1.438 | 1.033 |

(With the bump within 0.01 - 0.09 of these.) Energy is right (sum 1.02 -
1.05, Victoria 1.15); the core is near Iray's, the shoulders 0.76 - 0.90,
the far skirt above. A narrower core with longer tails at the same peak -
a distribution's shape, not its width or energy.

## 3. The MDL and Octane: Phong against GGX

- Iray's lobes are `simple_glossy_bsdf`, Phong (Ashikhmin-Shirley, `libbsdf`
  #162) - near the peak Beckmann-like, D ∝ 1 − tan² θ / α².
- The skins are OpenPBR (505). Its specular and coating are GGX, no model
  pin (probe 98 on `NT_MAT_OPEN_PBR_SURFACE`: only `diffuseBrdf`). GGX at
  `phong_pin`'s α has Phong's D(0) (#95, #162) but falls twice as fast at
  the core, D ∝ 1 − 2 tan² θ / α², and keeps longer tails.

Analytic check, no fit: the sphere's lobes from the exported pins
(roughness / coatingRoughness, α = pin², weights `specular` / `coating`;
Victoria 0.542 / 0.300, 0.553 / 0.098; Kayo 0.544 / 0.399, 0.468 /
0.083; Gabriela 0.544 / 0.399, 0.85 / 0.15; Nishi 0.448 / 0.448), a
distant light 30° above the view, GGX / Phong of the same pins:

| quantile | 99.99 | 99.9 | 99.5 | 98 | 95 | 90 | 80 | sum |
|---|---|---|---|---|---|---|---|---|
| Victoria model | 0.992 | 0.941 | 0.835 | 0.860 | 0.819 | 0.747 | 0.940 | 1.028 |
| Kayo, Gabriela model | 0.998 | 0.982 | 0.928 | 0.820 | 0.757 | 0.755 | 0.972 | 1.029 |
| Nishi model | 0.998 | 0.979 | 0.914 | 0.764 | 0.695 | 0.900 | 3.2 | 1.028 |

The measured curves follow the model's shape, Victoria's kink at p99.5 /
p98 (her sharp lobe 2) included; what is left is a level of 0.95 - 0.96
(Victoria, Kayo, Gabriela) and 1.13 (Nishi) at the top. The face box's
p99.5 lands in the forehead highlight's shoulder (a few hundred pixels of
a small glint), the model's 0.76 - 0.86 band: the faces' 0.74 - 0.84.

The plain Calib-DualLobe spheres sit at 1.06 - 1.12 because that is a
disc mean under the HDRI (energy), and their sharp lobes (α ≤ 0.25) render
Beckmann on Universal (`phong_brdf`) - Phong's own shape.

## Where it stands

No export change. The gap is the skins' GGX against Iray's Phong at the
same peak and energy - #162's GGX EP / STD trade-off, on a node with no
choice. Options, asked in #175: keep OpenPBR's GGX (Iray's energy, a
highlight with narrower core and longer tails; Octane over Iray), or put
the skin's dual lobe on Specular Layers (139: `brdf` Octane / Beckmann /
GGX / GGX EP / STD) on OpenPBR's `layer` pin with OpenPBR's own specular
off - Iray's shape, but a new layering to verify, and Beckmann's rough
dielectric past α 0.25 is off under small lights (#172, the skins' lobe 1
is α 0.20 - 0.30). If #162 settles on STD, the skin would follow it this
way.

`calib/scenes/Calib-DualLobePores.duf` (generated, dumped, Octane side
`octane/t175-pores`, 2000 samples, ~2.2 min; DAZ dump 16 min, a cold
start): Uber's lobes over pore-sized noise (2048², blurred white noise,
features ~1 px on screen) - heights at 2 / 6, the same slopes as a normal
map at 1 and 0.5 (#176's check), both, the diffuse alone, both lobes. Not
needed for this finding (the faces' twins already show the bump right);
kept for #176 and a sphere-level pore check.

## Process

- Twins first: the nb pair cleared the bump in one 25-minute batch,
  before any new scene was worth rendering in Iray.
- The scene's own smooth sphere is a decomposition for free: quantile
  curves of a known shape say *where* in the lobe Octane differs (core,
  shoulder, skirt), which one percentile over a face cannot.
- A quantile curve's shape against a two-line analytic model (same pins,
  two distributions) names a distribution mismatch without fitting
  anything.
- Generated the pore scene before the twins' numbers were in; the twins
  made it unnecessary for #175. Read the cheap decomposition before
  building the next scene.
