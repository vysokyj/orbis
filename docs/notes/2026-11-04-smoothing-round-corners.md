# 2026-11-04 — Smoothing and Round Corners (#40)

Goal: map the material channels in DAZ's /Geometry/Smoothing group to
Octane. They are `Smooth On`, `Smooth Angle`, `Round Corners Radius`,
`Round Corners Across Materials` and `Round Corners Roundness`, on every
Iray shader (Uber, PBRSkin; PBRSkin's defaults are on, 89.9°, 0, on and 1).
Variant: enhanced-final, 512 samples.

## Scenes

`scripts/dev/make-smooth-scenes.py` copies Calib-Materials (eight spheres
under an HDRI) and swaps the spheres for its own low-poly shapes. Each
shape is 30 cm and wears a glossy grey Uber (roughness 0.2), so the HDRI's
reflections show the normals.

- **Calib-Smooth**:
  - S1 icosahedron, Smooth off; S2 at 30°; S3 at 45°; S4 at the default
    89.9° (icosahedron edges are 41.8°).
  - S5 icosahedron with two materials, split by x.
  - S6 cube at the default; S7 cube at 91°.
  - S8 12-sided cylinder (30° round the side, 90° at the caps).
- **Calib-RoundCorners**: cubes rotated 30° (all at roundness 1 and across
  materials on, unless noted):
  - R1 radius 0; R2 0.005; R3 0.01; R4 0.02; R5 1 (the unit check).
  - R6 radius 0.01, roundness 0.
  - R7 / R8 a cube with a blue top (a second material), radius 0.01,
    across materials on / off.

Both scenes dumped, `.mi` exported. Fixtures are in `calib/references/mi`.

## What DAZ hands Iray

- **Smoothing is baked into the geometry.** The trilists carry per-vertex
  normals, split where an edge is hard. The material gets no smoothing
  parameter. Vertex and normal counts (the export's in brackets, the same
  for all):

  | Object | .mi vertices | normals vs face |
  |---|---|---|
  | S1 Smooth off | 60 | 0° (faceted) |
  | S2 30° | 60 | 0° |
  | S3 45° | 12 | 37.4° (radial) |
  | S4 89.9° | 12 | 37.4° |
  | S5 two materials | 12 | smooth across the border |
  | S6 cube 89.9° | 24 | 0° |
  | S7 cube 91° | 8 | 54.7° (corner diagonals) |
  | S8 cylinder | 50 (26 positions) | 0 – 15° (side smooth, caps split) |

- **Round Corners is a material function.** Every Uber shader has
  `"geometry_bump" = <shader>.rounded_corner_normal_N`, a call of
  `mdl::state::rounded_corner_normal(float,bool,float)`. The arguments are
  the channels as they are: `radius` 0.005 for 0.005 and 1 for 1, and
  `across_materials` / `roundness` only when set. Radius 0 (the default)
  passes no radius, which leaves the function off. The scene options carry
  `mdl_meters_per_scene_unit 0.01`. The MDL spec gives the radius in meters
  in world space. DAZ's own MDL never converts units.

## Attempts

1. **Per-face angle rule** (a corner averages the faces at its vertex
   whose normal lies within the angle of the face's own): S3 (45°) split
   into faceted corners, unlike DAZ's 12 normals. Faces across an
   icosahedron vertex lie about 70° apart, although each edge is 41.8°.
2. **Edge rule** (an edge is soft within the angle, and faces joined
   through soft edges around a vertex share a normal; union-find over face
   corners): this gives the `.mi`'s counts for all eight objects. Kept.
3. **First Octane render**: S7 (91°, 8 smooth normals) rendered as a sharp
   cube, and S3 / S4 looked faceted. A Lua probe listed the mesh nodes'
   attributes. There was **no `normals` array at all**: Octane's USD import
   ignored our `normals` attribute and computed its own at the import's
   `maxSmoothAngle` (45°). That held for every export so far.
4. **`primvars:normals`** (vertex interpolation, or faceVarying with
   `primvars:normals:indices`): the mesh nodes now carry `normals` and
   `polyNormalIndices` (S8: 50 normals, 120 indices). The render shows S1 /
   S2 faceted, S3–S5 smooth, the S6 cube sharp, S7 blobby and the S8
   cylinder smooth round its side with crisp caps. This matches DAZ.
5. **Regression check** of the switch (Octane had used its own 45°
   normals until now), against earlier enhanced-final renders. Mean |Δ|
   per pixel, 0–255:

   | Scene | mean \|Δ\| | p99 | mean level old / new |
   |---|---|---|---|
   | Calib-Hair (G8 figure, SubD) | 0.33 | 3.7 | 97.6 / 97.6 |
   | Calib-L-Distant-45 (cubes, sphere) | 0.15 | 2.0 | 44.1 / 44.1 |

   This is the level of noise.

## Round Edges in Octane

The node-type probe found Round Edges = 137, on the `roundEdges` pin
(P_ROUND_EDGES 467). Its pins are `roundEdgesMode` (enum: off, fast,
accurate, accurate convex only, accurate concave only),
`roundEdgesRadius` (0–100), `roundEdgesCurvatureRoundness`,
`roundEdgesSampleCount` (4–16, default 8) and
`roundEdgesConsiderOtherObjects`.

Mapping: Accurate, radius = DAZ's value as meters, roundness as is,
8 samples, other objects off. This is on every leaf material: Universal,
Standard Surface / OpenPBR, the parts of the mixes, NVIDIA metal and
dielectric.

What the render shows (Calib-RoundCorners):

- R2–R4: the edge highlight widens from 5 mm to 2 cm.
- R5 (1 m): the whole cube is shaded as a blob.
- R6: a flat chamfer band.
- R7 = R8: Octane always rounds across the material border.

## Against Iray (second session, same day)

The Iray batch came in (5000 samples, `calib/references/iray`). Octane:
enhanced-final, 1024 samples, exported from the My Library scenes.

**Calib-Smooth**: Octane renders every shape as Iray does: S1/S2 faceted,
S3–S5 smooth, S6 a sharp cube, S7 blobby, S8 smooth round the side with
crisp caps. Per object (Octane / Iray level, mean |Δ| per pixel 0–255):

| | S1 | S2 | S3 | S4 | S5 | S6 | S7 | S8 |
|---|---|---|---|---|---|---|---|---|
| level | 1.024 | 1.024 | 1.030 | 1.029 | 1.026 | 1.056 | 1.059 | 1.053 |
| \|Δ\| | 2.5 | 2.7 | 3.3 | 3.0 | 2.5 | 4.4 | 4.9 | 3.8 |

The whole frame is 1.017 in both scenes, so the 2–6 % is the general
level, not smoothing. The edge rule stands.

**Calib-RoundCorners**:

1. **Unit: meters.** Iray's R5 (radius 1) is a smooth blob over the whole
   30 cm cube. At 1 cm it would look like R3 as we export it. The export
   was right.
2. **R2–R4** (5 mm – 2 cm): the edge highlights widen alike in both. **R6**
   (roundness 0): a flat chamfer band in both.
3. **R7 / R8**: Iray keeps the blue top's border sharp with across
   materials off (R8), and R7 has a highlight along it. Octane had both
   rounded, R8 even brighter. Fix: cut a material with across materials
   off (and a radius) out of its mesh into a prim of its own
   (`split_round_corner_borders`, after the normals). Octane's Round Edges
   with other objects off then has nothing across the border to round
   against. The re-render shows R8's border sharp, as Iray's.
4. **R5 is not matched**: Octane draws creases along the face diagonals
   where Iray is smooth. Round Edges with a radius larger than half the
   object breaks. That is no realistic setting, so it is left as is.
5. On the left faces of R3/R4/R6, Octane shows a sharp-edged darker
   rectangle where Iray has a soft gradient. It is most likely the
   neighbouring cube's reflection, which Octane renders sharper at
   roughness 0.2. It is not the rounding (R1, unrounded, doesn't face a
   neighbour), and it is outside #40.

| | R1 | R2 | R3 | R4 | R5 | R6 | R7 | R8 |
|---|---|---|---|---|---|---|---|---|
| level | 1.024 | 1.026 | 1.039 | 1.055 | 1.038 | 1.052 | 1.031 | 1.038 |
| \|Δ\| | 2.7 | 2.8 | 4.0 | 4.6 | 5.2 | 4.6 | 3.0 | 3.2 |

Timings: Octane 88 s (RoundCorners), 73 s (Smooth) at 1024 samples.

## Still open

- How DAZ weights faces in a smoothed normal (area vs. angle). The test
  shapes are symmetric and can't tell; it doesn't show in any render.
- Across materials off when only one side of a border has it: Iray
  decides per shading point (the other side still rounds), the cut
  stops both. Not tested (needs another Iray render, not worth one).

## Process lessons

- Probe what Octane imported before trusting the USD you wrote: the
  normals had been silently dropped since the first export.
- A test shape whose edges sit between two angles settles the rule
  better than a sphere (icosahedron: 41.8°, tested at 30° / 45°).
- `octane-render.sh` wrote its job list before taking the render lock.
  Two sessions waiting at once swapped jobs (this run rendered another
  session's Calib-CoatBump). Fixed in #74.
- A session that builds into the shared cargo target dir overwrites the
  others' binary. `CARGO_TARGET_DIR` in the scratchpad (and
  `cargo build --release --examples`, for `orbx_ocs`'s tone LUT) keeps a
  run on its own code.
