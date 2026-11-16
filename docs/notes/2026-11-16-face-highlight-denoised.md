# 2026-11-16 The skin faces' highlight p99.5 on the denoised pass (#151)

Goal: the spec's skin highlight numbers (dual lobe / Top Coat section) were
pixel p99.5 over the face box of raw Octane renders at 400 samples
(2026-11-07 skin hue, 2026-11-09 bump as normal map), against Iray
references rendered with the Post Denoiser on. #139 could not recheck
them (no `denoised/` twin). Re-render, re-measure, correct the spec if
they move. Measurement only: no rule changed (#19 / #57 hold).

Scenes: the eight private Calib-Skin-<figure>-front scenes (G8 Basic
Female in a store skin: renders local only) and the dlflat / gloss0 pairs
of Victoria, Gabriela, Nishi, Kayo. enhanced-final, 960 x 720, 400 and
2000 samples, current master (b30fe8a). Octane ~40 s - 7 min a scene,
16 scenes at 400 ~13 min, at 2000 ~2 h.
Renders: `~/Library/Caches/orbis/calib/octane/f151-400`, `f151-2000`.

Tool: `scripts/dev/face-highlight.py` (new): face box x 380 - 580, y 90 -
340, linear luminance (gamma 2.2), p99.5, Octane / Iray, both passes of
one render side by side; `--sub` for dlflat - gloss0, `--blur` for the
p99.5 after a 2-pass 5-tap binomial blur. It reproduces the spec's
2026-11-09 numbers from the old renders exactly (Angel 1.112, Kayo 1.006,
Gabriela 0.999 ...; dual lobe Victoria 0.830, Kayo 0.969).

## 1. Iray's side first (#171)

#171 asks whether the Iray Server applies the Post Denoiser at all. For
this metric it does not matter: two independent Iray renders of the same
front scenes (`iray-400`, 2026-10-28, and `iray-full`, 2026-10-30) give
the face's p99.5 to 0.1 % (Victoria 0.2604 / 0.2604, Kayo 0.1468 /
0.1468, Monique 0.0696 / 0.0695, Twosret 0.0949 / 0.0950); their
difference's high-pass is 1.9 % of the face's mean, i.e. ~1.3 % noise per
render, against Octane raw 400's 10 - 19 % grain. In the dlflat - gloss0
differences Iray's high-pass std is 0.0017 - 0.0044, Octane 2000
denoised's 0.0014 - 0.0030. Iray's number is a fixed target.

## 2. What the denoiser moves (same render, raw / denoised)

First on the 2026-11-12 renders of #47 (`octane/t47`): denoised / raw
0.93 - 0.96 at 400 samples, 0.99 at 2000. Then fresh:

| face p99.5, Octane / Iray | raw 400 | denoised 400 | raw 2000 | denoised 2000 | 2026-11-09 (raw 400, spec) |
|---|---|---|---|---|---|
| Angel | 1.118 | 1.019 | 1.042 | 1.020 | 1.11 |
| Victoria | 0.940 | 0.881 | 0.893 | 0.878 | 0.96 |
| Kayo | 1.002 | 0.922 | 0.954 | 0.928 | 1.01 |
| Gabriela | 0.992 | 0.941 | 0.949 | 0.936 | 1.00 |
| Nishi | 0.945 | 0.921 | 0.925 | 0.921 | 0.95 |
| Twosret | 0.904 | 0.850 | 0.866 | 0.851 | 0.91 |
| Monique | 0.931 | 0.828 | 0.860 | 0.829 | 0.95 |
| Zurie | 0.984 | 0.943 | 0.954 | 0.946 | 1.01 |

- Raw 400 sits 3 - 11 % over the denoised pass (Monique, the darkest
  skin, most), raw 2000 still 1 - 4 %: the random walk's grain lifts the
  top half-percent of pixels.
- **Denoised 400 = denoised 2000 within 1 %** on every face: the denoiser
  is not clipping the highlight, it is converged. The denoised pass at
  the export's samples is the number to quote.
- A 1 / sqrt(N) extrapolation of the raw pair (bias(400) = (raw400 -
  raw2000) / (1 - 1 / sqrt 5)) lands 2 - 4 % under the denoised pass
  (Angel 0.98, Victoria 0.86, Monique 0.80): the grain's tail does not
  scale that simply, so it is not used.

Grain-robust check: the same p99.5 after a 2-pass binomial blur (sigma
~1.4 px) reads 1.02 / 0.89 / 0.93 / 0.94 / 0.92 / 0.86 / 0.86 / 0.95
on raw 2000, denoised 400 and denoised 2000 alike (within 0.6 %).

## 3. What the export moved since 2026-11-09

Blurred (grain-free) face p99.5, old raw 400 (bump85 `full-new`) -> now:
Angel 1.037 -> 1.020, Victoria 0.929 -> 0.886, Kayo 0.943 -> 0.936,
Gabriela 0.948 -> 0.937, Nishi 0.930 -> 0.923, Twosret 0.862 -> 0.858,
Monique 0.913 -> 0.857, Zurie 0.981 -> 0.947: 0 - 6 % down. So the
spec's 1.11 -> 1.02 (Angel) is ~7 % grain and ~2 % export; Monique's 0.95
-> 0.83 ~5 % / ~6 %.

Split by the decomposition scenes (blurred p99.5, Octane / Iray):

| | gloss0 old | gloss0 now | dual lobe (dlflat - gloss0) old | now: raw 400 | denoised 400 | denoised 2000 |
|---|---|---|---|---|---|---|
| Victoria | 1.13 | 1.01 | 0.751 | 0.527 | 0.524 | 0.532 |
| Gabriela | 1.12 | 1.00 | 0.858 | 0.802 | 0.799 | 0.788 |
| Nishi | 1.13 | 1.02 | 0.756 | 0.711 | 0.701 | 0.706 |
| Kayo | 1.13 | 1.01 | 0.825 | 0.558 | 0.534 | 0.524 |

(gloss0 columns unblurred pixel p99.5 of the gloss0 face, old raw /
now denoised 2000.)

- The gloss0 faces (dual lobe on, glossy layer off) went from 11 - 13 %
  over Iray to 0 - 2 %: something since fixed the base / gloss part.
- The dual lobe's own share fell, most on Victoria and Kayo (0.75 /
  0.83 -> 0.53 / 0.52). Not grain (the blurred measure is the same on
  every pass) and not the reference: the export. Opened as #174 with
  the candidate commits (Top Coat layering #138, dual lobe weight /
  reflectivity on Universal, coating lobe on the base normal #140, the
  Phong exponent #162); bisecting binaries on the four pairs is the
  next step there.
- The unblurred dlflat - gloss0 p99.5 is grain-dominated at 400 raw
  (Kayo 0.85 raw 400, 0.64 raw 2000, 0.52 denoised 2000): a difference
  of two grainy images doubles the noise on a small signal (Kayo's Iray
  lobe peak 0.054). The spec's 2026-11-09 dual lobe levels (0.83 - 0.97)
  were mostly that; its with / without bump comparison is a ratio of
  equally grainy renders and stands as a relative statement.

## Spec

The dual lobe / Top Coat paragraph now quotes the denoised full faces
(1.02, 0.88, 0.92, 0.94, 0.92, 0.85, 0.83, 0.94), the blurred dual lobe
(0.53, 0.79, 0.71, 0.52), the measurement rule (denoised pass; a pixel
p99.5 on raw 400 reads grain) and #174. The 2026-11-07 derived-roughness
list is marked as raw, pre-#85 history. No rule changed.

## Process

- Two independent references of the same scene answer "is the reference
  noisy?" without knowing what the renderer did (#171).
- For a percentile over grainy skin, render the same scene at a second
  sample count: if the denoised pass doesn't move, it is converged; a
  blurred percentile gives the same answer from a single render.
- Never subtract two raw renders for a peak metric; blur first.
- zsh does not split an unquoted `$S` into words: the first batch went to
  `calib-variants.sh` as one scene name (export failed at once). Use an
  array.

Open: #174 (dual lobe drift), #171 (server denoiser; not needed here).
