# 2026-11-13 — Displacement in Mix and Layered materials (#144)

#102's review found that the flake and backscattering Mixes wrote a
displacement node on the Mix and the Universal inside wrote its own. Since
then more Mix constructions had come (translucent / refractive metal,
PBRSkin Makeup, the Layered coat over a skin), each deciding on its own.
Question: which displacement does Octane use - the Mix's, the
sub-materials', both added? Is anything visibly wrong, or only waste?

## Docs

The OTOY manual's Mix material page says nothing about displacement; the
Layered material page lists only the base material and the layers;
Composite lists a Displacement pin. Not enough: measured.

## Pins (probe 144, `octane-probe-pins.lua` with Mix / Layered / Composite)

- Mix (19): `amount`, `material1`, `material2`, `displacement`,
  `customAov`, `customAovChannel`.
- Layered (143): `material1`, `customAov`, `customAovChannel` (+ the
  dynamic `Layer N`). **No `displacement`.**
- Composite (138): has `displacement`.

## Render variants

`scripts/dev/orbx-disp-variants.py` on Calib-Disp (enhanced-final export,
64 samples, denoised pass): P2 (white map, 0..5 cm, `amount` 0.05 = +5 cm)
replaced by Mix / Layered constructions of its own Universal, with or
without its displacement node. P2's width along row 575 (over the floor):
91 px undisplaced, 124 px at +5 cm (3.3 px/cm, the 2026-10-29 note's 3.2);
+10 cm would be ~157 px.

| variant | where the displacement is | P2 width |
|---|---|---|
| base | on the Universal (as exported) | 124 |
| mix-top | on the Mix only | 124 |
| mix-sub | on both sub-materials only | **91** |
| mix-m1 | on `material1` only | **91** |
| mix-both | 5 cm on the Mix and on both subs | 124 (not doubled) |
| mix-conflict | 5 cm on the Mix, 10 cm on the subs | 124 (the Mix's) |
| mix-nested | 5 cm outer Mix, 10 cm inner Mix | 124 (the outer's) |
| lay-base | Layered, on its base Universal | **91** |
| lay-mix | Layered over a Mix carrying it | **91** |
| wrap1 | Mix at amount 1 around the Layered, on the Mix | 124, shading 0.2 / 255 of base |
| wrap0 | the same at amount 0 | `material2`'s default diffuse: amount 1 = all `material1` |

So Octane uses the outermost material's displacement only; nothing inside
a Mix counts, and a Layered material can't carry one at all.

## What it meant for the export

- Flakes, backscattering, weighted mixing, the weighted coat, translucent
  and refractive metal, Makeup: the outermost Mix wrote it, the nodes
  inside wrote more (ignored) - waste: image and displacement nodes, the
  same texture file (the budget counts files once, `Textures::asset`).
- **Lost**: the anisotropic Top Coat / coat over gloss over dual lobe
  (Layered material outermost: only its base wrote it, ignored), and a
  full unmapped Makeup over a metal without Makeup Metallicity Enable
  (its metal / diffuse Mix outermost wrote none; its leaves did, ignored).

Rule now (`open_sub_materials` / `close_sub_materials` /
`leaf_displacement_pin`): one flag, `nested`, set while a Mix's or
Layered material's sub-materials are written; the outermost Mix writes the
displacement after its sub-materials, a leaf only when not nested, and a
displaced Layered coat gets a Mix at amount 1 around it. Test
`displacement_is_written_once_on_the_outermost_material` walks 18
constructions (plain, weighted mixing, flakes, backscattering, both, the
weighted coat over them / over weighted mixing / over a translucent metal,
translucent metal, refractive metal, partial refraction over a translucent
metal, anisotropic coat alone / over flakes / over a skin, skin, Makeup,
Makeup over a metal, full Makeup over a metal) and asserts one
displacement pin on the outermost node; on the old code it failed at
flakes (2).

## Effect on the calibration scenes

All 258 `calib/scenes` with a cached dump exported before and after: the
project.ocs of every one byte-identical - no calibration scene has a
displaced material under a Mix or Layered material, so there is nothing to
re-render; the Iray references stay valid, no Iray needed.

## Process

- An orbx variant of an exported scene settled it in two short batches
  (9 + 3 renders, ~10 s each) - the uniform displacement maps of
  Calib-Disp make the silhouette width the measurement.
- Width along a row over a uniform floor (threshold) is robust; the
  bottom edge read the contact shadow and the first gradient scan was
  wrong both ways.
