# 2026-11-14 Translucent metal under backscattering, compare (#145)

Resume of #145 with Calib-MetalTL-Back's Iray reference in (91b1388, 5000
samples, DAZ Studio Iray, Post Denoiser on@8): does the translucent
metal's two-Mix construction under the backscattering Mix (b30b17b,
`backscatter_metal_amounts`, no constant) hold?

Octane: enhanced-final, 400 samples, denoised pass, re-exported from the
cached dump with HEAD (after #87, #114, #134 and the #155 / #157
backscatter changes). For the before/after, the same dump exported with
b30b17b^ (the one-node rule, translucency taken as lost) in a temporary
worktree and rendered beside it (`__prefix`). Numbers: linear luminance
(Y) over 0.6 R of each projected disc (`sphere_discs.py`), RGB over
r < 0.5 (centre) and 0.6 - 0.9 (rim). Floor 1.04 in both. Octane ~35 s
per render.

Note: the export finds the dump only by the library path
(`My Library/Scenes/Calib-MetalTL-Back.duf`); the `calib/scenes` copy
(byte-identical) said "no scene dump". `calib-variants.sh` takes the
scene name for that reason.

## Octane / Iray

| sphere | Iray Y | pre-fix | now | now, centre R G B | now, rim R G B |
|---|---|---|---|---|---|
| K1 metal 0.3 tl 0.5 back 0.5 | 0.044 | 0.94 | **1.05** | 1.04 1.01 0.96 | 1.13 1.15 1.13 |
| K2 metal 0.3 tl 0.5 (no backscatter) | 0.024 | 1.07 | 1.06 | 1.02 1.06 1.04 | 1.03 1.21 1.25 |
| K3 tl 0.5 back 0.5 (no metal) | 0.092 | 1.20 | 1.20 | 1.16 1.20 1.18 | 1.17 1.23 1.22 |
| K4 metal 0.3 back 0.5 (no translucency) | 0.076 | 1.19 | 1.19 | 1.20 1.19 1.16 | 1.17 1.18 1.19 |
| K5 metal 0.75 tl 0.5 back 0.5 | 0.034 | 1.15 | **1.18** | 1.17 1.18 1.17 | 1.14 1.23 1.30 |
| K6 grey map tl 0.5 back 0.5 | 0.094 | 0.43 | **1.16** | 1.15 1.15 1.12 | 1.17 1.20 1.19 |
| K7 Base Mixing 1, K1's channels | 0.103 | 0.34 | 1.10 | 1.09 1.10 1.10 | 1.09 1.13 1.15 |
| K8 Base Mixing 1, grey map, tl 0.5 | 0.032 | 0.51 | **1.01** | 0.92 1.06 1.02 | 0.98 1.21 1.24 |

(Iray Y: Rec. 709 luminance of the centre RGB.)

- **The construction holds.** K1 / K5 / K6 land between K2 (the plain
  translucent-metal Mix, 1.06, as Calib-PBRSkin-MetalTL's 1.04 - 1.08)
  and K3 / K4 (the backscatter over one part, 1.19 / 1.20). K3 / K4
  are the backscatter lobe's own level: Calib-Backscatter's w0.5 sphere
  is 1.15 under #73's rule, and neither K3 (no metal) nor K4 (no
  translucency) goes through `translucent_metal_material`. So the
  ~+15 % on K3 - K6 belongs to the lobe, not to #145's shares.
- **The fix mattered most under a map.** K6 0.43 → 1.16: the one-node
  rule both took the translucency as lost and gave the backscatter the
  channel's amount instead of b (1 − w(x)) per texel. K1 0.94 → 1.05
  (the lost translucency, about a tenth). K5 (metal 0.75: the base is
  small) barely moves, 1.15 → 1.18, as the shares predict.
- **Base Mixing 1.** K8 0.51 → 1.01: a Metallic Weight map no longer
  counts as a metal, the sphere is the translucent dielectric DAZ hands
  Iray. K7 0.34 → 1.10 is #155's (Base Mixing 1 backscattering, now
  passed by 359451d) and its gain over K1 is the dropped metal; reported
  there.
- Rims: the spheres with translucency and no backscatter (K2, K8) are
  1.21 - 1.25 in green / blue at the rim with red at 1.0 - 1.03; the
  same pattern as the closed translucent spheres of #134 / #163 (the
  translucency colour's hue at grazing), not this issue's.

## Process

- The pre-fix render, from a detached worktree at b30b17b^ built into its
  own target dir and exported from the same dump, cost one 35 s render
  and turned "lands with its parts" into a before / after per sphere.
- Read K3 / K4 against the lobe's known level (spec, #73) before reading
  K1 / K5 / K6 against them: otherwise the +15 % looks like this issue's.

## Where it stands

#145 closed. K7 → #155. The backscatter lobe's +15 % at w0.5 (#73's
Beckmann stand-in) is unchanged and known.
