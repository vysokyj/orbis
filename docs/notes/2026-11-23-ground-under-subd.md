# 2026-11-23 Auto ground under a SubD figure (#228)

Goal: DAZ's auto Iray ground (Ground Position Mode Auto) under a Genesis 9
figure standing on the floor sat 0.3 mm lower than the fixture test put it
(Calib-Hair-Pixie-T1-coat: DAZ -0.0962 cm, ours -0.0672). The issue guessed
DAZ measures the subdivided feet and the export the control cage. It is the
other way round.

## Measured (headless DAZ, `.mi` export, no render)
`daz-export-mi.dsa` now writes `<scene>.lowest.json`: per visible node with
geometry the lowest world y (cm, full precision) of its cached geometry as
the scene stands, of its world bounding box, and of its cached geometry at
base resolution (the resolution level switched to 0, as the scene dump
does).

| Scene | ground (.mi) | .mi mesh lowest | G9 cached (lod 1) | G9 box | G9 base |
|-------|--------------|-----------------|-------------------|--------|---------|
| Calib-Hair-Pixie-T1-coat | -0.0961936 | -0.0662 | -0.0951936 | -0.0951936 | -0.0951936 |
| Calib-SkinG9-F02-front | -0.0961936 | -0.0648 | -0.0691587 | -0.0951936 | -0.0951936 |
| Calib-SkinG9-F04-front / -M03-front | -0.0961936 | -0.0648 | -0.0691587 | -0.0951936 | -0.0951936 |
| Calib-Mi-SkinG9-Floor (new) | -0.0961936 | -0.0648 | -0.0691587 | -0.0951936 | -0.0951936 |
| Calib-Mi-SkinG9 (figure +1 cm) | -0.001 | | 0.9308 | 0.9048 | 0.9048 |

- DAZ's ground = the base-resolution minimum - 0.001 cm, to 1e-7: the
  control cage. The `.mi` itself holds the refined mesh (410 202 trilist
  vertices for Genesis 9), whose soles lie 0.29 - 0.30 mm higher; the
  viewport's refinement (cached at lod 1) lies between.
- The world bounding box equals the base geometry for Genesis 9; for the
  Pixie hair cap they differ (box 150.85, base 132.36) - no scene here
  decides between box and base on a node that is the lowest, the export
  takes the base (the cage), which is what the dump carries.
- Unrefined nodes (spheres, floors, strand hair: Calib-Env-Mi-GroundAuto-Low,
  Calib-Strand, Calib-Hair-Pixie-T1-nofig-nogloss-pr0-sunk): all three
  measures agree with the `.mi` mesh, as before.

## Rule
- Export: a refined mesh keeps its cage's lowest point (`Mesh::base_low`,
  over its shown faces) and `environment::ground_height` takes that instead
  of the refined points. Before, the export put the ground under the refined
  feet, up to 0.3 mm above DAZ's.
- Fixtures: `mi-oracle.py` writes `base_min_y` from `<scene>.lowest.json`;
  the ground test prefers it over `geometry_min_y`. New fixture
  Calib-Mi-SkinG9-Floor (Calib-Mi-SkinG9 with the figure where it stands,
  `make-skin-g9-scenes.py`); `base_min_y` added to the three fixtures with
  a ground below 0. `iray::DomeSetup::ground_y` unchanged (doc only).

## Dead ends
- Adding Calib-Hair-Pixie-T1-coat and Calib-SkinG9-*-front as whole fixtures:
  their eyes, lashes, mouth cavity and the shirt are Uber materials defined
  in DAZ's library, which the fixture tests don't read (299 arguments off) -
  the reason Calib-Mi-SkinG9 hides every mesh but the figure. Dropped;
  #232 (percent-encoded geometry ids) stays open, not needed for this.

## Open
- Box vs base on a node whose bounding box is lower than its geometry: not
  covered.
