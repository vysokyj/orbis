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

## 5. Next

- One hue correction on the baked albedo (≈ 1/0.86, 1/1.20, 1/1.48).
- Top Coat: its weight map. Then OpenPBR's coat darkening, against
  Universal's.
- The dual lobe as two lobes in OpenPBR (specular + coating?), for dark
  skins.
- Then verify on all 48 scenes and Calib-Eye, and replace the trial code
  with the rule.
- The skin sphere (Universal translucency on a prop) separately.
