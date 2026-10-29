# 2026-10-29 — Wearables in a stale fit (#51)

In the user's teens scene the Octane render (export of the displacement
check) differed from Iray's in the wardrobe. The difference was the same
with displacement and without it:

- the right-hand girl's toes came through her Comfy Shorts Slip Ons;
- the man wore one boot, and it stood off his foot;
- the blonde's stockings looked missing and her hair white.

## What it was

Not the export: its meshes matched the dump to the point (bounding boxes
equal to 0.1 cm), and the dump matched what DAZ's `getCachedGeom()` gave
right after the scene was opened.

The fault was DAZ's cached geometry itself. A diagnostic script opened
teens headless and measured the mean distance from wearable vertices to
the nearest foot vertex:

| step | boots | slip-ons |
|---|---|---|
| opened | 11.1 cm (74 % of them > 5 cm) | 2.6 cm |
| `processEvents()` | 11.1 | 2.6 |
| man, then boots `finalize()`, `forceCacheUpdate(false)` | 1.7 | 2.6 |
| every node `finalize()`, `forceCacheUpdate(true)` | 1.7 | 1.2 |

The dump's `baseResolution()` switches every mesh to resolution level 0
first. After that switch, one pass of `finalize()` and
`forceCacheUpdate(true)` left both at 11.1 / 2.6 cm. Only
`processEvents()` followed by a second pass fitted them (1.8 / 1.2 cm). A
side view of the slip-ons on her feet showed the toes above the shoes'
uppers in the stale fit, in DAZ's render-level cache as in its base mesh.
The boots stood flat on the floor while the man's foot was lifted.

So `baseResolution()` now runs twice: `processEvents()`, then
`finalize()` and `forceCacheUpdate(node, true)` on every node.

## Check

A fresh dump and export of teens, with the distance from each shoe
surface (face centres of the export's GeomSubsets) to the skin, signed by
the skin's normals:

| surface | before: median, under skin | after: median, under skin |
|---|---|---|
| Slip Ons Foot (upper) | 0.61 cm, 39 % | 0.49 cm, 0 % |
| Slip Ons Ankle_Trim | −0.49 cm, 67 % | 0.41 cm, 0.2 % |
| Slip Ons Toes | 2.33 cm, 15 % | 1.40 cm, 0 % |
| Boots Upper | 6.94 cm, 9.8 % | 1.05 cm, 3.7 % |
| Boots Sole | 16.9 cm, 0 % | 2.28 cm, 1.4 % |

In Octane (teens at 200 samples, and Calib-Feet-Teens, a copy with the
camera 110 cm from her feet, at half resolution):

- both of the man's boots are on his feet;
- the slip-ons cover her feet as in Iray.

A small patch of skin still shows at the heel of the man's back boot. DAZ
renders his body at SubD level 3 and the export refines it once; that is
not this bug.

## Not wardrobe

- **The stockings were there.** Side by side with Iray they match: a dark
  sheer leg at opacity 0.2 under opaque bands. Reading them as missing was
  my mistake.
- **The blonde's hair** (Morgan Hair, card hair on Iray Uber with a cutout
  map) renders white in Octane and honey blond with strong highlights in
  Iray. That is a material question: #52.

## Aside

`orbis export --dump <file>` keeps the dump as the scene's cached one by
*moving* it. Exporting a copy of teens with teens' cached dump took that
dump away from teens. Pass a copy.
