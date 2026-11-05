# 2026-11-05 — texture baking speed (#78)

Goal: cut the time `orbis export` spends baking textures (skin colour and
detail mask, roughness, blends) with the same output bytes (#78). Scenes:
intake (private; 320 meshes, 956 textures, several Genesis figures),
teens (private), Calib-Skin-Victoria-front (one figure, textures not
scaled down). Numbers only; nothing of the private scenes is in git.
Release build, `/usr/bin/time -l`, phase times from `--progress` lines.

## Where the time went

intake, before (311.6 s in all):

| phase | time |
|---|---|
| reading + building the meshes | 46 s |
| writing the USD layer | 5 s |
| translating the materials | **185 s** |
| writing the `.orbx` (927 of 956 textures scaled to 1024 px) | **74 s** |

`sample` over 30 s of the materials phase: `powf` 60 % of the samples
(`derived_skin`, and `derived_mono` from the roughness pins), resizing
6 %, JPEG decoding 4 %, PNG encoding 3 %. Per skin texel the bake ran ~33
`powf`: 18 decodes (diffuse, translucency and their low-passed texels), 3
hue, 3 dark gain, 3 encodes, plus 6 decodes and 3 encodes for the detail
mask. All on one thread.

## What was done

1. **Exact tables, no approximations.** The decode `colour_to_linear(c /
   255)` takes 256 values: a table. The encode `(v.clamp(0, 1)^(1/γ) · 255
   + 0.5) as u8` is monotonic in `v`, so it is the number of thresholds
   ≤ `v`, each threshold the smallest `f32` that encodes to that byte —
   found by bisecting the bit patterns with the original expression
   (`src/bake.rs`). Checked on every `f32` in [0, 1] at γ 2.2 and 1 (an
   ignored test, 15 s in release) and around every threshold in the
   normal suite. NaN and negatives give 0 as the cast did.
2. **Skin bake.** The hue (`SKIN_HUE_OPENPBR^e`) and dark gain depend only
   on the low-passed texels, which come in long equal runs (a 32 px image
   upscaled): worked out once per run. The detail mask depends on two
   bytes per channel: a 65 536-entry table.
3. **Luminance / channel bakes** (`derived_mono`): a channel has 256
   values, so `f` (roughness curve, offsets) runs once per byte; under
   Average a grey texel (r = g = b, nearly every roughness map) takes the
   table, a coloured one runs `f` itself.
4. **Threads**: `std::thread::scope` over blocks of 16 rows, taken as
   threads free up (the efficiency cores are slower). The two maps of a
   skin decode and low-pass side by side. No new dependency (no rayon).
5. **Writing the `.orbx`**: textures are read and scaled a batch (2 ×
   cores) at a time in parallel, written in order — this was the second
   largest phase and the same bytes either way.

Not done: baking at the texture-budget size. The budget is chosen from
all textures, generated ones included, after the materials; and a
non-linear bake of a downscaled map is not the downscale of the bake, so
the output would change. Not needed at these times.

## Result

| scene | before | after | output |
|---|---|---|---|
| intake, materials phase | 185 s | **30 s** | |
| intake, writing the `.orbx` | 74 s | **15 s** | |
| intake, whole export | 311.6 s | **96.2 s** | `.orbx` (3.2 GB) byte-identical (`cmp`) |
| teens | 58.4 s | 28.4 s | byte-identical |
| Calib-Skin-Victoria-front (full-size textures) | 15.6 s | 1.9 s | byte-identical |

Peak footprint on intake 7.5 → 8.3 GB (the parallel texture batch); max
RSS unchanged (5.7 GB). The output did not change, so no render check.

A test (`bakes_match_the_per_texel_formulas`) keeps the old per-texel
formulas and compares the skin colour, detail mask, luminance / channel
bakes and blend byte for byte on noise maps of odd sizes.

## Process lessons

- `sample` on the running export names the hot function in 30 s; it
  pointed at `powf`, not at the image decoding that was suspected.
- Making a lookup exact (thresholds from the original expression) is no
  harder than an approximation and needs no render check: `cmp` of the
  whole package is the proof.

What is left in the materials phase (30 s) is spread out: the serial
low-pass resizes, JPEG decoding, PNG encoding.
