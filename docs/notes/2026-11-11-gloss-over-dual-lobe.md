# 2026-11-11 — Glossy layer over the dual lobe (#106)

Goal: keep Iray Uber's dual lobe specular when the glossy layer is on.
Before, `universal` took the glossy branch and never reached the dual
lobe, and `skin_gloss` filtered `DualLobe::of` by `glossy <= 0` — found by
code review, not by a render.

## What Iray does (docs first)
DAZ's `irayubermaterial.mdl`, `metallic_layered_base`: the dual lobe is a
custom-curve layer over the diffuse (`add_dual_lobe_specular` →
`dl_diffuse_bsdf`), and the glossy layer is `add_top_coat` in custom-curve
mode with **that** as its base. Both are in the stack at once, each at its
own roughness. The `.mi` of the new Calib-DualLobeGlossy confirms DAZ hands
both over (`dual_lobe_uniform_specular_weight` 1 and `glossy_enable` true
on G3 – G8); the scene's fixture is in `calib/references/mi`, and
`src/iray.rs` reproduces it unchanged.

A second bug on the way: `roughness_source` returns Specular Lobe 1
Roughness whenever a dual lobe is on, so the gloss — when it replaced the
dual lobe — was rendered at lobe 1's roughness, not Glossy Roughness (the
weighted mix's gloss metal too).

## Mapping (no fitted constant)
Iray's order kept: the dual lobe on Universal's / OpenPBR's `specular`,
the gloss on the `coating` above it. The specular takes the dual lobe as it
already did under a Top Coat (full share, both lobes at the ratio's
roughness blend, IOR above r 0.5); the coating the glossy layer's weight ×
min(2r, 1) (Universal) or weight + F0's IOR per texel (OpenPBR skin), its
own roughness at the Phong pin, the base's bump on the coating pins. A Top
Coat over both moves into the Layered material (as the anisotropic coat
does) so the coating stays free. Left out: metal, refraction, Base Mixing
1 / 2, OpenPBR skin with a Top Coat (one coat) — the gloss alone there, now
at its own roughness. Weighted mixing's dropped dual lobe: #121.

How often: in the installed People presets 348 materials set a dual lobe,
15 of them also a glossy layer, one of those a Top Coat (presets save only
what they change, so a lower bound).

## Scenes
- `calib/scenes/Calib-DualLobeGlossy.duf` (make-calib-scenes.py): G1 gloss
  only, G2 dual lobe only (the decomposition), G3 both, G4 gloss 0.4, G5
  sharp gloss over a broad lobe, G6 Glossy Reflectivity 1, G7 under a Top
  Coat, G8 dual lobe at W 0.5.
- Calib-Skin-Base-hdri-coat0 / -dlgloss / -dlglosstc (make-skin-variants.py,
  private: stay in My Library): the Genesis 8 base skin's gloss alone, with
  a dual lobe added, and with it under the skin's Top Coat (the OpenPBR
  fallback).

Octane side (enhanced-final) rendered for all four; no Iray references yet,
so no ratio table. Dump and render: ~20 s per skin dump, ~2 min per render.

## Process
- `calib-variants.sh` finds a dump by the path it was dumped from: a scene
  dumped from `calib/scenes` must be rendered by that path, not by its name
  in My Library (the first run said "export FAILED (no cached dump?)").
- `make-calib-scenes.py` does not copy into `calib/scenes`; copied by hand.

Open: the Iray batch in #106 decides whether the order (gloss over the dual
lobe) and the blended lobes hold; G1 / G2 against G3 tell whether the two
add as Iray's layering does.
