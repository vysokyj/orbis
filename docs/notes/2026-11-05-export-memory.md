# 2026-11-05 — export memory: a binary USD layer (#16)

Goal: cut `orbis export`'s peak memory and output size on big scenes with
strand hair without changing the render (#16, its last open part; width
and colour were settled in 6b5dbc9). Scenes: intake (private; 296 nodes,
60 M mesh points after refinement, 4.9 M strand points, 754 textures),
teens (private), Calib-Hair for the render check. Numbers only; nothing of
the private scenes is in git.

## Where the peak came from

`/usr/bin/time -l` plus a temporary counting allocator printing the live
heap at each progress step (removed again), intake, before:

| step | live heap | peak so far |
|---|---|---|
| `.duf` parsed (`serde_json::Value`) | 0.95 GB | 1.24 GB |
| dump loaded (typed, already not a `Value`) | 1.14 GB | 1.52 GB |
| meshes built | 14.1 GB | 15.7 GB |
| … of which library `.dsf` files as JSON values | 6.4 GB | |
| … of which the meshes (points 0.74, faces 1.83, UVs 1.96, normals 1.33, subsets 0.32) | 6.2 GB | |
| USDA written (9.6 GB of text, 14.8 GB capacity) | 27.7 GB | 35.1 GB |

- The dump is not the problem: 348 MB of JSON, typed, 0.2 GB in memory;
  strand hair is 4.9 M points of 65 M. Its format stays as it is.
- The library cache kept every `.dsf` as a `Value`: 300 MB of JSON → 6.4 GB
  (number arrays cost ~20× as values). Half of the biggest (a 94 MB hair
  file) is `modifier_library`, which nothing reads.
- The text layer had outgrown the zip: 9.6 GB > 4 GiB, so **intake no
  longer exported at all** ("scene.usda: over 4 GiB") after 280 s at 19.7 –
  22.7 GB footprint on a 24 GB Mac, swapping (the earlier 47 s / 16 GB in
  the issue was before normals and more figures were added).
- Translating the materials took 180 s of it with or without memory
  pressure: per-texel skin baking (`derived_skin`, `powf`), not memory —
  #78.

## What was tried

1. **Octane and usdc, probed first.** Calib-Hair's `scene.usda` converted
   with macOS's own `/usr/bin/usdcat -o scene.usdc` (Pixar's writer): 63 →
   19.5 MB. Packed (a) as `scene.usdc` inside the `.usdz`, (b) bare as
   `assets/scene.usdc` with `project.ocs` pointing at it. Octane X 128 spp,
   1920 × 1080, mean absolute difference to the text layer's render: (a)
   0.55, (b) 0.55 of 255 — sampling noise. Both import; (b) has no 4 GiB
   zip limit and needs no CRC before the data, so the package holds the
   bare layer.
2. **Own crate writer** (`src/usd/usdc.rs`), version 0.8.0, from OpenUSD's
   `crateFile.cpp`. Ground truth for which fields Sdf writes: a small usda
   converted with `usdcat` and decoded with a throwaway Python reader (no
   target/connection specs, `variability` on relationships, `custom` on
   every attribute). LZ4 without a match finder: blocks of literals only
   are valid LZ4. Mine was 23.5 MB on Calib-Hair against Pixar's 19.5 (its
   LZ4 matches); 20.4 MB after step 5 — a real LZ4 is not worth a
   dependency.
3. **Streaming.** A `LayerSink` trait with two writers — text (tests,
   `-o x.usda`) and binary. Arrays go out the moment a prim hands them
   over; the export passes the meshes by value and each is dropped once
   written. The crate's header (offset of its table of contents) is
   patched at the end into the package's first chunk
   (`OrbxWriter::patch_current`). What still needs the meshes (UV scales,
   ground height, shadowless prims, framing camera) is computed first.
4. **Library files**: read without `modifier_library` (a custom
   `Deserialize` skipping it), and a geometry or UV-set entry is taken out
   of its file's JSON once parsed (cache keyed by decoded `file#id`).
   Clearing the whole file cache after the meshes was a dead end: lights,
   cameras and the environment look up every node's `.dsf` again.
5. **Output size**: the first intake layer was 3.44 GB; per attribute,
   `faceVertexIndices` stayed at 4.03 bytes per index — a refined mesh
   lists face, edge and corner points in three runs, so a quad's corners
   jump between them and every delta needs 32 bits. Numbering the points in
   the order the faces first use them (at write time, the meshes keep
   their order for everything else): 1.19 bytes, the layer 2.84 GB.
   Floats (points 0.72 GB, normals 0.67, UVs 0.47) are the rest and stay
   whole.

## Result

| scene | | time | peak footprint | max RSS | `.orbx` | scene layer |
|---|---|---|---|---|---|---|
| intake | before | 281 s, **failed** | 19.7 – 22.7 GB | 7.2 – 15.3 GB | — | 9.6 GB usda (> 4 GiB) |
| | after | 299 s | **8.8 GB** | 9.8 GB | 3.20 GB | 2.84 GB usdc |
| teens | before | 60.4 s | 3.66 GB | 3.98 GB | 1.13 GB | 686 MB usda |
| | after | 56.7 s | **1.89 GB** | 2.28 GB | 666 MB | 222 MB usdc |
| Calib-Hair | before | 1.4 s | | | 84.2 MB | 63.2 MB usda |
| | after | 1.2 s | | | 41.4 MB | 20.4 MB usdc |

- Intake's peak is now the meshes while they are built (live heap 7.3 GB
  at "writing the USD scene"); writing the layer took 4 s and the heap
  fell to 1.2 GB after it. Its time is the skin baking (#78).
- **Render**: Calib-Hair exported before and after, Octane X 128 spp: mean
  absolute difference 0.61 of 255 over the frame, 3.07 on the hair (noise
  between two renders of the same scene: 0.55 and 3.06).
- `usdcat` reads intake's 2.84 GB layer whole (10.7 GB of text back).
  A test compares `usdcat` of the text and the binary layer of a sample
  scene (mesh, subsets, instanced curves, light, materials): identical.

## Process lessons

- Measure the live heap per phase before guessing: the suspected parts
  (the dump's JSON, the strands) were small; the library cache and the
  text layer were the peak.
- macOS ships Pixar's `usdcat`: a free reference reader and writer to
  check a binary writer against, and to probe Octane with before writing
  any code.

Left: the meshes themselves (6.2 GB on intake) could be built and written
one at a time; that needs the lift onto refined figures, the ground height
and the instancers in a pass of their own. Not needed for now.
