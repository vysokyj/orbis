# 2026-11-13 PBRSkin Detail Normal Map as an OSL blend (#140)

Split from #54. PBRSkin's Detail layer tiles a tangent-space normal map
(DAZ's default 10 × 10; every Genesis 9 base skin has one at Detail Weight
1) and blends it onto the base normal. The roughness half went in with
#54; this is the normal.

Scene: `calib/scenes/Calib-PBRSkin-Occlusion.duf` (generated for #54), sphere
O8: the synthetic dimples normal map (`calib-coatbump-dimples-n.png`),
4 × 4 tiles, Detail Weight 0.5, no base normal. Nothing new generated.

## What the MDL does (read, no fit)

`pbr_skin.mdl`'s `pbr_detail_normal`: the base normal is the Normal Map
and the bump blended by `base::blend_normals` (or either alone); under
`detail_enable` the detail normal is blended onto that by
`base::blend_normals` at `detail_normal_weight = detail_weight`. NVIDIA's
`base` module docs: the weight is a linear blend between `state::normal()`
and the given normal. The blend itself is reoriented normal mapping
(already in `detail_normal::blend_normals`, #85). The Top Coat
(`pbr_add_clearcoat`) blends only its own bump onto `state::normal()` - it
doesn't take the detail.

DAZ's `.mi` (fixture from #54) for O8: `normal_detail` =
`tangent_space_normal_texture` of the map at gamma 1, uvw transform scale
4 (the Detail Tiles), `detail_weight` 0.5. The gamma 1 is PBRSkin.dsf's
`default_image_gamma` for the channel (an unlisted image takes it, #91);
the PBRSkin fixture test resolved no gammas until now, so the channel
defaults' gammas went into `calib/references/daz-defaults/PBRSkin-gammas.json`
(`daz-defaults.py --gammas`) and the test resolves them as the export
does. `iray::pbrskin` now carries `pbr_detail_normal.normal_detail` and
`.detail_weight`; the fixture agrees (700 arguments).

Also in the definition: `Detail Normal Map Mode` (1 = normal map; 0 reads
it as a height map) and `Detail Horizontal / Vertical Offset`. Mode 0 is
not exported (none in the library); the offsets follow the material
offset's rule on a map (only offset 0 has a fixture).

## Bake or procedural

- **Bake** (`detail_normal.rs`, as the Uber height maps): the result has
  to hold the detail at its own texel density, so the base map at tiles ×
  its resolution: a 4K skin map at 10 × 10 tiles of a 1K detail is 40K
  square - far over `texture_budget.rs`'s budget, and a capped bake keeps
  exactly what the detail isn't.
- **Procedural**: Octane has no node that blends two normal maps (manual
  mirror: Mix / Composite textures mix colours, not normals), but the
  OSL texture does any per-point maths on texture inputs, and an OSL
  output on `normal` is already proven to read as a tangent-space normal
  (the retro-reflective normal, 2026-11-05). Two image textures as inputs,
  each with its own UV transform, the MDL's blend in the shader: exact,
  no memory, full detail. Chosen; no trade-off left for the user.

## Probes

1. First export: O8's shading normal (Info kernel with and without
   bump/normal mapping, `octane-probe-bumptilt.lua`, `bump-tilt.py`)
   tilted **0.00°** RMS - nothing. `octane-probe-oslpins.lua` (new): the
   OSL node loads, compiles, has its `base` / `detail` texture pins, but
   both hold a default RGB colour node - the image node from the `.ocs`
   was dropped.
2. Octane's own `vectron.orbx` (an OSL geometry with inputs) writes its
   dynamic pins as `<pin name='Iterations' dynamicType='3'>`. With
   `dynamicType='5'` (texture) on ours, `detail` loads with the image
   (`34 RGB image`).
3. Known input: the same scene with the OSL node replaced by the dimples
   map pre-blended at weight 0.5 and tiled 4 × 4 by hand (2048², numpy,
   swapped into the package with `examples/orbx_ocs`). O8 tilt OSL 8.61°,
   baked 8.62°, per pixel 0.18° RMS apart (8-bit and interpolating before
   vs after the weighting); the other seven spheres 0.00. The RNM onto a
   non-flat base is the same formula as `detail_normal::blend_normals`
   (unit-tested); not rendered separately.

Probe timings: 1 – 2 s per Info render (16 samples).

## Where it stands

Exported: under Detail Enable, mode 1, weight > 0 - `normal` = OSL
`OrbisDetailNormal` of the base (the baked Normal Map + height map, the
normal map alone, or flat) and the tiled detail. Octane side of
Calib-PBRSkin-Occlusion rendered (enhanced-final); its Iray reference is
in #54's batch (5000 samples) - the resume compares O8's highlight
texture and level against O1 / O7 (same roughness detail, no normal).

Process: an OSL node's input pins need `dynamicType` in the `.ocs` - any
future OSL texture with inputs needs it (now in `docs/octane/octane-x.md`).
