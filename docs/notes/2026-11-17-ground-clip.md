# 2026-11-17 Geometry below Iray's ground (#69)

Goal: the user's decision on #69 (2026-11-16) - Faithful cuts the geometry
below Iray's ground, Enhanced keeps Octane's see-through shadow catcher.
A `Tuning` field, `clip_below_ground`.

## What Iray does (docs, no fit)

- Iray manual 12.3 / 4.4: the implicit ground is a matte object - opaque,
  it shows the environment in the ray's direction weighted by its shadow;
  geometry below it is hidden from camera and secondary rays.
- References already in git show it twice:
  - Calib-L-Mi-Dome-turned: a Manual ground at 12 cm hides the floor (at 0)
    and the sphere's lower half.
  - Calib-Env-Mi-GroundAuto-Instance: the instance copy down to -60 cm under
    the Auto ground (-0.001 cm) is gone. In this camera the floor at 0 hides
    it from above anyway, so the scene doesn't tell camera visibility apart.
    It still shows the Auto case matters: only instance copies can be below
    an Auto ground (`ground_height` counts a mesh's own points, not copies).

## What the export does

`usd::ground_clip::clip_below_ground`, run on the meshes before the layer is
written, when the shadow catcher is (HDRI + Draw Ground, the Ground shadow
option on) and the Look is Faithful:
- faces across the plane clipped polygon by polygon (Sutherland-Hodgman
  against one plane); a cut point per edge (keyed by its two point indices,
  so neighbouring faces share it and the mesh stays closed), UVs and
  faceVarying normals per edge and corner pair (UV seams keep both sides);
  unused points dropped, subsets remapped;
- strands split where they cross; each run above is a strand, its end
  interpolated (point, width);
- instanced meshes judged per copy: above stays a copy, below dropped,
  across baked in world space (normals by the inverse transpose, faces
  turned round under a mirroring copy) and clipped as a mesh of its own.

## Renders (Octane Final, 400 samples, denoised; Iray 5000)

| scene | variant | lower 40 % mean Octane / Iray | correlation (lower 40 %) |
|---|---|---|---|
| Calib-L-Mi-Dome-turned | Faithful | 0.86 | 0.46 |
| Calib-L-Mi-Dome-turned | Enhanced | 1.55 (the floor shows) | 0.15 |
| Calib-Env-Mi-GroundAuto-Instance | both | 1.04 | 1.00 |

Faithful Dome-turned now has Iray's layout: no floor, the sphere a half
dome on the ground, the environment seen through the catcher. Its level
(0.86) is the whole frame's - the sky band (top 30 %) is 0.86 too - so it is
not the cut. The ground band's lower correlation is the shadow catcher's own
shading and the blurred HDRI there, unchanged by this issue.

## Open

- A camera below the ground: Iray shows the ground from below (Ground
  Visible from Below), the cut export shows the cut. Not covered.
- Spot / point lights and volumes below the ground are not cut.
- Sun-Sky with Draw Ground: the export writes no catcher, so nothing is cut;
  no reference shows what Iray hides there.
