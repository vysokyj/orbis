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

> **Status: experimental.** The export currently needs a `.dbz` with the posed
> geometry beside the `.duf`; a direct dump from DAZ Studio replaces it
> ([#2](https://github.com/vysokyj/orbis/issues/2)).

## Install

1. Download `orbis-<version>-macos.zip` from the
   [releases](https://github.com/vysokyj/orbis/releases) and unzip it.
2. Copy the contents of its `Content/` folder into one of your DAZ libraries
   (e.g. `~/Documents/DAZ 3D/Studio/My Library`), so that you get
   `Scripts/ORBIS/Export to Octane.dsa` and `Scripts/ORBIS/bin/orbis`.
3. Until the binary is notarized ([#3](https://github.com/vysokyj/orbis/issues/3)),
   macOS blocks a downloaded one. Clear the quarantine flag once:

   ```sh
   xattr -dr com.apple.quarantine ~/Documents/DAZ\ 3D/Studio/My\ Library/Scripts/ORBIS
   ```

## Use

1. In DAZ Studio, save the scene.
2. Run *Content Library → Scripts → ORBIS → Export to Octane*. Pick the
   options (final path tracing or fast preview, shadow-catching ground).
3. Octane X starts with the `.orbx` revealed in Finder and its path on the
   clipboard. Octane X only opens files from its own panel: *File → Open*,
   ⇧⌘G, paste, then select the render target named after the scene.

Exports go to `~/Documents/ORBIS/<scene>.orbx`.

## Command line

```sh
orbis export <scene.duf> [-o out.orbx|out.usdz] [--preview] [--no-ground] [--content-dir DIR]… [--open]
```

Without `--content-dir` the content directories are read from DAZ Studio's
preferences.

## Build

```sh
cargo test
cargo build --release          # target/release/orbis
scripts/package.sh             # dist/orbis-<version>-macos.zip
```

## Documentation

- [docs/spec/octane-export.md](docs/spec/octane-export.md) — what the export does and why
- [docs/octane/](docs/octane/) — Octane X, the ORBX container, USD
- [docs/daz/](docs/daz/) — DAZ files, Iray, content layout

## License

GPL-3.0-or-later. OctaneRender and Octane X are trademarks of OTOY Inc.; DAZ
Studio and Iray are trademarks of their owners. ORBIS is not affiliated with
them.
