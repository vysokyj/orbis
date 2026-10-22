# ORBIS

**O**ctane **R**ender **B**ridge for **I**ray **S**cenes — exports DAZ Studio
scenes to Octane X.

The name nods to *Orbis sensualium pictus* (1658) by John Amos Comenius, the
first picture book for teaching children: the world in pictures. Here: a DAZ
scene, painted by Octane.

Octane X (OctaneRender for macOS) has no DAZ Studio plugin. ORBIS converts a
saved DAZ Studio scene into an Octane package (`.orbx`) — geometry, Iray
materials translated to Octane's, lights, the HDRI environment, every DAZ
camera as its own render target and DAZ's tone mapping — and opens Octane X
with it.

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

1. In DAZ Studio, save the scene.
2. Run *Content Library → Scripts → ORBIS → Export to Octane* (after the
   first run also *File → Export to Octane...*). Pick where the `.orbx` goes
   (beside the scene or a folder, `~/Documents/ORBIS` by default) and the
   options (final path tracing or fast preview, shadow-catching ground);
   they are remembered.
3. Octane X starts with the `.orbx` revealed in Finder and its path on the
   clipboard. Octane X only opens files from its own panel: *File → Open*,
   ⇧⌘G, paste, then select the render target named after the scene.

## Command line

```sh
orbis export <scene.duf> [--dump FILE] [-o out.orbx|out.usdz | --out-dir DIR] [--preview] [--no-ground] [--content-dir DIR]… [--open]
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

- [docs/spec/octane-export.md](docs/spec/octane-export.md) — what the export does and why
- [docs/octane/](docs/octane/) — Octane X, the ORBX container, USD
- [docs/daz/](docs/daz/) — DAZ files, Iray, content layout

## License

GPL-3.0-or-later. OctaneRender and Octane X are trademarks of OTOY Inc.; DAZ
Studio and Iray are trademarks of their owners. ORBIS is not affiliated with
them.
