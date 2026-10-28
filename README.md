# ORBIS

**O**ctane **R**ender **B**ridge for **I**ray **S**cenes — exports DAZ Studio
scenes to Octane X.

The name nods to *Orbis sensualium pictus* (1658) by John Amos Comenius, the
first picture book for teaching children: the world in pictures. Here: a DAZ
scene, painted by Octane.

Octane X (OctaneRender for macOS) has no DAZ Studio plugin. ORBIS converts a
saved DAZ Studio scene into an Octane package (`.orbx`) — geometry, Iray
materials translated to Octane's, lights, the environment, every DAZ camera
as its own render target and DAZ's tone mapping — ready to open in
Octane X.

> **Status: experimental.** Not yet verified end to end in DAZ Studio
> ([#1](https://github.com/vysokyj/orbis/issues/1)).

## Install

1. Download `orbis-<version>-macos.zip` from the
   [releases](https://github.com/vysokyj/orbis/releases) and unzip it.
2. Until the binary is notarized ([#3](https://github.com/vysokyj/orbis/issues/3)),
   macOS blocks a downloaded one; clear the quarantine flag and install:

   ```sh
   cd orbis-<version>-macos
   xattr -d com.apple.quarantine orbis
   ./orbis install
   ```

   This puts the binary into `~/Library/Application Support/ORBIS/bin/` and
   the script into your DAZ library ("My Library" from DAZ Studio's content
   directories, or `--library DIR`) as `Scripts/ORBIS/Export to Octane`.

From a checkout: `scripts/install.sh` builds and installs.

## Use

1. In DAZ Studio, run *Content Library → Scripts → ORBIS → Export to Octane*
   (after the first run also *File → Export to Octane...*). The script saves
   the scene first (asking for a file if it has none) — the export reads the
   saved `.duf` — then asks for the options, remembered for next time:
   - **Save the .orbx** beside the scene or in a folder
     (`~/Documents/ORBIS` by default);
   - **Fast preview** — Octane's direct-lighting kernel instead of the path
     tracer; **Shadow-catching ground** when Iray draws one; **Denoise** —
     Octane's AI denoiser on the final render;
   - **Look** — *Enhanced* (default) keeps Octane's take where it is more
     natural: OpenPBR skin, a warm low sun, glass reflections; *Faithful*
     stays as close to Iray as it can
     ([details](docs/spec/octane-export.md#look)).
2. Open the `.orbx` in Octane X with *File → Open* (Octane X only opens
   files from its own panel) and select the render target named after the
   scene.

## What it translates

- **Geometry** — figures as posed and morphed, subdivided like DAZ renders
  them, geografts and shells, strand hair (dForce), instances (UltraScatter
  and the like).
- **Materials** — Iray Uber (diffuse, gloss, metal, top coat, translucency,
  refraction, absorption, emission, maps and tiling), PBRSkin, dForce hair,
  skin with subsurface scattering, layered images, the NVIDIA MDL example shaders (metals, concrete, porcelain, granite, resin, …).
- **Lights** — point, spot, distant, area (rect, disc, sphere), emissive
  surfaces; photometric and legacy units.
- **Environment** — HDRI dome (rotation, intensity, hidden dome), Iray
  Sun-Sky as Octane daylight, the shadow-catching ground.
- **Camera and image** — every DAZ camera as a render target, depth of
  field, Iray's exposure and tone curve.

Each rule is calibrated against Iray — see the
[spec](docs/spec/octane-export.md). What's still missing is tracked in the
[issues](https://github.com/vysokyj/orbis/issues)
([overview](docs/spec/octane-export.md#open-work)).

## Command line

```sh
orbis export <scene.duf> [--dump FILE] [-o out.orbx|out.usdz | --out-dir DIR]
             [--preview] [--no-ground] [--look enhanced|faithful] [--no-denoise]
             [--content-dir DIR]… [--progress]
orbis install [--library DIR]
```

`export` needs the scene dump the DAZ Studio script writes; without `--dump`
it uses the last one it kept for the scene, so a scene exported once from
DAZ Studio re-exports from the command line.

## Build

```sh
cargo test
cargo build --release          # target/release/orbis
scripts/package.sh             # dist/orbis-<version>-macos.zip
scripts/install.sh             # build + install into your DAZ library
```

## Documentation

- [docs/](docs/README.md) — map of all documents
- [Export spec](docs/spec/octane-export.md) — what the export does with each
  part of a scene, and why
- [Calibration](docs/calibration.md) — how it is checked against Iray: test
  scenes, references, scripts
- Background: [Iray in DAZ Studio](docs/daz/iray.md),
  [DAZ files](docs/daz/files.md), [Octane X](docs/octane/octane-x.md)

## License

GPL-3.0-or-later. OctaneRender and Octane X are trademarks of OTOY Inc.; DAZ
Studio and Iray are trademarks of their owners. ORBIS is not affiliated with
them.
