# 2026-11-11 Hidden instance target (#130)

Goal: #125 found that DAZ's .mi of Calib-Env-Mi-GroundAuto-InstanceHidden
(the sphere Visible off, an instance node of it) has neither the sphere
nor the copy, while ORBIS kept a hidden node as its instances' prototype
and drew the copy (spec: "UltraScatter hides its sources"). Open: does an
instance group (UltraScatter's `DzInstanceGroupNode`) behave the same, and
does Visible in Render off differ from Visible off?

## Scenes
`scripts/dev/daz-build-instance-scenes.dsa` (headless, base
Calib-Tone-Default): the sphere "M1 tinted" and copies 60 cm in front of
the row.
- DzInstanceGroupNode API (probed, undocumented): `getTargetControl()`,
  `beginItemEdit()` / `createItem()` / `finishItemEdit()`;
  `DzInstanceGroupItem.setPosition(x, y, z)` (a `DzVec3` gives
  "Insufficient arguments"). The items survive a save and a reload: the
  saved scene's .mi equals the live one.

| Scene | Target | .mi objects |
|-------|--------|-------------|
| Calib-Mi-InstGroup | shown | 10: the sphere + 2 `SphereGroup` items |
| Calib-Mi-InstGroup-Hidden | Visible off | 8: neither |
| Calib-Mi-InstGroup-NoRender | Visible in Render off | 8: neither |
| Calib-Mi-Instance-NoRender | Visible in Render off (instance node) | 8: neither |
| Calib-Env-Mi-GroundAuto-InstanceHidden (#125) | Visible off (instance node) | 8: neither |

## UltraScatter
Demo-Gladiator's sources (HFGG Tree 01 - 12, Grass 01 - 03) are Visible
on, no Visible in Render channel; its local .mi has each source tree as an
object beside its `_Ins_` copies. The grass sources sit at y -200 cm,
under the terrain. So nothing in the project's scenes relies on a hidden
source.

## Fix
A hidden node (the same `visible` as any node: Visible, Visible in Render,
a hidden group) is skipped with its copies; `placements` always puts the
source first at identity; `Mesh::prototype_only` and its ground exception
are gone (a hidden target is no longer in `meshes`). Exports after the fix:
InstGroup 2 instances, the three hidden variants and -InstanceHidden 0
("M1 tinted: hidden in the scene"), GroundAuto-Instance 1.

No Iray render needed: the .mi decides what DAZ hands Iray.
