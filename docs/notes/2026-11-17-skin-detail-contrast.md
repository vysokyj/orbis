# 2026-11-17 Enhanced skin detail contrast (#47)

The user's call on #47 (2026-11-16): an Enhanced-only detail contrast on
the baked skin colour, colour × (D / D_low)^s, s = 0.15 for light and dark
skins alike, Faithful untouched; the final s is #179. This session
implements it and measures what it does on the four Calib-Skin fronts.

## The rule

`Tuning::skin_detail_contrast` (src/tuning.rs: Enhanced 0.15, Faithful 0).
`Textures::derived_skin` multiplies both parts of the random-walk split
(the base's colour and the walk's) by (D / D_low)^s per channel, D the
diffuse map texel and D_low its 32 px low-pass on the UV islands - the
ratio the detail mask already reads. The mask itself stays on the unchanged
ratio, so which texels leave the walk does not move. On the albedo every
band's log contrast is × (1 + s) by construction (unit test
`bakes_match_the_per_texel_formulas`, within 2 of 255 after 8-bit
encoding). The mean of (D / D_low)^s is ~1, so region levels stay put:
forehead / cheek / chest luminance over Iray within ±0.05 of the s = 0
export (`skin-compare.py`; a small cheek box's hue moves up to ±0.13,
Monique's r / g, with its grain).

## Measured (denoised pass, 400 samples, 960 × 720)

Metric of docs/notes/2026-11-12-skin-texture-contrast.md, now
`scripts/dev/skin-texture.py`: log luminance, fine = L − G(1.2 px), mid
= G(1.2) − G(4); σ per region over Iray's; median of forehead, both
cheeks, chest, shoulder (face = forehead + cheeks). s = 0 rendered from
the same HEAD build in the same session.

| fine / mid (face fine / mid) | s = 0 | s = 0.15 |
|---|---|---|
| Victoria | 0.83 / 0.76 (0.92 / 1.08) | 0.82 / 0.79 (0.94 / 1.12) |
| Kayo | 0.44 / 0.52 (0.62 / 1.05) | 0.44 / 0.53 (0.63 / 1.02) |
| Twosret | 0.42 / 0.66 (0.62 / 1.01) | 0.65 / 0.78 (0.65 / 1.07) |
| Monique | 0.45 / 0.78 (0.71 / 0.96) | 0.50 / 0.91 (0.73 / 0.94) |

Per region the fine band moves +0 – 15 % (Twosret's chest 0.42 → 0.75,
the one outlier, which takes her five-region median with it), the mid band
−5 – +17 %. Raw (`ORBIS_PASS=raw`) it is lost in the walk's grain (fine 4 –
9 × Iray either way).

- **The render gains less than the albedo's 15 %.** What a region's
  rendered log contrast holds is the albedo's part plus the shading's
  (normal and bump maps, the dual lobe's gloss) and the denoiser's
  smoothing; s scales only the first. Where the render barely moves
  (Victoria, Kayo: +0 – 5 %), the texture on screen is mostly shading;
  where it moves most (Twosret's chest, Monique's forehead), the diffuse
  map carries it. At this framing (a face ~200 px wide on a 4k map) the
  texel-scale pores average out under a pixel; D / D_low's detail reaches
  the screen mostly as the mid band (blotches, fine wrinkles).
- Faces stay at or above Iray's mid band (0.94 – 1.12); the fine band on
  the face is still 0.63 – 0.94 × Iray - the denoiser's loss is not what s
  fixes, it only adds albedo contrast on top.
- Chest / shoulder stay the lowest regions in both settings (with #137).

## Left for #179

Whether 0.15 is enough is a look call: on these fronts it is a mild
change, visible mainly on the darker skins' mid band. Judge on Calib-Skin
fronts and Demo-* figures at a closer framing (pores resolved), and
whether dark skins want their own s. Decompose first if s grows: a render
that stays flat while s rises is shading, not albedo.

## Process

- A baseline built from HEAD in a scratch worktree (separate target dir)
  gives an s = 0 export without a temporary switch in the code.
- 8 renders at 400 samples: 88 – 181 s each, ~17 min in all.
