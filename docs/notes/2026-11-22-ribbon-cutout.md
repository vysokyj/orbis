# 2026-11-22 — The cutout map on ribbon hair (#161)

Goal: Iray's own cutout share on ribbon hair, against Octane's under
#202's interim rule (a ribbon / prism strand writes the cutout map's mean
× the value as a constant). The decided Look split (docs/tuning.md) and
decision (a) (no pile factor) stand.

Scenes (calib/scenes-store, store hair, numbers only): Calib-Hair-Kevin,
-Hudson and their -op1 twins (`scene-channels.py`: Cutout Opacity 1, no
map, on the hair materials). Iray: base / -nogloss 2000 samples, Iray
Server CPU, denoiser off (renders.tsv); -op1 2000 samples, GPU, denoised
(948258f, 9b81af2). Octane: faithful-final and enhanced-final, 256
samples, re-exported after #202 / #216 / #220 / #222.

## Iray's backgrounds and the noise

- Background corners op1 / base: 0.995 - 1.000; pixels that differ by
  more than 0.02 linear lie inside the hair only.
- Pixel noise of the undenoised base: the decode bias (mean of decoded
  pixels over decoded 2 x 2 means) 1.001 - 1.006, the denoised op1 1.002
  - 1.010 - the base's noise moves no patch by 1 %.
- Octane raw pass against the denoised one: the same table to 0.01. So
  base (no denoiser) and op1 (denoised) compare directly.

## The shares (luminance, hair-set-compare.py's boxes)

| patch | Iray op1 / base | Octane F op1 / base | Octane E op1 / base | F base / Iray | F op1 / Iray op1 | E op1 / Iray op1 |
|---|---|---|---|---|---|---|
| Kevin crown | 0.91 | 1.23 | 1.09 | 0.73 | 0.99 | 0.89 |
| Kevin front | 0.92 | 1.25 | 1.13 | 0.82 | 1.12 | 1.00 |
| Kevin curls | 0.96 | 1.21 | 1.11 | 0.71 | 0.90 | 0.82 |
| Kevin shaved | 1.03 | 1.28 | 1.15 | 1.03 | 1.29 | 0.94 |
| Hudson crown | 0.96 | 1.18 | 1.03 | 0.78 | 0.95 | 0.73 |
| Hudson front | 0.94 | 1.14 | 0.93 | 0.79 | 0.96 | 0.73 |
| Hudson side | 0.97 | 1.18 | 1.03 | 0.92 | 1.12 | 0.82 |

Over the whole hair box (background included) Iray op1 / base is 0.981
(Kevin) / 0.986 (Hudson); in the 2 % of pixels where Iray's base and op1
differ most (hairline wisps, strand ends over the background) 0.95 /
0.79 - the map's only visible work in Iray.

The constants the mean rule wrote: Kevin's four hair materials 0.54 /
0.28 / 0.43 / 0.54, Hudson's 0.52 / 0.53.

Reading: in dense hair Iray's cutout costs nothing (a faded ribbon part
shows the hair behind it, of the same colour; with the map off Iray is
even 4 - 9 % darker), Octane's constant semi-opacity on every strand
costs 14 - 28 % in Faithful, up to 15 % in Enhanced (the transmission
Enhanced keeps hides part of it). The resume's second outcome: Iray's
share smaller, Octane's cutout loses light. Octane / Iray with the map
off, per variant (F op1 / Iray op1: 0.90 - 1.12, Kevin shaved 1.29),
is the rest of the export without the cutout - most of the "dense
ribbon hair ~0.8 x" of 2026-11-18 was this cutout, not the ribbons'
shape alone.

## Options weighed

- Keep the mean (#202): 14 - 28 % short in every dense patch.
- A gradient along the strand (W) from the map's profile: the map's v
  runs along Kevin's guide lines (2026-11-18), but its render hairs are
  grown from the guides and their v is not in the dump; Hudson's map
  varies across u (one opacity per strand), which no profile along the
  strand gives. And Iray's share says the map is worth ~1 in dense hair
  whatever its layout. Not built.
- **Drop the map on ribbons / prisms (the value alone)** - chosen.
  Opacity 1 is the constant nearest Iray's share in every patch. The
  cost: wisps and strand ends Iray fades come out whole. A plain value
  without a map stays (a uniform cutout in Iray too; not measured).

## With the rule (Octane = the -op1 renders: the new export of the base scenes writes opacity 1 where the -op1 export omitted the pin; otherwise identical project.ocs)

| | Faithful / Iray (R / G / B, lum) | Enhanced / Iray (lum) |
|---|---|---|
| Kevin crown | 0.83 / 0.93 / 0.97 (0.90) | 0.81 |
| Kevin front | 0.93 / 1.06 / 1.13 (1.02) | 0.92 |
| Kevin curls | 0.81 / 0.88 / 0.94 (0.86) | 0.78 |
| Kevin shaved | 1.30 / 1.32 / 1.40 (1.32) | 0.96 |
| Hudson crown | 0.91 / 0.91 / 1.04 (0.92) | 0.70 |
| Hudson front | 0.84 / 0.91 / 1.01 (0.90) | 0.68 |
| Hudson side | 1.04 / 1.10 / 1.27 (1.09) | 0.80 |

Before (mean rule, Faithful): Kevin 0.71 - 0.82 (shaved 1.03), Hudson
0.78 - 0.92. Kevin's shaved side overshoots now: Iray's cutout share
there is 1.03, so the mean (0.28) was hiding a Faithful-only excess on
short strands over the scalp - filed as #223. Free content is
unaffected: Pixie Hair's strands carry no cutout map.

## Process

- The -op1 dumps are keyed by their calib/scenes-store paths (the -op1
  dump was derived from the base dump there); `calib-variants.sh` with
  the My Library name failed "no cached dump". Pass the git copy's path.
- `calib-variants.sh` re-renders every `.orbx` in the variant's folder,
  not only the scenes named; clean the folder (or the cache) first to
  keep a batch short.
