# 2026-11-13 Translucent metal under backscattering, Base Mixing 1 (#145)

Goal: the leftover of #123 - where a translucent metal stayed one
Universal node (under the backscattering Mix, Base Mixing 1) it still
took all of its translucency as lost (the diffuse x (1 - t), #109), the
rule #123 showed failing on thin metal (Calib-PBRSkin-MetalTL M4 0.83,
M7 0.68 x Iray).

## Premise check (code, MDL, `.mi`)

- **Base Mixing 1 with a plain Metallic Weight**: the premise was stale.
  `metallic()` reads `crate::iray`, which passes 0 in modes 1 and 2, so
  such a surface was already a translucent dielectric.
- **Base Mixing 1 with a Metallic Weight map**: the premise held. The
  metal test also looked at `mat.image("Metallic Weight")`, so a map
  still counted as a metal: no transmission, and the diffuse dimmed by
  t, though `metal_w` was 0 (no metal written). DAZ drops the map too
  (Calib-MetalTL-Back K8's `.mi`: no `metallic_weight`, `metallic_enable`
  false). Fixed: `metallic_map` / `has_metal` ask `crate::iray`.
- **Under backscattering**: `metallic_layered_base` stacks the metal
  over the backscattering, which lies over diffuse + gloss + the
  translucency (docs/daz/iray-uber-mdl.md). The backscatter Mix already
  takes b (1 - w) and leaves the surface M = w^2 / (1 - a) metal and
  B = (1 - w)(1 - b) / (1 - a) base (#108). Fix: that surface becomes
  the translucent metal's two Mixes at those shares instead of one
  node, outer amount x = 1 - B and metal layer y = M / x (x y = M,
  1 - x = B; at b 0 it gives (w, w), #123's Mix). Derived from the
  stack, no constant. Under a metallic map the backscatter amount and x,
  y are baked per texel (the backscatter amount used to take the
  channel's value while the surface's shares were already per texel).
- **Base Mixing 1 backscattering**: DAZ passes it (K7's `.mi`), but the
  export drops backscattering outside mode 0 (`backscatter_weight`).
  That is a separate gap, filed as #155.
- `.mi` side: the Shown twin found DAZ omitting `backscattering_anisotropy`
  at 0 (its MDL default). `uber_default` now has that entry, so iray.rs
  matches both fixtures.
- #135's gap (a partial refraction over translucency staying one node in
  Base Mixing 1 / PBRSkin / nested Mixes) is not closed by this: under
  a backscattering Mix there is no refraction (backscattering needs
  none), and Base Mixing 1's refraction-vs-translucency single
  transmission is unchanged.

## Scene

`Calib-MetalTL-Back` (make-calib-scenes.py, closed spheres, red base
0.45 / 0.05 / 0.04, Translucency Color 0.9 / 0.6 / 0.4, gloss 0.2,
backscatter roughness 0.3): K1 metal 0.3 / t 0.5 / b 0.5, K2 without
backscatter, K3 without metal, K4 without translucency, K5 metal 0.75,
K6 a grey map (x 0.75), K7 Base Mixing 1 twin of K1, K8 Base Mixing 1
with the grey map and t 0.5. `-Shown` twin for the `.mi` only.

## Octane side

enhanced-final, rendered after the fix
(`~/Library/Caches/orbis/calib/octane/enhanced-final/Calib-MetalTL-Back.png`);
no Iray reference yet, parked in #145's `## Iray batch`. Regression: the
`project.ocs` of all 259 calibration scenes that have a dump, exported with
HEAD and with the fix, are byte-identical except Calib-MetalTL-Back.
That covers Calib-PBRSkin-MetalTL (its reference stays at 1.04 - 1.07),
Calib-Backscatter and Calib-MetalRefr-TL(-Solid). In the new scene, K1 / K5 / K6
are now Mix(backscatter, Mix(x, Mix(y, metal, black), translucent base)),
and K8 transmits its translucency (before: no transmission, the diffuse
x 0.5).

Process: the all-scenes `.ocs` diff (an orbx_ocs pass over every dumped
calib scene with both binaries) settles "nothing else moved" without a
render.

## Open

The Iray comparison (#145's `## Iray batch`); Base Mixing 1 / 2
backscattering (#155).
