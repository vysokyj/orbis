# 2026-11-12 Skin texture contrast against Iray (#47)

Goal: before any Enhanced-only "more texture" extra, find where the skin's
texture (pores, fine wrinkles) stands against Iray today and what, if
anything, loses it. Enhanced + Final against Faithful + Final, the current
export (derived skin colour of #19, mean-free-path radius, Top Coat over
the dual lobe of #138). Faithful's export is unchanged by this session.

Scenes: four Calib-Skin front scenes (Victoria, Kayo, Twosret, Monique;
private store figures: renders local only), their Iray references (400
samples, Iray's Post Denoiser on, as DAZ saved them).

## Metric

Log luminance (linear, gamma 2.2 decode), band-passed with Gaussians:
*fine* = L − G(1.2 px) (pores, skin grain at the scenes' 960 × 720 framing,
a face ~200 px wide), *mid* = G(1.2) − G(4) (fine wrinkles, pore clusters).
The standard deviation of each band in forehead, cheeks, chest and
shoulder, Octane over Iray; the median of those five (the neck region
holds the jaw's shadow edge and is left out). Log luminance makes it a
contrast, independent of the level mismatch. 1.00 = Iray's texture.

## 1. The renders compared were raw

`octane-render.lua` saved `octane.render.saveImage` - the raw beauty -
also when the export has the denoiser on; Iray's references are
denoised. At 400 samples Enhanced's random walk is grainy, and the grain
reads as texture:

| fine / mid, face + body median | Enhanced raw 400 | Enhanced raw 1600 (`--no-denoise`) | Faithful raw 400 |
|---|---|---|---|
| Victoria | 8.12 / 1.31 | 4.10 / 1.14 | 1.56 / 1.00 |
| Kayo | 5.16 / 1.14 | 2.73 / 1.03 | 1.52 / 0.98 |
| Twosret | 3.26 / 1.37 | 1.92 / 1.09 | 1.43 / 1.03 |
| Monique | 6.21 / 1.53 | 3.27 / 1.12 | 1.43 / 0.94 |

The fine band halves from 400 to 1600 samples: noise, not skin. The
round-10 renders of 2026-10-28 (the per-texel dark gain v · (v / 0.3)^0.2
the user liked) measure 3.5 – 6.3 / 1.4 – 1.9 raw, round 11 (gain from
the smooth colour) within ±5 % of them: the "deeper pores" were mostly
the walk's grain on dark skins, which a raw comparison cannot tell from
texture.

Fix (dev tooling): `octane-render.lua` also saves
`BEAUTY_DENOISER_OUTPUT` (`octane.render.saveRenderPass(id, path,
PNG_8)` - works, returns true); `octane-render.sh` puts it in
`<out>/denoised/<scene>.png`. The raw PNG stays where it was.

## 2. Denoised against Iray

| fine / mid | Enhanced denoised 400 | Enhanced denoised 2000 | Enhanced noise-free (extrapolated) | Faithful denoised 400 |
|---|---|---|---|---|
| Victoria | 0.83 / 0.87 | 0.99 / 0.92 | 0.79 / 0.89 | 1.42 / 1.00 |
| Kayo | 0.54 / 0.79 | – | 1.05 / 0.75 | 1.16 / 0.97 |
| Twosret | 0.50 / 0.85 | – | 1.11 / 0.94 | 0.85 / 0.99 |
| Monique | 0.58 / 0.90 | 0.72 / 0.89 | 1.35 / 0.89 | 0.83 / 0.92 |

Noise-free: per region from the raw 400 / 1600 pair, σ²(signal) = (4 ·
σ²(1600) − σ²(400)) / 3 (noise variance ∝ 1 / samples); medians of the
regions, so rows need not be consistent with each other.

- **Enhanced's texture is close to Iray's**: mid 0.79 – 0.92, fine 0.50 –
  0.99. Nothing structural loses it: one figure's maps are far below the
  8 GB texture budget (no downscale), the noise-free walk keeps the fine
  band at 0.79 – 1.35 (the random walk does not blur the baked colour at
  this scale).
- **What softens it is Octane's AI denoiser on the walk's grain**: at
  400 samples it takes the fine band to 0.50 – 0.83; at 2000 0.72 – 0.99
  (Monique, Victoria). The export's sample count is 500
  (`kernel_path_tracing.ocs`), so a default Enhanced render shows about
  the 400-sample row. Faithful (a plain diffuse, little grain) keeps its
  texture through the denoiser.
- **Chest and shoulder** are below Iray in both Looks (mid: Enhanced
  0.36 – 0.62, Faithful 0.70 – 0.84, against 0.95 – 1.08 on the cheeks).
  Not split further: the region carries the dual lobe's broad sheen,
  where #137 has the gloss 1.2 – 1.7 × Iray (a brighter, flatter sheen
  lowers the log contrast). Open with #137.

## 3. What is left is taste

The issue's per-texel compensation was a fitted dark gain on the old
skin colour; #19's derived rule has no gain to put it on, and the rules
say no fitted skin constant. What remains for Enhanced:

- nothing (texture as Iray's; denoised fine 0.5 – 0.8 at the default
  samples);
- more samples by default (2000: fine +15 – 25 %, mid unchanged, render
  4 – 5 × longer: Victoria 428 s against 91 s, Monique 227 s against 51 s at 960 × 720);
- an Enhanced-only contrast on the baked skin colour, colour × (D /
  D_low)^s with D / D_low the diffuse map's own detail (the mask's
  ratio, `derived_skin`): every band's contrast × (1 + s) on the albedo,
  s 0.1 – 0.2 the issue's range; a look, not a match.

Asked in #47.

## Process

- Compare like with like: a denoised reference against a raw render
  measures the noise. Any texture, highlight-peak or p99 metric on
  Enhanced renders should read the `denoised/` pass.
- Two raw sample counts give the noise-free texture without a long
  render.
- The screen-lock warning appeared; renders were fresh (timestamps
  checked).
