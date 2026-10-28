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

## 4. Next

- Measure Octane's random-walk transfer on primitives: OpenPBR SSS spheres
  over a grid of colours, radii and weights, with no maps. Octane only,
  minutes a round. Then invert it.
- Combine that with Iray's model above and the gloss fix, and verify on
  all 48 scenes.
- The skin sphere (Universal translucency on a prop) separately.
