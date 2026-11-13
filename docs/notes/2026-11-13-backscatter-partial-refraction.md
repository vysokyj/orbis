# 2026-11-13 Backscattering under a partial refraction (#157)

`backscatter_weight` returned 0 for any Refraction Weight > 0, so a Base
Mixing 0 / 1 surface with a partial refraction lost its backscattering
Mix. #154 had shown DAZ switches `backscattering_enable` off only under a
*full* refraction; nothing covered the partial case in modes 0 / 1.

## MDL
`irayubermaterial.mdl`, `metallic_layered_base` (read, not copied): the
stack is `weighted_layer(refraction_weight, refraction_bsdf,
weighted_layer(metallic_weight, metallic_bsdf,
weighted_layer(backscattering_weight, backscattering_bsdf, bottom)))`.
Unambiguous: the refraction covers the backscattering, so the lobe keeps
(1 - refraction)(1 - metal) b. Same nesting as #129's refractive metal.

## .mi
Generated `Calib-Mi-RefrBack` (.mi only, closed: refraction 0.3 / 0.7 x
backscattering 0.5 / 1, modes 0 and 1, translucency, a translucent metal,
shared inputs), its `-Shown` twin, and the Iray scene `Calib-RefrBack`
(ties without backscattering / without refraction, r 0.5 x b 0.5 / 1, mode
1, closed and thin-walled, metal 0.6; unshared inputs, green lobe over a
red base). One headless export, three fixtures. Every sphere with
0 < refraction < 1 and b > 0 gets `backscattering_enable` true, in both
modes, closed or thin; b 1 switches the diffuse off as without refraction
(Q2, Q4, R4). Mode 1's roughness (1 - 0.7)^2 = 0.09 as before. Hidden and
shown alike. `src/iray.rs` already said so: no change, the fixtures now
prove it.

## Octane side
- `backscatter_weight` follows `crate::iray`'s `backscattering_enable`
  (the channel's full-refraction check without an Uber).
- Dispatch: backscattering under a partial refraction goes through
  `refractive_metal_material` first (refraction part at Refraction Weight,
  metal 0, no lobe since its refraction is 1), its base part (refraction
  0) then takes the backscattering Mix as without refraction. No constant.
- `displacement_is_written_once_on_the_outermost_material`: modes 0 / 1
  under a partial refraction (2 Mixes), a full refraction (plain node).

Calib-RefrBack rendered enhanced-final; Iray parked (#157).
