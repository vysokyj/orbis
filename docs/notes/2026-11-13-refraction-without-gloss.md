# 2026-11-13 Refraction without gloss per Base Mixing (#143)

Goal: #101's worker read in `irayubermaterial.mdl` that `normalized_base`
calls `refraction_base(..., true, true, 1, ...)` - the refraction's
reflection fixed on - and suspected that a Base Mixing 1 glass without a
glossy layer still reflects in Iray while the export writes `specular` 0.
No render: the MDL and DAZ's `.mi` decide it.

## Docs first

- `base_layer_mixing` has two values: `blm_layered` (0) and
  `blm_normalized` (1). DAZ's three Base Mixing modes map 0 and 1 to
  `blm_layered` (`metallic_layered_base`) and only 2 (Weighted) to
  `blm_normalized` (`normalized_base`) - already verified by every
  `.mi` fixture (`src/iray.rs`, `base_mixing`). So the issue's premise
  ("Base Mixing 1 goes through `normalized_base`") is wrong; the question
  moves to Base Mixing 2.
- `metallic_layered_base` passes DAZ's `refraction_reflection_enable`,
  `full_refraction_reflection_enable` and `glossy_layered_weight` on.
- `normalized_base` fixes them (true, true, 1) - but its refraction sits
  inside the gloss part: `weighted_layer(refraction_weight, refraction,
  backscatter / glossy)`, mixed against the diffuse at Glossy Weight by
  `normalized_mix` (or alone when the diffuse is off).

## The `.mi` (Calib-Mi-RefrNoGloss, -Shown)

Eight Uber spheres, refraction at IOR 1.5, Glossy Layered Weight 0 and
Glossy Weight 0, closed unless said. One headless DAZ run, both scenes,
`export-mi.done` clean; `mi-audit.py` 264 checked, 0 disagree.

| Sphere | Mixing | DAZ hands Iray |
|--------|--------|----------------|
| N1 | 0 | refraction 1, `refraction_reflection_enable` false, diffuse off |
| N2 | 1 | the same (`blm_layered`) |
| N3 | 1, thin walled | the same |
| N4 | 1, refraction 0.5 | refraction 0.5, flag false |
| N5 | 2 | **no refraction** (`refraction_enable` / `refraction_weight` not passed), diffuse on, no flags; volume and `abbe_number` 0 passed |
| N6 | 2, thin walled | no refraction |
| N7 | 2, refraction 0.5 | no refraction |
| N8 | 1, Glossy Specular black, layered weight 1 | refraction 1, flag on, `full_refraction_reflection_enable` true; `glossy_reflectivity` not passed (0.04) |

The -Shown twin (every `visible` key dropped) gives the same arguments.

`src/iray.rs` disagreed in 24 arguments: it passed the Base Mixing 2
refraction (12 args over N5 - N7 x 2), and gave N8 `glossy_reflectivity`
0 (2). Fixed: under weighted mixing the refraction counts only with
Glossy Weight > 0 (Abbe from the channel still); in Base Mixing 1 the
reflectivity stays at 0.04 where the diffuse is off (a full refraction
leaves `glossy_enable` off; Calib-SpecMode's S2, black specular on a
diffuse sphere, keeps its 0).

## What it means for the export

- Base Mixing 0 and 1 without gloss: no reflection in Iray - the export
  is right (`specular` 0; Faithful's IOR 1 on a thin wall). Nothing
  changes. Morgan Hair (Base Mixing 1, #99, #135, #145) has a glossy
  layer; its refraction reflects through `metallic_layered_base` as #101
  / #120 found - unaffected. `uber_glass_level` (#118) reads the same
  flags DAZ passes; its case selection stands. `SPECULAR_MIN_IOR` is not
  touched (it applies under refraction < 0.5 only; N8's full refraction
  takes the Refraction Index).
- N8: the export already writes the full Fresnel at the Refraction Index
  (white `specular`, weight 1: a black Glossy Specular's hue is white).
- Base Mixing 2 without gloss: Iray renders a **diffuse** sphere where the
  export wrote glass. The export now takes the Refraction Weight from
  `crate::iray` (`refraction_weight`), so the surface is the plain
  diffuse DAZ hands Iray. Unit test: no transmission pins.

Not done here, filed: Base Mixing 2 *with* gloss and a full refraction
goes through the weighted Mix (a metal at Glossy Weight's share), while
`normalized_base` with the diffuse off returns the gloss part alone -
the refraction over everything (#154).

No Iray confirmation: the `.mi` drops the refraction outright, the MDL
then has nothing but `dl_diffuse_bsdf`; no reading in doubt.

## Process

- The `.mi` question was answered by an existing fixture column
  (`base_mixing`) before any scene: reading `base_layer_mixing` first
  would have closed half the issue at once.
- `src/iray.rs`'s test needs the scene in `calib/scenes` (it reads the
  `.duf`); copy before running it.
