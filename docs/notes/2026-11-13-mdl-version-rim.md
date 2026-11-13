# 2026-11-13 — metal_generic's rim and the MDL version: our own modules in DAZ (#112)

Question (#112, after #98 / #111): Iray darkens NVIDIA's `metal_generic`
towards the rim as about √cos θ_v under a uniform dome, the Uber's lobe
with the same `simple_glossy_bsdf` stays flat, and nothing reachable
through DAZ's channels (maps, bump, weight, a bumped layer normal, the
lobe's tint) is the cause. Left: what only a hand-written MDL module can
change. No renders this session; the batch is parked for the Iray Server.

## 1. Reading: the version line

- Every NVIDIA example module DAZ ships is `mdl 1.0;` (`metal_examples`,
  `architectural`, `resin_polyurethane_matte`, concrete, porcelain ...;
  a few 1.1, `core_definitions` 1.6). DAZ's `irayubermaterial.mdl` is
  `mdl 1.7;`. No earlier session compared the versions.
- MDL SDK `IMdl_configuration::set_simple_glossy_bsdf_legacy_enabled`:
  configures `df::simple_glossy_bsdf` **in modules of MDL version < 1.3**;
  only Iray acts on it ("no effect in the MDL SDK"). The Iray release notes
  (2020.1) say the legacy behaviour became configurable then; neither
  says what it is, nor whether DAZ enables it.
- MDL 1.3 spec, changes since 1.2: "specified that `df::simple_glossy_bsdf`
  is based on the Phong microfacet model with the v-cavities shadowing
  term" - before 1.3 the lobe's model was unspecified, so Iray kept its
  older lobe for older modules.
- So the one structural difference left between `metal_generic` and the
  Uber metal that the #111 list missed: the module version. It would also
  fit #116 - NVIDIA's architectural and resin (1.0) lose in rough
  transmission where the same df through the Uber (1.7) is `libbsdf`'s to
  1 % per ring.
- Not decidable offline: `libbsdf` (src/mdl_layer.rs, the MDL SDK) has no
  legacy path, and the Uber metal in Iray already matches it (#116,
  Calib-Furnace-Metal against the tracer). What a legacy lobe keeps is
  Iray's own; a render decides.

## 2. Our own MDL in DAZ

- NVIDIA's presets are Shader Mixer brick materials: a `DzCustomMDLBrick`
  ("MDL Callable" `nvidia/metal_examples.mdl#metal_brushed`) into a
  `DzMDLSurfaceBrick`. `make-mdl-scenes.py` writes the same two-brick
  graph, no user parameters, one material per module material, into
  `My Library/data/ORBIS/Calib/MDL Rim/MDL Rim.dsf`, and the modules
  (`calib/mdl/*.mdl`, ours, a few `df::` calls each) into
  `My Library/shaders/iray/orbis/`.
- Calib-Mi-MdlPath (.mi, headless): the module in `<library>/shaders/iray`,
  `<library>` and `<library>/data` - none found (`C120 could not find
  module '::orbis::orbis_path_a' in module path`). DAZ searches its own
  `shaders/iray` and its MDL directories (`NumMDLDirs = 0`), not the
  content directories. `DzContentMgr.addMDLDirectory(dir, false)` maps one
  for the session, but Iray takes the new paths from a posted event: only
  after a `processEvents()` did P1 resolve
  (`mdl::orbis::orbis_path_a::probe`). daz-export-mi.dsa and
  daz-render-queue.dsa now map every `<content dir>/shaders/iray` at start.
- DAZ wraps our material as it wraps NVIDIA's:
  `daz_3d::basic::pass_material(v: <our material>)`.
- Each scene uses its own modules (#94: a module imports once per DAZ
  session).

## 3. The batch (Calib-Furnace-Metal's spheres, dome, floor hidden)

| Scene | Spheres |
|---|---|
| Calib-Furnace-MdlVersion | the same material at 1.0 / 1.3 side by side: white reflect r 0.1 (M1 / M2), r 0.4 (M3 / M4), the r 0.1 lobe under `metal_generic`'s two weighted layers (M5 / M6), a closed refractor IOR 1.5 r 0.2 (M7 / M8) |
| Calib-Furnace-MdlLobe | the 1.0 reflect lobe at Calib-Furnace-Metal's eight roughnesses (F1 - F8 are the Uber's 1.7 twin) |
| Calib-Furnace-MdlTransmit | the closed refractor at r 0.05 / 0.1 / 0.2 / 0.4, 1.0 (T1 - T4) and 1.3 (T5 - T8) |

Predictions: if the version is the rim, M1 / M3 fall like Metal Brushed's
B5 / B6 (≈ 0.6 of the centre at r 0.9) and M2 / M4 equal F2 / F5; M5 = M1
(the layers add nothing); M7 below M8 as architectural below the Uber
glass. If M1 = M2, the version is ruled out and the rim stays Octane over
Iray (#111's decision) with nothing left to test.

## Process

- Grep the module headers before reading the bodies: the version line is
  part of the material.
- A DAZ script that changes paths Iray uses must let the event loop run
  before loading anything.
