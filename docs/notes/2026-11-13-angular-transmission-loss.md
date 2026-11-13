# 2026-11-13 — The angular spread of Iray's rough-transmission loss (#117)

Question (#117, from #115): the export scales a rough transmissive base by
one factor per channel, solved so a closed sphere in a white furnace keeps
in Octane what it keeps in Iray. The whole sphere matches, the spread over
it doesn't. Decompose the ring profiles — Iray, the `libbsdf` model (with
#115's measured per-event tint τ), Octane's model, Octane's render — and
find what Octane could draw an angle-dependent loss with. Octane
enhanced-final 1024 samples, denoised pass; no Iray this session.

## Tools

`scripts/dev/furnace-rings.py`: `measure` gives radiance / dome per ring of
the entry radius (0 – 0.1 … 0.8 – 0.9) and the disk r < 0.75 for the eight
furnace spheres (spans measured on the Iray references; gamma 2.2; the
8-bit references step 0.962 / 0.981 / 1 near white); `model` is a numpy
port of `src/mdl_layer.rs`'s tracer (Phong + v-cavities for `libbsdf`,
Beckmann + Smith for Octane, `fresnel_layer` / `custom_curve_layer`
weights, thin walls, a per-event `--tau` or `--loss` in the event's
cosines) with the same rings. Checked on the Uber (Calib-Furnace-UberGlass,
`libbsdf` exactly per #116): transmit r 0.354 Iray 0.981 → 0.802, model
0.985 → 0.820; r 0.6 0.834 → 0.644 v 0.850 → 0.651; reflect_transmit r 0.5
0.877 → 0.686 v 0.887 → 0.693 (≤ 0.02 per ring).

## 1. Octane side rendered fresh (current code)

Calib-Furnace-ArchGlass, -ArchGlass2 (its dump cloned from -ArchGlass's,
labels H1 – H8) and -Resin2, enhanced-final. Octane / Iray per ring,
centre → 0.8 – 0.9:

| sphere | Octane / Iray, rings |
|---|---|
| G4 refr_gloss 0.2 | 1.02 1.02 1.02 1.03 1.04 1.05 1.08 1.13 1.20 |
| G5 refr_gloss 0 | 0.87 0.89 0.90 0.94 0.99 1.05 1.12 1.20 1.26 |
| G7 refl 0.6 Schlick | 1.00 1.00 1.00 1.01 1.01 1.03 1.05 1.09 1.15 |
| G8 refl 0.6 Fresnel term | 1.18 1.17 1.17 1.17 1.17 1.17 1.16 1.14 1.08 |
| U4 resin r 0.5 | 1.03 1.03 1.03 1.03 1.04 1.06 1.08 1.12 1.18 |
| U5 r 0.75 | 1.06 1.06 1.07 1.08 1.10 1.14 1.20 1.31 1.44 |
| U6 r 1 | 1.06 1.04 1.06 1.08 1.12 1.18 1.27 1.41 1.61 |
| U8 coated r 0.25 | 0.80 0.82 0.84 0.86 0.90 0.94 0.99 1.06 1.24 |

Everywhere Iray falls towards the rim more steeply than Octane. Two side
findings, filed separately:
- **#147**: architectural's `thin_walled` isn't exported — H4 / H5 render
  exactly as the closed H2 / H3 (0.729 → 0.507), Iray's are thin walls
  (0.52 → 0.50 flat, 0.386 flat).
- **#148**: `tint_factor_measured` solves τ at the material's IOR, but the
  lobe tables were measured at 1.5: H6 IOR 1.2 0.89 ×, H7 IOR 2 1.32 × on
  the disk. τ held at its 1.5 value (0.861) and the model traced at the
  real IOR predict Iray's disk within a few per cent (0.696 v 0.713, 0.489
  v 0.461).

## 2. Decomposed: what `libbsdf` explains, what is left

Iray / the `libbsdf` model (τ constant per base event, as the export
assumes), per ring:

| sphere | model rings | Iray / model, centre → rim |
|---|---|---|
| U8 coated resin r 0.25 (τ 0.974) | 0.822 → 0.527 | 1.01 1.00 0.99 0.99 1.00 0.98 0.98 0.97 0.95 |
| G8 Fresnel term over refr_gloss 0.2 (τ 0.861) | 0.460 → 0.413 | 1.00 1.02 0.99 0.99 0.98 0.98 0.97 0.94 0.91 |
| G4 refr_gloss 0.2 alone (τ 0.861) | 0.679 → 0.495 | 1.06 1.04 1.03 1.02 1.01 0.97 0.95 0.91 0.86 |
| U4 uncoated r 0.5 (τ 0.917) | 0.745 → 0.557 | 1.05 1.04 1.04 1.03 1.02 1.00 0.98 0.95 0.92 |
| U5 r 0.75 / U6 r 1 | — | centre 1.03 / 1.05 → rim 0.75 / 0.71 |

- **The layered spheres are `libbsdf`'s** (A, derivable): U8's steep fall
  (0.83 → 0.50) and G8's flat profile are `fresnel_layer`'s macro-normal
  weights, to 1 – 5 % per ring. Octane's physical coating keeps a flat
  0.67 on U8 at the whole-sphere factor (centre 0.80 ×, rim 1.24 × Iray).
  Nothing measured: the angular factor follows from the two models.
- **The lobes alone keep an unexplained angular residual** (B): NVIDIA's
  1.0 modules lose towards the rim beyond `libbsdf` × τ, more with
  roughness (r 1: 0.71 at the rim). On thin walls the loss is flat in
  angle (H4: 0.51 at every ring; `libbsdf` ~1), on closed spheres it falls,
  so it isn't a function of the outside angle alone (both events of a
  closed sphere meet the same outside angle).
- Dead end / open hypothesis: the legacy lobe killing totally reflected
  samples of a transmit-only lobe (`libbsdf` reflects them). It needs no
  refraction to vanish on a thin wall, grows with IOR and roughness; with
  τ on top it flattens H2's residual at the export's 3.7 × roughness
  (0.79 → 0.77) but over-corrects IOR 2 (0.79 → 0.90) and under-corrects
  1.2 (0.82 → 0.69), and at the MDL's own roughness it leaves most of the
  centre loss. Not decidable offline — Iray decides (Calib-Furnace-MdlTir).

## 3. Octane: what can draw a per-event angular factor

Falloff Map (50, normal vs. eye ray: (θ / 90°)^skew between two values,
already in the export for thin-film tints) into a Mix on `transmission`.
Probe (orbx variant `set=<material>.transmission=o<head>,<grazing>,<skew>`,
new in `orbx-variants.py`) on Calib-Furnace-ArchGlass2:

| sphere | Octane | per hit (1 − θ_in/90)(1 − θ_t/90) | camera ray twice |
|---|---|---|---|
| H1 near-smooth, 1 → 0 | 0.930 … 0.300 0.206 | 0.928 … 0.305 0.212 | 0.917 … 0.211 0.122 |
| H6 IOR 1.2, 1 → 0 | 0.940 … 0.214 0.123 | 0.902 … 0.216 0.126 | 0.915 … 0.180 0.094 |
| H2 0 → 1 (inverted) | 0.004 … 0.161 0.202 | 0.004 … 0.138 0.169 | 0.002 … 0.203 0.261 |

The falloff is taken per hit at that hit's own incoming ray — inside a
solid, the refracted ray's angle — so a Falloff Mix on `transmission` is a
per-event factor g(cos θ_hit), the same structure as both components
above. Cost: two texture nodes per material. The numpy Smith model is
Octane to ~1 % near-smooth and up to 5 – 10 % at α 0.5 / IOR 2 (Octane
brighter).

## Where it stands

No export change. The construction is known (Falloff Mix on the
transmission, its head-on / grazing / skew solved per material so the
Octane model's rings meet the Iray model's), but solving it now would bake
in a constant τ for component B, whose angular law is exactly what is
unknown. Parked for Iray: Calib-Furnace-MdlTir (closed transmit lobe r 0.2
at IOR 1.2 / 2.0, thin-walled r 0.2 / 0.4, each `mdl 1.0` / `1.3`) and
Calib-Furnace-MdlResin (resin's reflect_transmit r 0.25 / 0.5 / 0.75 / 1 at
1.0 / 1.3, U3 – U6's twins), next to #112's batch (MdlVersion's M7 / M8,
MdlTransmit). `libbsdf` predictions for them (rings, centre → rim): X ior
1.2 1.000 → 0.961, ior 2 0.999 → 0.888, thin r 0.2 0.998 → 0.958, thin r
0.4 0.907 → 0.865; R r 0.25 / 0.5 / 0.75 / 1 0.995 → 0.882, 0.891 →
0.694, 0.697 → 0.563, 0.534 → 0.451. With TIR killed: ior 1.2 → 0.907,
1.5 → 0.753, 2 → 0.607 at the rim, thin unchanged.

#118 (derive the Uber rough glass level from `libbsdf`): the Uber's rough
transmission is `libbsdf` per ring (§ Tools), so its derivation has no
residual B; only the layering part A applies, and the per-hit falloff
works for it too.

## Process lessons

- Re-render the Octane side before decomposing: two of the three
  findings in § 1 (#147, #148) are in the export, not in Iray.
- A probe of a texture's semantics needs one near-smooth sphere: its rings
  separate per-hit from camera-ray to a factor 2 at the rim.
- An orbx variant (`orbx-variants.py set=…`) beats a Lua probe for a
  node construction: no Octane scripting, the render as exported.
