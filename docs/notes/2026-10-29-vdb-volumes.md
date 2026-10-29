# 2026-10-29 — VDB volumes (#21)

Until now the export left out every node whose material renders an OpenVDB
file: Octane drew its carrier box as an opaque white wall (Gladiator's haze
cube). This session exports the volume itself.

## What the scenes use

The user's three scenes with a VDB (Gladiator, Grungeon, prison) all use
the same product: DimensionTheory's Atmospheric Skies "DTAS - Optimized
Haze". It is a tiny box node (±0.005 cm, scale 0.1, Y −10000 cm) with
DAZ's built-in **Simple OpenVDB MDL Volume** shader on
`DTAS_HazeLayer.vdb` (Houdini 20.5, one float grid `density`, 54 MB).

The shader's brick graph is:

```
MDL Surface (clear_surface)
  → daz_3d::irayubermaterial::add_inhomogeneous_volume(
        coefficients: base::lookup_volume_coefficients(density texture_3d,
            scattering_multiplier, absorption_multiplier,
            scattering_offset, absorption_offset,
            density_multiplier, density_relative_to_size),
        sub_surface_scattering_direction)
  + base::volume_blackbody_emission (Emission Enable, off)
```

`lookup_volume_coefficients` (MDL SDK `base.mdl`) computes:

- σs = scattering_multiplier · d · D + scattering_offset · D
- σa = the same with the absorption channels

Here d is the grid value and D is Density Multiplier. Relative to size, D
is multiplied by `transform_scale(object, world)`.

DTAS's values are: Scattering 1, Absorption 0.0902, Density 1000, relative
to size, SSS Direction 0.85, Clip To Geometry off.

The VDB header gives a uniform voxel of 3000 units and an index bbox of
x/z −279…279 and y 0…143. Its active voxels have values up to 3.4·10⁻⁷
(mean 7.8·10⁻⁸; 25 M voxels, all loaded by Octane).

Read as DAZ cm and placed by the node, it is a slab 1.67 km wide that runs
from −100 m to +329 m.

## Octane's nodes (probe)

- **Volume (91).** Its attributes are `filename` and `scaleUnitType` (4 =
  m). Then `volumeAbsorptionId` / `volumeScatterId` / `volumeEmissionId`
  (grid names) and `volume*Scale` (applied to the grid as it loads; set
  later from Lua, they change nothing).
- **Pins of the Volume node:** `medium` and `objectLayer`. A saved project
  keeps only the file name and the grid names; Octane reads the voxels when
  it loads the file.
- **Volume medium (98).** Pins `scale` ("Density", max 10⁴),
  `rayMarchStepPercent` (1 = 100 %), `absorption` (with `invertAbsorption`,
  on by default), `scattering`, `phase` (60, `scattering_direction`) and
  emission.
- **Transform value (67)** takes a whole matrix (`transform`, type 13, 3×4
  rows) and derives rotation, scale and translation from it.

The exported volume opens in Octane as a 559 × 144 × 559 grid with a
3000-unit voxel at −837000. Placed at 0.001 (cm → m with the node's 0.1),
it sits where Iray has it.

## Calib-VDB

`scripts/dev/make-vdb-scenes.py` builds the Calib-VDB scenes: Calib-HDRI
plus Gladiator's haze node. These are local scenes (DAZ Store content).

| variant | change |
|---|---|
| base | as DTAS |
| d100 / d10000 | Density 100 / 10000 |
| noabs | Absorption 0 |
| abs | Scattering 0, Absorption 1 |
| norel | not relative to size |
| tint | Scattering (1, 0.6, 0.3) |
| iso | SSS Direction 0 |

The measure is region means in linear light, each divided by the same
renderer's haze-free Calib-HDRI. The regions are: sky (top right), horizon
(far left, above the floor), below (the strip under the floor, distant
ground).

### Iray

- **Relative to size divides by the node's scale.** norel renders as d100,
  so DTAS's density 1000 at scale 0.1 acts as 10000. (The MDL multiplies by
  `transform_scale(object, world)`; DAZ's object space evidently makes that
  1/0.1.)
- **Absorption 0.09 barely counts.** noabs renders as base: 1.22 / 1.31 /
  1.22.
- **SSS Direction is applied.** iso is brighter than base (sky × 1.08,
  below × 1.32), even though the brick never wires
  `sub_surface_scattering_direction_enable`.
- **Thick haze is bright.** d10000 gives sky 2.14 and below 2.48.

### Octane, the dead ends

1. The density pin × grid scale was first set at Iray's σ × 100, reading
   Iray's coefficients as per cm. The result was too thin (sky 1.04 against
   Iray 1.22), and it did not grow with density: d100, base and d10000 all
   came out 1.13–1.17.
2. Split between the density pin (≤ 10⁴) and the grid scales, it was the
   same.
3. The haze lifted above the camera (Octane and Iray alike) is invisible
   looking up. The haze sits in the slab's bottom layers, so the extent was
   not the problem.
4. **The step.** Lengthening the step 4× or 10× made the haze *thinner*:
   abs went from 0.84 / 0.81 to 0.96 / 0.95. At 1 % and at 0.1 % the
   result was the same, now nearly opaque: 0.035 / 0.00 / 0.00. Octane
   takes the percentage of the VDB's own voxel (3000 units), not the placed
   one (3 m). At 100 % one or two steps crossed the whole haze, and every
   "match" before this was an artefact of sampling the haze at one point.
5. At a converged step (1 %), the grid scale was scanned on abs:

   | grid scale | sky | horizon | below |
   |---|---|---|---|
   | 100 (σ × 100) | 0.035 | 0.00 | 0.00 |
   | 10 | 0.18 | 0.00 | 0.12 |
   | 3 | 0.53 | 0.06 | 0.49 |
   | **1** | **0.797** | **0.357** | **0.786** |
   | Iray | 0.79 | 0.35 | 0.77 |

   **Iray's coefficients go to Octane unchanged, per meter.** All three
   regions match at once, so the geometry and the profile agree as well.

## The export

- **`volume::scene_volumes`.** For a visible node whose material has a
  Volume File, it finds the file in the content directories, takes the
  grid ("Volume Channel", else the file's first grid from the header,
  `vdb.rs`) and the transform (from the dump). It computes:
  - σs = Scattering Multiplier · D
  - σa = Absorption Multiplier · D
  - D = Density Multiplier ÷ node scale, when relative to size

  The multipliers are decoded from sRGB like every DAZ colour (see the
  results). The anisotropy is SSS Direction. The box itself is not drawn. Emission
  and the offsets are not exported; emission is reported.
- **`.orbx`.** The `.vdb` goes into the package once as
  `assets/vdb/NN_<file>`. A placement (4) is added to the "DAZ 3D lights"
  group: a transform (67) holding the node's matrix in meters, and a Volume
  (91) in meters with a Volume medium (98) inside it:
  - density = min(peak σ, 10⁴), the grid scales take the rest;
  - colours σ / peak;
  - `invertAbsorption` off;
  - phase g = SSS Direction;
  - step = 4 placed voxels (here 0.004).

## Results

The step was 10 voxels and each render had 200 samples. Each cell gives
Iray / Octane, as a fraction of each renderer's haze-free render. The hue
column gives the share of r, g and b in the sky region, Iray / Octane.

| variant | sky | horizon | below | sky hue r, g, b |
|---|---|---|---|---|
| base | 1.22 / 1.17 | 1.31 / 1.20 | 1.22 / 1.09 | equal |
| abs | 0.79 / 0.80 | 0.35 / 0.36 | 0.77 / 0.79 | equal |
| iso | 1.40 / 1.35 | 1.61 / 1.49 | 2.15 / 1.92 | equal |
| noabs | 1.22 / 1.20 | 1.31 / 1.31 | 1.22 / 1.13 | equal |
| norel, d100 | 1.02 / 1.02 | 1.04 / 1.04 | 1.02 / 1.01 | equal |
| tint | 1.08 / 1.09 | 1.04 / 1.06 | 1.08 / 1.04 | 0.83, 0.93, 1.24 / 0.79, 0.94, 1.27 |
| d10000 | 2.14 / 1.62 | 1.53 / 0.88 | 2.48 / 1.29 | bluer in Iray |

The multipliers were then decoded from sRGB, tested by editing the
exported colours:

| variant | sky | horizon | below | sky hue r, g, b |
|---|---|---|---|---|
| base | 1.22 / **1.20** | 1.31 / **1.30** | 1.22 / **1.12** | equal |
| tint | 1.08 / 1.07 | 1.04 / 1.07 | 1.08 / 1.05 | 0.83, 0.93, 1.24 / **0.83, 0.92, 1.25** |

Raw, Iray's noabs equals base (absorption 0.09 makes no difference), but
Octane's does not. Decoded (0.0086), base comes out as noabs. The tint's
hue lands on Iray's. The export decodes them.

Render times at 200 samples were 3–6 minutes per Calib-VDB scene, and 12
for d10000. At a 4-voxel step base took over 20 minutes for 400 samples,
which is why the step is 10.

## Open

- **Thick haze.** d10000 (σ up to 3.4 / m) stays darker and less blue than
  Iray's bright fog. Octane's multiple scattering may be cut by the path
  depth (maxDiffuseDepth 8, maxScatterDepth 128). This is not an issue for
  DTAS's own values.
- **Not exported:** emission, the coefficient offsets, Clip To Geometry.
