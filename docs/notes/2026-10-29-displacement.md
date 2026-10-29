# 2026-10-29 — Displacement (#24)

The export ignored displacement until now. In the user's scenes 63 scenes
have a displacement map at a non-zero strength. They are mostly the
genital and clothing add-ons, hair (OOT's Fine* caps) and props. Of the
distinct maps, 32 are 8-bit RGB JPEG, plus one TIF and two PNGs (the
others are in libraries that are not installed).

## Iray

The channels are Displacement Strength (with the map), Minimum and Maximum
Displacement (cm, defaulting to −0.1 and 0.1) and SubD Displacement Level.
They are the same ids in Iray Uber and PBRSkin; DAZ Default uses
`displacement` / `_min` / `_max` (#41).

In the MDL, `irayubermaterial.mdl` only multiplies `displace_direction ·
displace_amount`. DAZ's brick computes the amount.

### Calib-Disp

Calib-Disp has eight Uber spheres with *uniform* synthetic maps
(`/Runtime/Textures/ORBIS/calib-disp-*.png`, ours). A uniform map pushes
the whole sphere along its normals, so the silhouette measures the amount.
The radius is read from the sphere's bottom edge against the floor, at
3.2 px/cm. The edge spheres (P1, P8) and P6 (where the edge meets its
shadow) misread on that edge, so I checked them on the image.

| sphere | map, range, strength | Iray Δr |
|---|---|---|
| P1 | none | 0 |
| P2 | white 0..5 | +5 cm |
| P3 | white 0..5, s 0.5 | +2.5 cm |
| P4 | black −5..5 | −4.7 cm |
| P5 | grey 0..10 | +2.2 cm |
| P6 | white 0..5, s 2 | ≈ +9 cm (image) |
| P7 | white −5..5 | +5 cm |
| P8 | grey −5..5 | smaller (image) |

So the amount is Strength × lerp(Minimum, Maximum, map). A grey of 128 is
read as ≈ 0.22, not 0.5.

### Calib-Disp-Map: how the map is read

All spheres run over 0..10 cm at strength 1:

| sphere | map | Δr | as a map value |
|---|---|---|---|
| M2 | grey 128, L PNG | +2.2 cm | 0.22 |
| M3 | grey 128, RGB PNG | +2.2 cm | 0.22 |
| M4 | grey 64 | +0.6 cm | 0.06 |
| M6 | grey 128, JPEG | +2.2 cm | 0.22 |
| M7 | grey 128, `map_gamma` 1 | **+5 cm** | 0.50 |
| M8 | white | +10 cm | 1.0 |

With no gamma of its own (`map_gamma` 0, "automatic"), DAZ decodes an
8-bit map at 2.2: 0.502^2.2 = 0.22 and 0.251^2.2 = 0.048. The Uber
channel's `default_image_gamma` 1 does not change that. The user's scenes
set their maps by `image_file`, with no image_library entry, so they are
automatic.

It holds for the other float channels too (#49, below).

## Octane (probe)

- **Texture displacement (80).** Pins `texture`, `black_level` (mid),
  `quality`, `levelOfDetail`, `amount` (height), `displacementDirection`,
  `filterType`, `filtersize` and `clamp`.
- **Vertex displacement (97).** Pins `texture`, `amount`, `black_level`,
  `displacementMapType` (1 = height), `textureSpace`, `inputAxes`, `bump`
  (auto bump), `displaceRestPosition` and `subdLevel` (the GPU subdivides).

Vertex displacement is Iray's model: vertices moved along the normal, on a
mesh subdivided by a level. It is what the export uses:

- `amount` = Strength · (Maximum − Minimum) · 0.01 m;
- `black_level` = −Minimum / (Maximum − Minimum);
- the map at gamma 2.2, or at the image's own `map_gamma`;
- `subdLevel` = SubD Displacement Level on figures, capped at 2, and 0 on
  props.

## Results

The Octane silhouettes at 200 samples, against Iray's, measured the same
way (bottom-edge row in px):

| scene | Iray | Octane |
|---|---|---|
| Calib-Disp | 609 623 615 592 614 677 623 608 | 609 623 615 592 614 673 623 608 |
| Calib-Disp-Map | 609 614 614 609 670 614 623 639 | 609 614 614 609 624 614 614 638 |

- **Equal to the pixel.** Only M5 differs, where Iray's edge is the floor
  shadow and Octane's +5.3 cm matches 0.753^2.2. P6 differs by the same
  kind of shadow artefact.
- **M7 (map_gamma 1)** was +2.2 cm before the export read the image's
  gamma (Octane at 2.2). It reads it now: `Channel::gamma` comes from
  `image_library`.

## Open

- **A figure.** Checked in the afternoon, see below.
- **16-bit maps.** TIF maps and DAZ's automatic gamma for 16-bit images
  are untested; they are read at 2.2 too.
- **DAZ Default** (`displacement` ids): #41.

## Data maps (#49)

Calib-DataGamma has eight Uber spheres with the same grey 128 map:

- G1 to G3: Cutout Opacity as a map, the map at `map_gamma` 1, and a
  plain 0.5;
- G4 to G6: Metallic Weight the same way;
- G7 and G8: Glossy Roughness as a map and the map at `map_gamma` 1.

Sphere-centre mean, linear:

| sphere | G1 | G2 | G3 | G4 | G5 | G6 | G7 | G8 |
|---|---|---|---|---|---|---|---|---|
| Iray | 0.131 | 0.177 | 0.168 | 0.110 | 0.088 | 0.081 | 0.105 | 0.097 |
| Octane, maps at 2.2 | 0.135 | 0.183 | 0.173 | 0.139 | 0.120 | 0.111 | 0.118 | 0.109 |
| Octane, maps at 1 (before) | 0.145 | 0.183 | 0.174 | 0.127 | 0.120 | 0.111 | 0.119 | 0.109 |

- **Iray decodes all three at 2.2.** The automatic map renders apart from
  the plain 0.5, the `map_gamma` 1 map renders as it, and the roughness map
  comes out glossier.
- **The export now does the same** (`data_gamma`): `float_or_map`, the
  specular map of Glossy / Dual Lobe Reflectivity, the glossy weight map
  and displacement.
- **The ratios to the plain 0.5 move towards Iray's:**
  - opacity: 0.78 now, Iray 0.78, before 0.83;
  - metallic: 1.25 now, Iray 1.36, before 1.14.

  Octane's metal is brighter overall (0.111 against 0.081), which is a
  separate matter.

The eight Calib-Skin figures (front light), new against old export,
changed by ≤ 2 % on the face and body regions and by ≤ 5 % on the ears.
That is little because their roughness maps sit mostly near 0 or 1.

Some skins sat far off Iray in that run: Nishi at 2.5–3.1 × and Zurie
at 2.0–2.4 ×. That was not the export; the c717cd8 export rendered the same.
Their products (Angel HD, Nishi, FWSA Zurie HD) had been uninstalled since
the references were rendered: 19–24 missing textures each, and the figures
came out bare white. Reinstall them before the next skin round.

## Afternoon: the figure check

### Content back

The Dazzle MCP installs DAZ store products now. It put back what intake
lacked (66 IM products and 2 Renderotica ones) and the Calib-Skin skins
(Angel HD with AugrDust Bodyart, Nishi and FWSA Zurie HD). For teens it
added Comfy Shorts Set and Underbelly Outfit. Exports with the cached dumps
report 0 missing textures for `Calib-Skin-{Angel,Nishi,Zurie}-front`
(37, 37 and 31 textures), intake (936) and teens (180).
The headless dump of intake, which failed before for want of content,
now runs in 6.3 min. Its export also reports 0 missing textures
(12.5 M faces, 320 meshes).

### Scene

intake is 12.5 M faces, too slow for a CPU Iray reference. teens is
1.2 M faces. Its girl on the right wears Comfy Shorts on a Genesis 8
Female, and their sweatshirt, tank top, shorts, belt, buckle and slip-ons
all carry a displacement map (8-bit RGB TIFs, mostly 4096²). As exported:

- `amount` is 3 mm on most surfaces and 5 mm on the sweatshirt, tank top
  and one slip-on surface. The product presets set the range, and the
  scene keeps it;
- `subdLevel` is 2 on the clothes and the dolly (a figure-class prop), and
  0 on the buckle (a plain node).

None of the user's 80 scenes with a displacement map sets Minimum or
Maximum in the scene itself; the ranges come from the products.

A far camera puts the clothes at about 3.5 mm per pixel, which makes the
relief sub-pixel. On that view, teens with displacement and teens with
`amount` 0 differ only by sample noise (region means ±0.1 of 255).

So `Calib-Disp-Teens` is a copy of teens in My Library only, with the
camera 140 cm from her waist (about 1 mm per pixel) and nothing else
changed. The Iray reference took 400 samples and 8.5 min.

### Octane

The full-resolution close-up mostly failed in Octane X with "render
engine failure", with displacement and without it. Its first try without
displacement rendered, and once 100 samples at parallel samples 8 did, but
the same file failed on the next run. Half resolution (640 × 360) always
rendered, and the far view of the same export did too. After a failure the
next big scenes kept failing until the Octane cache
(`intersection_functions.lib`) was moved away. Parallel samples 4 did not
help either. This is not displacement: it is its own issue (#50).

The comparison therefore runs at 640 × 360. That gives Octane with
displacement at half resolution, and Octane without it and Iray at full
resolution scaled down, all at 400 samples.

- **No cracks.** No gaps opened at the UV seams, at the hems or trims,
  or between the belt and the shorts. The silhouettes are the same as
  without displacement.
- **Relief.** At about 2 mm per pixel, 3 to 5 mm reads mainly as shading
  grain. Detail (std of a 3 px high-pass) on the sweatshirt is 26.2 for
  Iray, 23.2 for Octane and 20.9 for Octane without displacement. On the
  shorts it is 16.7, 14.1 and 13.9. The half-resolution render is noisier,
  so this only says the right direction; the amount itself rests on the
  Calib-Disp spheres.
- **Brightness** with displacement against without it: +1.5 (sweatshirt),
  +1.1 (shorts), +3.6 (belt) of 255. Both Octane renders sit 4 to 6 %
  above Iray on the clothes, and about 17 % above it on the wall. That is
  the scene's light, not the displacement.
- **Camera.** The export's camera (DAZ `XYZ` = Rz·Ry·Rx) frames the close-up
  as Iray does. Edge correlation with Iray is 0.71, against 0.31 for the
  other order (Rx·Ry); this camera has a yaw of −10° and a pitch of −30°.

Other differences in teens are not displacement. They are the same with
`amount` 0 (#51):

- her feet come through the Slip Ons;
- the man has one shoe;
- the blonde's stockings are missing and her hair is white.

So the figure agrees with the spec: vertex displacement at subdLevel 2
leaves the seams of the clothes closed. Skin displacement (HD figures'
detail maps) is still unchecked, and teens has none on its skin.
