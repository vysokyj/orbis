# 2026-11-16 Faithful skin dual lobe on Specular Layers (#175)

Follows [the dual lobe's shape](2026-11-16-skin-dual-lobe-shape.md): the
Calib-Skin faces' dual lobe sat 0.74 - 0.84 x Iray because OpenPBR's
specular and coating are GGX only, where Iray's lobes are Phong. The
user's decision (2026-11-16, on #175): split by Look as #162 split the
rough dielectric lobes - Enhanced keeps OpenPBR's GGX, Faithful puts the
skin's dual lobe on Specular Layers (139) at STD on OpenPBR's `layer`
pin, OpenPBR's own specular off. Through the existing
`Tuning::rough_lobe`, no new field; no constant.

Scenes: the private Calib-Skin-<figure>-front `-dlflat` / `-gloss0`
twins of Victoria, Kayo, Gabriela, Nishi (Iray references from #19);
faithful-final, 400 samples, Octane's denoised pass, ~35 - 47 s a scene.
Calib-PBRSkin, -Metal, -Makeup and Calib-DualLobe: Faithful exports
compared byte for byte before / after.

## 1. The pin

Probe 98 on `NT_MAT_OPEN_PBR_SURFACE`: 59 pins, the last `layer` (type
33, default node 144, "Material layer above this base material"). A
Material Layer Group (144) takes `pin_count` and dynamic `Layer N` pins,
as the Layered material (143) does.

## 2. Which `Layer N` is on top

The manual doesn't say. Variant of the Kayo export with both layers
strong (specular 1, IOR 10): one sharp (roughness 0.15, Beckmann), one
rough (0.8, STD), then swapped. Face box, linear luminance:

| Layer 1 / Layer 2 | p90 | mean |
|---|---|---|
| sharp / rough | 0.612 | 0.294 |
| rough / sharp | 0.173 | 0.109 |

The rough layer lights the face only in Layer 2: the upper layer hides
the lower one behind its Fresnel. `Layer 1` is the bottom. Lobe 1 goes
there, lobe 2 (the small weight) on top, as OpenPBR's coating (lobe 2)
lay over its specular (lobe 1).

## 3. First export: `phong_brdf` per layer

Each layer at `phong_brdf` of its own lobe: Beckmann under α 0.25, STD
past it (as Universal's dual lobe). Dual lobe (dlflat − gloss0, 2-pass
binomial blur, p99.5 of the face box) and its sum over the box, Octane /
Iray:

| | Victoria | Kayo | Gabriela | Nishi |
|---|---|---|---|---|
| lobes (α) | 0.29 / 0.09 | 0.30 / 0.16 | 0.30 / 0.16 | 0.20 / 0.20 |
| OpenPBR GGX (Faithful before), p99.5 | 0.819 | 0.891 | 0.854 | 0.764 |
| layers, `phong_brdf`, p99.5 | 0.878 | 0.978 | 0.987 | 0.922 |
| OpenPBR GGX, energy | 1.262 | 1.197 | 1.062 | 1.030 |
| layers, `phong_brdf`, energy | 1.181 | 1.181 | 1.043 | **0.773** |

The peaks moved as the model said; Nishi, both lobes at Beckmann, lost
a quarter of the lobe's energy. Beckmann on Universal keeps sharp dual
lobes at Iray's level (Calib-DualLobe 1.06 - 1.12), so this is the
Specular Layer's Beckmann (or Beckmann under the face's normal map).
Filed as #190; not decomposed here.

## 4. Shipped: STD for both layers

The decision named STD. Both layers at the Look's model (`brdf` 7):

| | Victoria | Kayo | Gabriela | Nishi |
|---|---|---|---|---|
| dual lobe p99.5 | **0.873** | **0.966** | **0.976** | **0.894** |
| dual lobe energy | 1.276 | 1.220 | 1.074 | 1.048 |
| full face p99.5 (dlflat) | 0.929 | 0.954 | 0.982 | 0.952 |
| full face, OpenPBR GGX before | 0.910 | 0.928 | 0.912 | 0.894 |

(gloss0 faces 0.961 / 0.942 / 0.970 / 0.989, unchanged.) STD on the
sharp lobes 2 costs Victoria / Kayo / Gabriela ~0.01 of p99.5 against
Beckmann and adds 0.02 - 0.10 energy; Nishi's energy is back to 1.05.
Energy over the face stays where GGX had it (Victoria and Kayo's 1.2 -
1.3 is there in every model, so not the lobe's). The re-export equals
the rendered STD variant (`.ocs` identical).

## 5. What didn't move

- The skin sphere beside each figure is a prop: Universal, not OpenPBR.
  Its quantiles are the same before and after (Faithful: Universal at
  STD, core 1.04 - 1.07, shoulders 0.85 - 0.96, skirt 1.29 - 1.51,
  energy 1.04 - 1.27). Correction to the shape note's §2: that sphere
  was Universal at GGX EP in Enhanced, not OpenPBR - GGX all the same,
  so its conclusion holds, but it doesn't test the OpenPBR path.
- Calib-PBRSkin, -Metal, -Makeup: PBRSkin stays on OpenPBR (its MDL
  lobes are GGX, #186): Faithful exports identical. Calib-DualLobe
  (Universal) identical.
- Enhanced and both Preview rows: unchanged (Preview skin is Universal,
  already at `phong_brdf` with the Look's model).

## Where it stands

Shipped: Faithful's Uber skin dual lobe on Specular Layers at STD
(`RoughLobe::skin_dual_lobe_layers`, `Ocs::skin_dual_lobe_layers`). Open:
#190 (Beckmann in a Specular Layer), #186 / #188 (PBRSkin's lobes and
rim). Not checked: a skin with a Top Coat or a glossy layer over the dual
lobe (one blended layer, over the coating where Iray has it under) - no
Calib-Skin twin has one.

## Process

- The order of a layer stack is a two-render check with exaggerated
  layers (IOR 10, opposite roughnesses); faster than reading docs that
  don't say.
- Measure energy next to the peak: the first export's peaks all looked
  right, the face-box sum showed Nishi's lost quarter.
- Check which material a test patch actually renders on before calling
  it a decomposition (the "skin sphere" was a Universal prop).
- Byte-identical exports are the cheapest proof that a path is
  untouched; no render for Calib-PBRSkin.
