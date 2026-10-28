# 2026-10-28: skin tones (#19), Iray Server

Commits `8133c77`, `2f7eeeb`, `684ec74`, `3fab01a`; issues #19, #46.
Enhanced + Final. Work in progress: no export rule changed yet.

## 1. Calib-Skin

`scripts/dev/make-skin-scenes.py` with `daz-build-skin-scene.dsa`.

**Setup**
- One Genesis 8 Basic Female, shape and pose untouched, wearing one
  character's full material preset ("All Maps" / "Full Mat"). Only the
  skin differs between scenes.
- Eight skins:

  | figure | preset | torso map patch, linear luminance |
  |---|---|---|
  | Angel | Angel Full Mat | — |
  | Victoria 8 | All Maps | 0.67 |
  | Kayo 8 | All Maps | 0.37 |
  | Gabriela 8 | All Maps | — |
  | Nishi | Nishi 00 All Maps | — |
  | Twosret 8 | All Maps | 0.17 |
  | Monique 8 | All Maps | 0.07 |
  | Zurie | All Maps | — (pale grey fantasy skin) |

- Josephene 8 was left out: its torso uses UDIM 1004, another UV layout.
- No hair.
- Beside the head, two 16 cm spheres:
  - a matte 18 % grey (sRGB 0.46): checks the light and exposure;
  - the Torso material: its maps cropped to one even patch (u 0.28–0.40,
    v 0.52–0.64 of the torso map, below the breast beside the navel).
    It is a prop, so it shows whether the skin rule needs a figure.
- Camera: 130 mm at 1.5 m, 960×720.
- Lights:
  - front: distant light, x −30°;
  - back: x −20°, y 180°;
  - hdri: Calib-HDRI's dome and tone mapping.
- Each scene also at Translucency Weight 0 (`-w0`).
- 48 scenes. They use DAZ Store skins, so they stay in My Library (not in
  `calib/scenes`).

**Fixes on the way**
- The primitive's size option is `OuterDiameter` (m), not `Size`; its
  origin is its bottom.
- A preset applies to the selected node's *selected surfaces*: select the
  materials first.
- The camera needs its `Focal Length` channel set as well as `perspective`.
- The cropped maps must not keep `_1002` in the name. Iray takes that for a
  UDIM tile, puts the map on UV tile 1002 (off the sphere's 0–1 UVs) and
  leaves the sphere white.
- The sphere's library `diffuse` channel must keep its definition
  (`"mappable": true`). Replaced by the scene copy it lost the map: the
  sphere rendered white while its SSS map, from the merged channels with
  full definitions, showed.

**Timings**
- Iray on the CPU, 400 samples: 60–90 s at w 0, 115–165 s with
  translucency. 48 scenes in 92 min.
- Octane, 400 samples: 20–76 s a scene.

## 2. Findings

**Light and surface.** Octane / Iray, median of forehead, cheeks, nose,
neck, chest, shoulder:

| | Y | r/g | b/g |
|---|---|---|---|
| grey sphere | 0.99–1.07 | — | 0.98–1.03 |
| skin at w 0, all eight figures | 0.95–1.07 | 0.90–1.01 | 0.98–1.21 |

So the light and the surface (diffuse + gloss, Universal path) are right.

**The skin (SSS) path at the time of measuring.** Front light:

| figure | Y |
|---|---|
| Zurie | 0.67 |
| Nishi | 0.91 |
| Angel | 1.05 |
| Gabriela | 1.25 |
| Victoria | 1.36 |
| Twosret | 1.70 |
| Kayo | 1.78 |
| Monique | 1.85 |

- Octane is less red (r/g 0.62–1.04) and the darker skins are bluer (b/g up
  to 1.39).
- Backlit ears: Y 0.45–0.88, r/g 0.15–0.26. Iray's ears glow red, Octane's
  hardly at all.
- F = render(w) / render(w 0) per channel is the same under the front light
  and the HDRI within ~3 %, so it behaves like an albedo factor.

**Iray's skin model.** Probes on the front light:
- `tcdiff`: w 0 with the translucency map as the diffuse, giving the local
  Tc.
- `dwhite`: no diffuse map.
- `tcwhite`: no translucency map.
- `w05`: w 0.5.

Results:
- The diffuse layer is D·(1 − w)^1.3. With w 0.9 and w 0.5 the model
  predicts w 0.5 at 0.94–1.03 on Victoria, Monique and Zurie.
- The subsurface part S, per channel, is proportional to D · Tc within a
  figure. S / (D·Tc), L = 0.71:

  | figure | S / (D·Tc), r / g / b |
  |---|---|
  | Victoria | 0.51 / 0.50 / 0.49 |
  | Kayo | 0.56 / 0.54 / 0.51 |
  | Zurie | 0.64 / 0.64 / 0.62 |
  | Monique | 1.19 / 1.36 / 1.31 |
  | Angel (Base Color Effect 1) | 0.39 / 0.42 / 0.25 |
  | Nishi (Base Color Effect 1) | 0.59 / 0.53 / 0.42 |

- The scalar falls with the material's brightness:
  c ≈ 0.27 · lum(D·Tc)^−0.42 (rms 14 %), or 0.31 · lum(D)^−0.62
  (rms 13 %).
- So Iray's skin albedo ≈ **D · [(1 − w)^1.3 + w · c · Tc]**: the old
  export's structure ½ · ((1 − w) + w · Tc), with c in place of ½.
- Dead ends:
  - A separable S = S_w · Tc^a · D^b: the exponents jump between figures.
  - S independent of D: true only for the light Victoria. Monique's S
    rises 1.6–4× with a white D.
- The volume reflectance from the Transmitted Color by van de Hulst
  (albedo 0.997 / 0.919 / 0.888 → R 0.88 / 0.52 / 0.46) has roughly the hue
  of S without the Tc map. But fed to Octane additively it turned skins
  blue (b/g 1.13–1.41).

**Octane's side.**
- **The skin's gloss.** The OpenPBR skin got only `roughness`, and that
  unsquared (0.548 for Iray's 0.3). It used OpenPBR's default specular 1 at
  IOR 1.5 and had no Top Coat. The same skin at w 0 (Universal) gets the
  dual lobe's 0.65 at roughness 0.3 and the Top Coat 0.45 at 0.0625. With
  the gloss as Universal's (`ORBIS_TMP_SPEC`, uncommitted):
  - Monique 1.85 → 1.40, Kayo 1.78 → 1.59, Twosret 1.70 → 1.56;
  - but Gabriela 1.22 → 0.69 and Zurie 0.68 → 0.57.
- **Octane doesn't render the colour it is given.** The baked albedo
  (`ORBIS_TMP_SKIN`, uncommitted) came out at 0.73–1.83 × what was fed, per
  figure and channel. Kayo and Monique gained green and blue (1.4–1.8×).
  The transfer isn't one function of the fed colour, so it needs measuring
  on its own.
- **The skin sphere** (the same Torso material on a prop, Universal path)
  is off too: Y 0.79–1.32, r/g 0.66–0.91, b/g 1.05–1.37.

## 3. Iray Server (Boost for Daz)

- The Bridge connection exists only in the DAZ Studio that made it.
  Connect and Rendering Enabled are panel widgets that a script can't
  reach: `setBridgeConfiguration` takes only Connection, Server, Port,
  Username, Password and Streaming, and panes have no `findChild` or
  `children()`.
- A second launch of DAZ Studio with a script starts its own instance,
  even under the same `-instanceName`.
- So a queue script runs in the connected DAZ Studio (Render > ORBIS Iray
  Queue, `daz-render-queue.dsa`), and `iray-queue.sh` feeds it capped
  scenes.
- RTX 4000:
  - Calib-Thin at 100 samples took 16 s, ~2.5 s of it connecting and
    uploading;
  - against the CPU reference: cards 0.98–1.01, mean difference 0.3 of 255.
  - A figure's first upload is ~1.05 GB in ~2 min; later scenes of the same
    figure upload little.
  - A render at 400 samples takes ~12–20 s (CPU: ~140 s).
  - One scene waited 15 min (Gabriela tcdiff), probably the transfer.
- If the connection drops, DAZ renders on the CPU without a word: watch the
  render times.

## 4. Octane's random walk on primitives; the Tc² model

**The tool.** `scripts/dev/sss-response.py` replaces Calib-SSS's eight
sphere materials:
- greys 0.05–0.8 and two skin tones;
- as a matte Universal, as OpenPBR without subsurface, and with full
  subsurface (skin radius, white radius, ×5 radius, weight 0.5);
- one Octane round takes ~4 min.

**OpenPBR without subsurface** = Universal (1.00).

**Full subsurface at the skin radius** (1.1 mm, scale 1 / 0.43 / 0.39)
renders a colour a as a · T(a):

| a | 0.05 | 0.1 | 0.2 | 0.4 | 0.6 | 0.8 |
|---|---|---|---|---|---|---|
| R (longest radius) | 0.95 | 0.93 | 0.92 | 0.84 | 0.73 | 0.59 |
| G / B (radius ~0.4) | 0.68 | 0.71 | 0.73 | 0.73 | 0.61 | 0.50 |

- The skin tones fall on the same curves.
- White radius: 0.90 → 0.55.
- ×5 radius: 1.0–1.1 → 0.63.
- Weight 0.5: flat 0.67–0.73 whatever the colour.

**On the faces, the sphere's inverse overshoots.** Inverting T made the
figures 20–45 % less red than Iray: G and B lose less on a face than on
the slab.

**Iray: the subsurface part is w · k · Tc², not c · D · Tc.**
- c had seemed to fall with the material's brightness. Estimated from
  whole-map means it misled: the padding outside the UV islands puts
  Monique's maps at lum 0.144, her skin at ~0.03.
- k = S / Tc² on all eight figures:

  | | R | G | B |
  |---|---|---|---|
  | range | 0.45–0.65 | 0.44–0.66 | 0.27–0.63 |
  | mean | ~0.58 | ~0.54 | ~0.45 |

- It matches Victoria's probe: a white diffuse map left S unchanged.

**Octane with albedo = (1 − w)^1.3 · D + w · 0.55 · Tc²** (baked per
texel, subsurface 1, gloss as Universal's, no inverse):
- Y 0.91–1.23 on six figures.
- Out of line: Gabriela 0.66, Monique 1.64.
- Hue the same on every figure: Octane / Iray 0.86 / 1.20 / 1.48 per
  channel, one correction.
- Gabriela has Top Coat 1 with a weight map. The export ignores the map
  (coating 1 everywhere), and OpenPBR's coat darkens the base where
  Universal's doesn't.
- Monique's very dark skin is mostly gloss: the dual lobe as one OpenPBR
  lobe.

**State.** The trial code is behind environment variables and
uncommitted in `src/material.rs`:
- `ORBIS_TMP_SKIN`: the baked albedo, with `_P` / `_K` / `_NOINV`;
- `ORBIS_TMP_SPEC`: the gloss as Universal's.

## 5. Rounds 3–8: gloss, coat, brows

Eight figures, front light; Octane / Iray, median of the face regions.

| round | change | Y range | notes |
|---|---|---|---|
| export | today's rule | 0.68–1.85 | |
| 3 | albedo (1 − w)^1.3·D + w·0.55·Tc², hue × (1.16, 0.83, 0.68), coat darkening off, Top Coat weight map | 0.85–1.59 | five figures 0.98–1.14, hue ±8 %; Gabriela 0.66 → 1.01 |
| 4 | skin gloss ~0.03 (meant ×0.5; the variant script's `x` on a float pin wrote 0.034) | 0.88–1.44 | **dead end**: flat, matte faces |
| 5 | subsurface detail from D / D_low | as round 3 | no effect on the brows |
| 6 | Dual Lobe Specular Reflectivity's map as the gloss weight | 0.85–1.52 | Twosret 1.31 → 1.16; Gabriela's forehead highlight 2.08 → 1.42 (Iray 1.28) |
| 7 | detail map as the coating's colour, coating 1 at IOR 1 | — | **dead end**: brows went light, a bright rim round the head |
| 8 | detail map (D / D_low, ≤ 1) as the subsurface weight | 1.00–1.42 (four figures) | brows dark as in Iray |

**Details**
- **Coat darkening.** OpenPBR's coat darkens the base to 0.58–0.80 of
  Universal's; `coatingDarkening` 0 makes them equal (1.00, Calib-SSS).
- **Gloss.** Highlight contrast (p95 / median luminance) on the dark
  skins' foreheads is higher in Iray (Monique 1.86, Twosret 1.57) than in
  Octane (1.49, 1.35). Iray does not dim the gloss of translucent skin.
  Region medians miss highlights: judge gloss on images and highlight
  contrast, not medians.
- **Brows and smudges.** Octane's random walk bleeds the skin's light
  under dark brows and make-up lines (blue-grey smudges). Universal at w 0
  keeps them dark. The fix is less subsurface where the detail map is
  dark.
- **Left:** Monique 1.42; Zurie 0.85, too red (r/g 1.25).

## 6. Rounds 9–12, the rule

**Rounds**

| round | change | front-light Y (Victoria / Kayo / Twosret / Monique) | notes |
|---|---|---|---|
| 9 | random-walk gain compensation v · (v / 0.3)^0.11 | 0.99 / 1.06 / 1.02 / 1.25 | |
| 10 | gain 0.2 | 0.99 / 1.00 / 0.94 / 1.12 | grainy dark skins (compensation per texel), Zurie orange |
| 11 | gain from the smooth colour, hue correction scaled by saturation | 0.99 / 1.00 / 0.94 / 1.11 | Zurie r/g 1.40 → 1.18 |
| 12 | detail mask through a smoothstep; Tc · Tc_low | 1.00 / 0.99 / 0.92 / 1.17 | the dark skins' texture still stronger than Iray's: not the mask, not Tc² (gloss on the normal map? next) |

The per-texel gain's stronger wrinkles looked *more* realistic to the user
than Iray's: an Enhanced idea, #47.

**The rule** (commit after this note). On OpenPBR with chromatic SSS:
- colour = `(1 − w)^1.3 · D + w · 0.55 · Tc · Tc_low`;
- × hue (1.16, 0.83, 0.68)^(saturation / 0.75);
- × (smooth colour / 0.3)^0.2;
- subsurface = the detail mask;
- gloss as Universal's with the reflectivity map;
- coat darkening 0.

**Verification**, Octane / Iray, face and chest medians:

| light | Y | export before | hue |
|---|---|---|---|
| front | 0.80–1.17 | 0.68–1.85 | |
| HDRI | 0.89–1.31 | 0.77–2.39 | Kayo r/g 1.28, Twosret b/g 1.41 |

- Ears, backlit, front scenes: 0.79–1.87 (export 0.64–2.36).
- **Calib-Eye (Genesis 8 base skin: mono SSS, w 0.5, Base Color Effect
  1, glossy layer) went 1.17 → 1.76** under the rule. None of the eight
  had mono SSS, so the rule applies to `SSS Mode` 1 only; mono skins keep
  the older tint with the OpenPBR lift (Calib-Eye 1.17 again).
- Calib-HDRI's dump is format 1 (old): not re-exported.

## 7. Next (series for a night)

- Dark skins' texture stronger than Iray's: variants without the normal
  map, without bump, without gloss.
- Mono-SSS skins (G8F base): Iray probes like Calib-Skin's on a mono skin
  (the server), then a rule.
- HDRI hue (Kayo too red, Twosret too blue); Zurie 0.80.
- Faithful (Standard Surface) on Calib-Skin.
- Baking speed: about 30–60 s a figure for 4K maps.

## 8. Earlier list

- One hue correction on the baked albedo (≈ 1/0.86, 1/1.20, 1/1.48).
- Top Coat: its weight map. Then OpenPBR's coat darkening, against
  Universal's.
- The dual lobe as two lobes in OpenPBR (specular + coating?), for dark
  skins.
- Then verify on all 48 scenes and Calib-Eye, and replace the trial code
  with the rule.
- The skin sphere (Universal translucency on a prop) separately.
