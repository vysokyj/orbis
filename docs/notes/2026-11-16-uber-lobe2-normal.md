# 2026-11-16 Uber dual lobe: lobe 2 is smooth in Iray (#174)

#151 found the skin faces' dual lobe (dlflat − gloss0, p99.5 of the
2-pass binomial-blurred difference over the face box, Octane's denoised
pass / Iray) down from 0.75 - 0.86 (2026-11-09 export) to 0.52 - 0.79
(2026-11-15), most on Victoria and Kayo. Same measure, same references:
the export. Bisect it, then judge the culprit against the MDL. No
constant tuned (#19 / #57).

Scenes: the private Calib-Skin-<figure>-front-dlflat / -gloss0 pairs
(G8 Basic Female in store skins; renders local only) of Victoria, Kayo,
Gabriela, Nishi. enhanced-final, 960 x 720, 400 samples (the blurred
measure doesn't move between 400 and 2000, #151). gloss0 renders from
#151's `f151-400` (current master; its export didn't change, below).

## Bisect without rendering first

Instead of building and rendering every candidate, the old binary
(78e5944, the 2026-11-09 bump bake, built in a scratch worktree) and
master exported Kayo's dlflat and gloss0 from the same cached dumps, and
their `project.ocs` were diffed material by material.

- gloss0's Face: identical but for a texture's file index. The baked
  normal map is byte-identical (md5), so the bump bake, the height decode
  gamma (ae3878e) and the rest of the base didn't move. (#151's gloss0
  1.11 - 1.13 -> 1.00 - 1.02 was raw-400 grain against a denoised pass,
  not the export.)
- dlflat's Face: lobe 1's `roughness` 0.538 -> 0.544 and lobe 2's
  `coatingRoughness` 0.397 -> 0.399 (the Phong pin, #162; < 1.5 %), and
  a new `coatingNormal` with the base's normal map (50858a5, #140). On
  every Uber skin material alike.

So one structural change: lobe 2, Universal's coating, went from smooth
to bumped.

## Renders

| dual lobe, Octane / Iray (blurred p99.5, denoised) | Victoria | Kayo | Gabriela | Nishi |
|---|---|---|---|---|
| 2026-11-09 export, raw 400 (#151) | 0.751 | 0.825 | 0.858 | 0.756 |
| 78e5944 binary, now | 0.755 | 0.812 | 0.862 | 0.742 |
| master (f151-400) | 0.524 | 0.534 | 0.799 | 0.701 |
| master, `coatingNormal` removed (orbx variant) | 0.734 | 0.813 | 0.835 | 0.714 |
| fix (lobe 2 smooth on Uber) | 0.749 | 0.797 | 0.840 | 0.739 |

The old binary reproduces 2026-11-09 within 2 % in today's Octane, so
nothing outside the export moved. Taking `coatingNormal` off master's
export alone brings it back to within 3 % (not split further: the two
roughness pins moved < 1.5 %): #140's coating normal is the whole drop.

Looked at, not measured: the dual-lobe difference images side by side
(face box, same scale). Iray has a smooth glint in the middle of the
forehead and down the nose - the plain mesh's highlight, no pores in
it. The old export has it too; master spreads it into the pore pattern
and loses the peak.

## Was #140 right for Uber? The MDL

- MDL spec 21.6 (layering BSDFs): a layerer's `normal` is "applied to top
  layer". DAZ's own `pbr_skin.mdl` leans on that twice - an empty base
  in two `weighted_layer`s because a normal can't be applied to a bottom
  layer.
- Uber `add_dual_lobe_specular`: `specular_combined` =
  `weighted_layer(normal: layer_bump, layer: lobe 1, base: lobe 2)`, and
  the material's surface is `custom_curve_layer(normal: layer_bump,
  layer: base.surface.scattering, base: specular_combined)`. Lobe 1 is
  a top layer (bumped), lobe 2 the base of both. The base sees the
  material's geometry normal, and DAZ hands `geometry_bump` =
  `state::rounded_corner_normal` (2012 materials in the `.mi` fixtures,
  never a map). Lobe 2 is smooth.
- PBRSkin `pbr_material`: the dual lobe (lobe 2 the layer over lobe 1 at
  `state::normal()`) is the *layer* of the `custom_curve_layer` on the
  detail normal - a top layer, so both lobes are bumped, as #140's
  Calib-PBRSkin-Detail render showed.

#140 read Uber's `weighted_layer` on `combined_bump` as one normal for
both lobes and checked only PBRSkin scenes. For Uber the old export
(normal on `specular`, coating smooth) was the MDL's.

## Fix

Universal's / OpenPBR's lobe 2 coating takes the base's normal pins on
PBRSkin only (`universal`, `skin_gloss`); Uber's stays smooth. Test
`dual_lobe_coating_takes_the_base_normal` covers both. Spec (Dual lobe
specular) and `docs/daz/iray-uber-mdl.md` say which lobe gets which
normal.

## After the fix

The dual lobe is back at 2026-11-09's level (table above, last row). The
full faces (Calib-Skin-<figure>-front, face p99.5, denoised 400, Octane /
Iray), master -> fix:

| | Angel | Victoria | Kayo | Gabriela | Nishi | Twosret | Monique | Zurie |
|---|---|---|---|---|---|---|---|---|
| master (#151) | 1.019 | 0.881 | 0.922 | 0.941 | 0.921 | 0.850 | 0.828 | 0.943 |
| fix | 1.020 | 0.919 | 0.963 | 0.941 | 0.921 | 0.891 | 0.912 | 0.970 |

Angel, Gabriela and Nishi don't move; the others gain 3 - 10 %
(Monique most). The dual lobe stays 0.74 - 0.84 x Iray: the old level,
not explained by this issue (on the plain Calib-DualLobe spheres it is
1.06 - 1.12) - follow-up #175.

## Calib-DualLobeBump (generated, not queued)

`make-calib-scenes.py`: Uber's dual lobe over the dimples normal map by
lobe - F1 / F2 lobe 1 alone, F3 / F4 lobe 2 alone, F5 / F6 both at ratio
0.5, each without / with the map; F7 lobe 2 over the dimples as a height
map, F8 the diffuse alone over the map. In Iray, F4 should show F3's
plain glint and F2 the dimples. Not needed for this issue (the faces'
Iray references already show the smooth glint); it is in git, dumped,
for whoever wants the sphere-level check. Octane side rendered
(enhanced-final, 2000 samples, `octane/t174-spheres`): with the fix F4
shows F3's plain glint over the dimpled diffuse, F2 and F6 dimpled
highlights, as the MDL reading predicts for Iray.

## Process

- Diff the exports' `project.ocs` across binaries before rendering any:
  two exports and a material-by-material diff named the one structural
  change, so two render batches (old binary, variant) confirmed it
  instead of a bisect of ~170 commits.
- A layerer's `normal` goes to its top layer only: read which slot (layer
  / base) a BSDF sits in, nested all the way up, before giving it a
  normal.

Timings: export of a Calib-Skin face ~15 s, Octane 400 samples 60 - 130 s
a face.
