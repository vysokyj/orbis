# 2026-11-05 — Node flags against DAZ's .mi (#36)

The rest of #36 after [2026-10-30](2026-10-30-node-flags.md): Render
Priority, Cast Shadows on instanced meshes, hidden surfaces and facet groups
on a figure. Everything checked against DAZ's .mi export first.

## Scenes

Built in a headless DAZ by `scripts/dev/daz-build-node-flags.dsa`, then
`make-node-flag-scenes.py` (camera, SubD level, copies into `calib/scenes`
and My Library):

| Scene | What | For |
|-------|------|-----|
| Calib-Mi-Priority | Calib-L-Distant-45, Render Priority Lowest / Low / Below Normal / Above Normal / Highest on the five meshes | .mi only |
| Calib-CastShadows-Off | Calib-L-Distant-45, Cast Shadows off on the sphere and on an instance node of it at x 120 | .mi + Iray reference |
| Calib-Shell-G9 | Genesis 9 (DAZ's free base figure) on the plane, a geometry shell (red diffuse) with Body and Arms shown, facet groups `l_forearm` and `l_pectoral` off; render SubD 1 | .mi + Iray reference |

Ways that didn't work at first:
- `MainWindow` doesn't exist headless (no action manager): the shell is
  made by hand, `new DzGeometryShellNode()`, its `Shell Node` set to the
  figure. Its `material_group_*_vis` / `facet_group_*_vis` channels appear
  only after `update()` / `finalize()`.
- The first shell wasn't parented to the figure: `orbis` skipped it
  ("shell without a host"). Every shell in My Library (64 scenes) is its
  host's child — DAZ's Create > New Geometry Shell does that — so the
  script parents it (`addNodeChild`); the export stays as it is.
- The first figure lookup took the last `DzFigure` (the eyebrows, two
  surfaces); now the one labelled Genesis 9.
- At DAZ's render SubD 2 the scene had 2.3 M faces (Octane froze on 1.2 M
  in a series); at 1: 708 k.

## Render Priority and Cast Shadows: not passed

`DzMIExporter` output, normalised (ids dropped), against Calib-L-Distant-45:
Calib-Mi-Priority is **identical**; Calib-CastShadows-Off differs only by
the instance node's own `instance` block. Every mesh instance, the
shadowless sphere and instance included, has `visible on`, `shadow 3`,
`transparency 3`, `finalgather 3`, … and `label` 0. Render Priority, an
enum DAZ hides in the UI, is a 3Delight leftover.

NVIDIA's Iray manual (4.3.5): Iray Photoreal ignores `shadow_cast` and
`shadow_recv` (only Iray Interactive uses them). So the Cast Shadows
mapping of 2026-10-30 (an object layer with `shadow_visibility` 0) made
Octane drop a shadow that Iray draws; it is gone. Instanced meshes needed no
probe then. Calib-CastShadows-Off's Iray reference should show both
spheres' shadows.

`mi-oracle.py` now writes `objects`: each mesh instance's flags and
attributes; an instance node's copy no longer re-lists its target's
materials under its own name. The 143 older fixtures come out the same
apart from that section (checked, not rewritten). Test:
`render_priority_and_cast_shadows_dont_reach_iray` (src/iray.rs).

## Hidden surfaces and facet groups

A figure has neither `material_group_*_vis` nor `facet_group_*_vis`:
DAZ creates them only on a geometry shell (`/Shell/Visibility/Surfaces`,
`/Shell/Visibility/Face Groups`, one per surface / polygon group of the
host). In the .mi the shell is its own object with all seven surface
materials listed; its triangles are only the shown faces:

| Object | .mi triangles | export (USD) triangles |
|--------|--------------:|-----------------------:|
| Genesis 9 | 804 992 | 804 992 |
| G9Shell | 343 776 | 343 776 |

So DAZ cuts a hidden surface and a hidden facet group away (a face goes
when either is off), as the export does. Iray reference pending (#36).

## Open

Iray references of Calib-CastShadows-Off and Calib-Shell-G9 (one batch with
the other pending scenes).
