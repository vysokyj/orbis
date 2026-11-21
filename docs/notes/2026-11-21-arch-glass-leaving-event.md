# 2026-11-21 — Architectural glass on the leaving event; Octane's coating outside only (#214)

Follow-up of #213 (`2026-11-21-rough-entry-leaving-event.md`): resin's
Faithful solve takes Octane's excess on the event that leaves the sphere;
architectural glass (a transmit-only base under a coating, no fake
shadows) and the Uber rough glass (fake shadows on) still took the
per-ring tables. Octane only, no new Iray. Probes on Calib-Furnace-Resin2's
Faithful export `--no-denoise`, every pin of every sphere re-pinned with
`orbx-variants.py set=` (`brdf` e1, `ior`, `albedo` c0, `specular`,
`transmission`, `roughness` √α (or `s<in>,<out>`), `coating`,
`coatingIor`, `coatingRoughness`, `fake_shadows`), 1024 samples raw,
`furnace-rings.py measure`; rings centre → 0.8 – 0.9 of the silhouette.
Model: `diag214` in `src/mdl_layer.rs` (ignored tests). Octane: 55 – 125 s
per job of eight spheres.

## 1. Probes

| probe | spheres | isolates |
|---|---|---|
| `ax15` | smooth outside, rough inside (`s<pin>,0`), specular 0, IOR 1.5, α 0.2 – 0.707; the coating alone (specular 0, transmission 0, coating 1) at coatingIor 2, α 0.5 / 0.707 | the exit through a transmit-only lobe; the reflection at another IOR |
| `ax2` | the same exits at IOR 2; the coating alone at IOR 2, α 0.3; both faces rough α 0.5 IOR 2 (`OCTANE_SPHERE_PLAIN`'s row: within 0.003) | the exit by IOR; the repeat |
| `ax12` | the exits at IOR 1.2; the coating alone at coatingIor 2.62 (architectural's default F0 0.2), α 0.5 / 0.707 | the IOR trend; the reflection at architectural's IOR |
| `axc` | transmit-only bases, both faces rough: coating 2.62 α 0.5 over α 0.5 flat / q 0; coating 1.5 α 0.3 over α 0.5, q 0.5; IOR 1.2 α 0.6 flat / q 0; IOR 2 α 0.5 q 0.25; IOR 1.5 α 0.707 q 0 s 3; coating 1.5 α 0.2 over α 0.2 q 0.5 | the check |
| `fs` | `fake_shadows` 1, IOR 1.5, α 0.707: rt rough out / smooth in, rt smooth out / rough in, the same two at specular 0, a bubble (IOR 0.6711) rt, the reflection alone, rt both rough; tr smooth out / rough in α 0.5 | the Uber glass |

## 2. The transmit-only exit

Deconvolved as #213's (one pass: the probe's paths with the ratio on their
exits meet the probe), Octane / model at the ring's internal cosine:

| α | IOR 1.2 | IOR 1.5 | IOR 2 | rt 1.49 (`OCTANE_EXIT`) |
|---|---|---|---|---|
| 0.2 | 1.004 … 1.009 | 1.002 … 1.000 | 1.011 … 0.981 | 1.005 … 1.003 |
| 0.4 | 1.018 … 1.009 | 1.045 … 0.993 | 1.030 … 0.945 | 1.034 … 0.993 |
| 0.5 | 1.038 … 1.011 | 1.078 … 0.995 | 1.030 … 0.912 | 1.097 … 0.988 |
| 0.6 | 1.081 … 1.008 | 1.081 … 0.977 | 1.029 … 0.908 | 1.098 … 0.987 |
| 0.707 | 1.114 … 1.016 | 1.088 … 0.982 | 0.942 … 0.891 | 1.079 … 0.988 |

At IOR 1.5 the transmit-only exit is rt's within 1 – 3 %; it falls with the
IOR. That is why `OCTANE_SPHERE_PLAIN`'s IOR 2 `tr` rows sat 0.82 – 0.95 ×
the model while IOR 1.5's sat above it. Not a path-depth effect: the model
at IOR 2 is the same at depth 16 – 128 (`print_tr_depth`).

`OCTANE_SPHERE_PLAIN`'s `tr` rows (both faces rough, no falloff, measured
for #201 on another scene) against the leaving-event model with these
tables (`print_plain_tr_on_leaving_event`): IOR 1.5 α 0.25 – 1 within
0.975 – 1.006 (the model alone 1.05 – 1.07 head-on); IOR 2 α ≤ 0.6 0.96 –
1.00, α 0.7 0.94 – 0.98 (architectural glass never passes α 0.707:
`legacy_phong_alpha` ≤ 0.707).

## 3. The reflection at another IOR

The coating alone, Octane − model per ring (centre → rim):

| coatingIor, α | Octane − model | Octane / (model + the 1.49 excess) | Octane / (model × the 1.49 ratio) |
|---|---|---|---|
| 2, 0.3 | 0.003 … 0.000 | 1.03 … 1.01 | 1.05 … 1.02 |
| 2, 0.5 | 0.001 … 0.011 | 1.01 … 0.98 | 1.00 … 0.88 |
| 2, 0.707 | −0.002 … 0.032 | 0.95 … 0.97 | 0.92 … 0.74 |
| 2.62, 0.5 | −0.001 … 0.006 | 0.99 … 0.96 | 0.98 … 0.84 |
| 2.62, 0.707 | −0.002 … 0.022 | 0.98 … 0.94 | 0.93 … 0.64 |

The rim's excess at α 0.707 is 0.036 at IOR 1.49, 0.032 at 2, 0.022 at
2.62: about the same amount, not the same ratio. The export takes it
additively: k → 1 + (k − 1) · m(1.49) / m(IOR), m the model's single
reflection at that ring.

## 4. Octane's coating from inside

`axc` against the model as it stood (the coating met from both sides,
Fresnel at 1 / coatingIor from inside): the coated spheres at coatingIor
2.62 rendered 1.39 – 1.48 × the model at the centre, 1.09 – 1.26 at the rim
— the model's coating from inside reflects totally past 22° (sin θ = 1 /
2.62) and traps the light. Variants (`print_coat_inside`, own tracer):

| sphere | coating from inside as the model | none | Fresnel at 2.62 (as outside) |
|---|---|---|---|
| coat 2.62 α 0.5 / base α 0.5, flat | 1.59 – 1.77 | 0.97 – 1.06 | 1.05 – 1.16 |
| the same, q 0 | 1.16 – 1.60 | 0.95 – 1.01 | 1.03 – 1.07 |
| coat 1.5 α 0.3 / α 0.5, q 0.5 | 1.02 – 1.29 | 0.99 – 1.04 | 1.01 – 1.09 |
| coat 1.5 α 0.2 / α 0.2, q 0.5 | 1.01 – 1.15 | 1.00 – 1.01 | 1.00 – 1.04 |

Universal's coating sits on the outside of a closed solid only. The
export's Octane model now traces it so (`trace_cf`, `front_coat`) for
architectural glass's and resin's coats, in both Looks (a model of the
render, not a tuning); the Uber glass's reflection is Universal's specular
(both sides), unchanged. Thin walls keep #168's rule.

With the exits, the additive reflection and the outside-only coating,
`axc`'s eight spheres: Octane / model 0.95 – 1.02 per ring (U1 / U2, the
2.62 coating, 0.955 – 0.979: the worst); the per-ring tables gave 1.02 –
1.41 on the coated ones (their coat from inside), 0.98 – 1.06 on the rest.

## 5. Fake shadows (the Uber glass)

`fs` against the same spheres without: the rough entry (rt 0.998 … 0.452 v
0.999 … 0.448, tr the same), the reflection alone (0.032 … 0.088 v 0.032 …
0.083) don't change; a smooth entry with a rough inside gains 1.10 – 1.20 ×
(rt 0.935 … 0.827 v 0.852 … 0.688), an α 0.5 transmit-only one renders
1.075 at its centre (above the dome); the bubble moves +2 % centre, −6 %
rim; rt both rough α 0.707 is `OCTANE_SPHERE`'s α 0.7 row within 1 %. The
leaving-event model with the fake-shadow exit ratios (1.19 – 1.22 rt, 1.07
– 1.15 tr at α 0.707) leaves rt both rough 0.90 – 1.05 and
`OCTANE_SPHERE`'s tr α 0.5 / 0.7 0.93 – 0.98 / 0.90 – 0.99: the extra is not
on the leaving event. OTOY's manual says fake shadows trace light through
the material in the shadow calculation, ignoring refraction: our reading,
the shadow rays from events inside reach the dome through the walls, so the
gain depends on how much of a path's light comes from rough events inside.
Not modelled: the Uber glass has no falloff (one k on the whole sphere,
`tint_factor_octane`), so `OCTANE_SPHERE`, measured on its own flat
spheres, is what it renders. Its export is byte for byte as before.

## 6. Renders

Faithful, 400 samples denoised, Octane / Iray (Iray Server 5000
references in git), disk / rim 0.8 – 0.9, before (f80dd3b) → after:

| sphere | disk | rim |
|---|---|---|
| Furnace-ArchGlass G4 (refr_gloss 0.2) | 1.000 → 0.980 | 1.037 → 1.005 |
| G5 (0) | 1.026 → 1.004 | 1.009 → 0.974 |
| G7 (0.2 under refl 0.6) | 0.994 → 0.991 | 1.012 → 0.992 |
| G8 (the same, Fresnel term) | 0.993 → 0.981 | 0.984 → 0.965 |
| Furnace-ArchGlass2 H2 (0.2) | 1.005 → 0.980 | 1.039 → 1.007 |
| H3 (0) | 1.030 → 1.004 | 1.015 → 0.977 |
| H6 (0.2, IOR 1.2) | 1.020 → 0.990 | 1.042 → 0.990 |
| H7 (0.2, IOR 2) | 1.037 → 1.000 | 1.067 → 1.003 |
| H8 (0.2, colour 0.5) | 1.000 → 0.964 | 1.118 → 1.059 |
| Calib-ArchGlass (lit) 2 – 5 | 0.977 – 1.018 → 0.958 – 0.994 | 0.974 – 0.997 → 0.950 – 0.977 |
| Furnace-Resin R7 (coated grey) | 0.992 → 0.973 | 1.133 → 1.116 |
| Calib-Resin (lit) R7 | 1.042 → 1.024 | 1.416 → 1.405 |

Every other sphere (G1 – G3, G6, the thin H4 / H5, Furnace-Resin's
uncoated, -Resin2, -Resin3) moves < 1 %, but G3's rim (refr_gloss 0.4, α
0.2) 0.996 → 0.976. Enhanced (`excess` off, the
outside-only coat on): G7 / G8 disk 0.998 / 0.990 → 1.012 / 1.007, R7 1.031
→ 1.012, the rest within 0.6 %.

The split of H2 / H3 / H6 / H7 (`print_arch_split`): Octane's render over
the export's Octane model at the solved falloff 0.96 – 1.01 per ring,
Iray's reference over its `libbsdf` model 0.95 – 1.03, the two models
against each other 0.97 – 1.04 (the falloff's shape; ring 0 is 2^16 paths'
noise). The disks that fell to 0.98 (G4, H2) are the models' own
disagreement, not Octane's excess: before, the per-ring table's flat-sphere
ratios happened to sit them at 1.00 while the rims were 1.04.

Octane X crashed once between two batches (a crash report at 23:30:40);
`octane-run.sh` restarted it and the batch rendered.

## Where it stands

Shipped: architectural glass solves against Octane's excess on the
leaving event (`OCTANE_EXIT_TR` by IOR, the reflection's excess additive
across IORs); Octane's coating is traced on the outside only (both Looks,
architectural glass and resin). The Uber glass keeps `OCTANE_SPHERE`.
Left: Calib-ArchGlass's lit sphere 8 (1.30 at the centre, 0.88 at the rim,
as before: not a furnace sphere); R7's rim (#211's cut); resin at an IOR
far from 1.49 still takes the 1.49 exit table (rt at IOR 2 sits 0.86 – 0.92
of the model with it; no resin in our scenes is there).

## Process lessons

- Before blaming an excess, check the model's own layering against a
  probe with a known coating: the coated spheres' 1.3 – 1.5 × was the
  model's coating from inside, not Octane's lobe.
- A ratio table measured at one IOR is not a ratio at another: check
  whether the excess is a ratio or an amount (one sphere at a second IOR
  tells).
- The excess with fake shadows is a different mechanism: one probe (`fs`)
  against its no-fake-shadow twin decides whether a model carries over.
