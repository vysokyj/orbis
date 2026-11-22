# 2026-11-22 — Faithful's ribbon fold on short strands (#223)

Goal: why Calib-Hair-Kevin's shaved side (short ribbon strands over the
scalp) came out 1.29 - 1.32 x Iray in Faithful (Enhanced 0.94) once
#161 dropped the strand cutout map. Scenes (calib/scenes-store, store
hair, numbers only): Calib-Hair-Kevin, -Hudson, their -nogloss and -op1
twins; existing Iray references only (base / -nogloss CPU undenoised,
-op1 GPU denoised; both compare directly, 2026-11-22 ribbon-cutout
note). Octane: Final, 256 samples, denoised pass;
`hair-set-compare.py`'s boxes.

## Decomposition (Kevin-op1, Octane only, project.ocs swapped between the Looks)

The two Looks' project.ocs differ only in materials. Variants of the
Faithful export with one part taken from Enhanced (whole material nodes
moved by name, ids renumbered):

| variant / Octane F | crown | front | curls | shaved |
|---|---|---|---|---|
| Enhanced skin + figure, Faithful hair | 1.00 | 1.00 | 1.00 | 1.00 |
| hair BRDF 7 (STD) -> 6 (GGX e.p.) | 1.00 | 1.00 | 1.00 | 1.00 |
| hair as Enhanced (fold off) | 0.88 | 0.88 | 0.88 | 0.73 |
| fold off, Faithful's BRDF kept | 0.88 | 0.88 | 0.88 | 0.73 |

- The skin shaders (IrayAlbedo vs OpenPBR on the scalp under the
  strands) and the rough-lobe split move nothing: **the whole Faithful /
  Enhanced difference is `ribbon_reflects`**, the fold of the Uber
  translucency into the diffuse.
- The -nogloss twin agrees: Iray's gloss share on the shaved patch 1.06,
  Octane's 1.08 - gloss is not it.

Then the fold split into what Octane's transmission returns: a variant
with the diffuse at its 1 − w share and no transmission (N):

| patch | N (lin) | Enhanced − N | Faithful − N | Iray − N |
|---|---|---|---|---|
| crown | 0.0291 | 0.0002 | 0.0041 | 0.0045 |
| front | 0.0236 | 0.0000 | 0.0033 | 0.0005 |
| curls | 0.0200 | 0.0001 | 0.0027 | 0.0051 |
| shaved | 0.0460 | 0.0022 | 0.0202 | 0.0054 |

- Octane's curves return almost none of a diffuse transmission
  anywhere here (Enhanced − N 0 - 11 % of the fold), as #161 measured on
  synthetic strands (0.44 in a dense patch).
- On the shaved side Iray gets back about a quarter of what the fold
  adds (0.0054 of 0.0202); in the dense patches the residual (Iray − N)
  is of the fold's size or noise around the ribbons' shape factor.

Reading: the fold stands for light Iray's flat ribbon passes on from
behind - #161 measured it on free strands under a dome, where behind a
strand is the lit dome, and in a pile the lit hair behind is of the same
colour. A short strand lies on the scalp: behind it is skin, which takes
the passed light. Nothing in Iray's MDL changes; the geometry behind does.

## Which strands are short (the dumps, median length per material group)

- Kevin: Short (the shaved side, 54 696 strands) 1.56 cm, Sideburns
  1.49, Hairline 1.82, Top 7.91.
- Hudson: Hairline 0.71, NeckThin 0.96, NeckLower 1.13, Sideburns 1.38,
  Bangs2 1.73, Sides 1.77, Back 1.95; every other group 3.56 (Bangs8) to
  16.6.

No group between 2.0 and 3.5 cm; the rule's threshold 2.5 cm splits them.

## Rule and result

`RIBBON_FOLD_MIN_LENGTH` (`src/material.rs`): a ribbon material group
whose median strand (`Material::strand_length`, from the dump's posed
strands in `hair_curves`) is under 2.5 cm keeps the transmission in
Faithful too. Enhanced unchanged.

Faithful / Iray (luminance; R / G / B in hair-set-compare.py):

| | before (-op1) | with the rule (-op1) | with the rule (base) |
|---|---|---|---|
| Kevin crown | 0.99 | 0.99 | 0.90 |
| Kevin front | 1.12 | 1.12 | 1.02 |
| Kevin curls | 0.91 | 0.91 | 0.87 |
| Kevin shaved | 1.29 | **0.94** | **0.96** |
| Hudson crown | 0.95 | 0.95 | 0.91 |
| Hudson front | 0.96 | 0.95 | 0.88 |
| Hudson side | 1.12 | **0.98** | **0.94** |

The base column is the shipped comparison (the export writes Cutout
Opacity's value alone, so base and -op1 exports are the same scene; the
Iray base keeps its map). Hudson's side mixes short and long groups,
hence 0.98 rather than the shaved side's 0.94. Free content: Pixie Hair
is prisms, not touched.

## Dead ends and process

- A hand-made "short groups unfolded" Hudson variant read 0.93 on the
  side: the Enhanced materials moved into the Faithful `.orbx` named a
  transmission map (Hudson_Hair_Tex02) that the Faithful export doesn't
  pack, so Octane rendered without it. The code's export (0.98) is the
  number. Moving material nodes between exports: check the textures they
  name are in the target `.orbx`.
- Renders: 26 - 78 s each at 256 samples (Kevin, Hudson), 13 Octane
  renders in all.
- Not tested: a short ribbon hair standing off the skin (buzz-cut tips, a
  beard), where the fold might still hold; the threshold is a judgment
  call (docs/tuning.md).
