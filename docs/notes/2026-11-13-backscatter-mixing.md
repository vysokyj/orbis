# 2026-11-13 Backscattering under Base Mixing 1 and 2 (#155)

Goal: `backscatter_weight` returned 0 for any Base Mixing but 0 ("not
calibrated there", since e753db9, #29), so Octane dropped the
backscattering lobe in glossiness (1) and weighted (2) mixing although DAZ
passes it (Calib-MetalTL-Back K7's `.mi`, #145). Map it from the MDL
stack, no constant; Iray parked.

## Premise checked on the current code

- The gate was the only reason: Base Mixing 0 and 1 go through the same
  layered branch (`metallic_layered_base`, #143), and with Base Mixing 1
  `has_metal` is false (DAZ drops Metallic Weight), so the lobe's Mix
  amount is b, the surface's shares (b, 0, 1) - nothing new to derive.
- Base Mixing 2 was dispatched past the backscatter Mix to
  `weighted_mix_material`, whose gloss part had no lobe.

## Docs first (irayubermaterial.mdl, `normalized_base`)

- `bottom_bsdf = glossy_enable ? basic_glossy_bsdf : bsdf()`.
- `bsdf_w_backscattering = weighted_layer(backscattering_weight,
  backscattering_bsdf, bottom_bsdf)`.
- `glossy_base_bsdf = weighted_layer(refraction_weight, refraction,
  bsdf_w_backscattering)`, mixed at Glossy Weight against the diffuse
  part (`normalized_mix`).
- So the lobe is a layer of the gloss part only, under the refraction;
  the diffuse part keeps none. Not a normalised weight beside Diffuse /
  Glossy Weight.

## DAZ's `.mi` (Calib-WeightedBack, new fixture)

| sphere | backscattering_enable | glossy_enable | note |
|---|---|---|---|
| B1 b0 | - | true | control |
| B2 b0.5 | true | true | roughness 0.09 |
| B3 b1 | true | **false** | lobe alone in the gloss part |
| B4 b0.5 d0.5 | true | true | diffuse_weight 0.5 |
| B5 b0.5 g0 | **false** | - | no gloss part: colour at the default 0.5 |
| B6 rough 0.7 | true | true | roughness 0.49 |
| B7 mode 1 | true | true | 1 - Backscattering Glossiness 0.7 -> 0.09 |
| B8 mode 0 | true | true | tie |

Backscattering Color passed (unshared) though the channel is hidden in
the generated material. B5 failed `src/iray.rs` (it enabled the lobe):
weighted mixing without gloss has no gloss part to hold it - the only
change there, proven by the fixture. V6 (Calib-Mi-WeightedRefr2, r 0.5 +
b 0.5) already showed the lobe under a partial refraction.

## Octane side

- Base Mixing 1: the gate lifted; the lobe's roughness falls back to
  1 - Backscattering Glossiness when no `.mi` path covers the material.
- Base Mixing 2: the weighted Mix's gloss part is now
  `weighted_backscatter_part` - the metal alone without a lobe, the lobe
  alone at weight 1, else a Mix at the weight (its map) of the lobe
  (`backscatter_node`, split out of `backscatter_mix_material`) and the
  metal, both at the gloss part's scale; the partial-refraction glass Mix
  holds it as its `material2`. Displacement test cases added.
- Partial refraction in modes 0 / 1 still drops the lobe (the old
  `Refraction Weight > 0` gate) though DAZ passes it: filed #157.

## Renders

Octane enhanced-final of Calib-WeightedBack and Calib-MetalTL-Back
(K7 now with its lobe), see the issue; Iray of both in the batch.
