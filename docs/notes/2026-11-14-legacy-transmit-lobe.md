# 2026-11-14 — The legacy transmit lobe: compare, second round parked (#117)

Resume of #117 with Calib-Furnace-MdlTir / -MdlResin (and #112's shared
-MdlVersion / -MdlTransmit) rendered (Iray Server, 5000 samples, 91b1388).
Question: is the angular residual of NVIDIA's rough transmission (Iray
under `libbsdf` × τ, falling to the rim on closed spheres) the `mdl 1.0`
lobe, and is it total internal reflection the legacy lobe kills? Measure:
`furnace-rings.py measure` (sphere / dome per ring of the silhouette,
denoised reference as in git); model: `furnace-rings.py model` (the numpy
port of `mdl_layer.rs`). No Octane render this session.

## 1. `mdl 1.3` is `libbsdf`, ring for ring

| sphere | Iray 1.3, centre → 0.8 – 0.9 | `libbsdf` model |
|---|---|---|
| X2 closed r 0.2 IOR 1.2 | 1.000 … 0.981 0.953 | 1.000 … 0.992 0.961 |
| M8 / T7 closed r 0.2 IOR 1.5 | 1.000 … 0.956 0.917 | 1.000 … 0.967 0.927 |
| X4 closed r 0.2 IOR 2 | 1.000 0.990 … 0.907 0.867 | 0.999 … 0.888 |
| T5 / T6 / T8 closed r 0.05 / 0.1 / 0.4 | rim 0.994 / 0.969 / 0.776 | 0.998 / 0.982 / 0.782 |
| X6 / X8 thin r 0.2 / 0.4 | 1.000 → 0.950, 0.906 → 0.855 | 0.998 → 0.958, 0.907 → 0.865 |
| R5 – R8 resin rt r 0.25 / 0.5 / 0.75 / 1 | 0.981 → 0.866, 0.888 → 0.690, 0.685 → 0.552, 0.538 → 0.445 | 0.995 → 0.882, 0.891 → 0.694, 0.697 → 0.563, 0.534 → 0.451 |

Within 1 – 2 % per ring everywhere (8-bit steps near white). The tracer
in `mdl_layer.rs` is Iray's current `simple_glossy_bsdf`, TIR reflected
(release notes, Iray 2020.1: "allow total internal reflection for glossy
BSDFs with mode `df::scatter_transmit`").

## 2. NVIDIA's resin base *is* the plain 1.0 lobe

Calib-Furnace-Resin2 U3 – U6 (NVIDIA's `resin_polyurethane_matte`,
uncoated) against our own `mdl 1.0` module R1 – R4 at the same r:

| r | U disk | R disk | U centre → rim | R centre → rim |
|---|---|---|---|---|
| 0.25 | 0.913 | 0.913 | 0.959 → 0.803 | 0.959 → 0.805 |
| 0.5 | 0.687 | 0.687 | 0.783 → 0.517 | 0.783 → 0.518 |
| 0.75 | 0.452 | 0.453 | 0.538 → 0.312 | 0.538 → 0.313 |
| 1 | 0.317 | 0.317 | 0.376 → 0.215 | 0.379 → 0.216 |

Identical to 0.003 in every ring. `RESIN_LOBE` (NVIDIA's disk) is the
legacy lobe: nothing in DAZ's wrapper or the module, only the version.

## 3. The legacy lobe isn't TIR killed

1.0 / 1.3 per ring (centre → 0.8 – 0.9):

| pair | ratio |
|---|---|
| closed r 0.05 / 0.1 / 0.2 / 0.4 (IOR 1.5) | 0.998 → 0.965, 0.989 → 0.956, 0.969 → 0.933, 0.923 → 0.840 |
| closed r 0.2 IOR 1.2 / 2 | 0.979 → 0.950, 0.962 → 0.916 |
| thin r 0.2 / 0.4 | 0.943 flat → 0.940, 0.902 → 0.841 |

`--tir-dies` against the 1.0 rings: IOR 1.2 rim 0.907 v Iray 0.905 but the
centre 1.000 v 0.979; IOR 1.5 rim 0.753 v 0.856, IOR 2 far under (0.61 v
0.79); a thin wall can't totally reflect and still loses 6 – 10 %. Dead.
Nor a rougher lobe: `libbsdf` at a larger r keeps the centre (closed r 0.3
0.999) and a thin wall rises towards the rim (r 0.6 0.718 → 0.785) where
1.0 falls.

**What fits: `libbsdf` × a per-event loss g(r, cos), the same with or
without refraction and at every IOR.** From the thin walls (both events
at the ring's angle): g(0.2) ≈ 0.975 flat, g(0.4) ≈ 0.95 − 0.07 (1 − cos).
Applied to every event of a closed sphere (the model's `--loss`) it
predicts the 1.0 closed spheres: IOR 1.2 0.950 … 0.909 v 0.979 … 0.905,
1.5 0.951 … 0.861 v 0.969 … 0.856, 2 0.949 … 0.804 v 0.962 … 0.794, r 0.4
0.872 … 0.634 v 0.888 … 0.652 (2 – 3 % under). Whether g takes the cosine
on the outside or the inside of the event doesn't show (0.634 v 0.623).
So #148's "Iray's loss grows a little with the IOR" is `libbsdf`'s longer
bent paths under a per-event g, not an IOR term. The reflect lobe's g
(#112's MdlLobe) is smaller head-on (0.98 at r 0.4 v 0.95).

Not derivable: the release notes name "the legacy behavior of
`df::simple_glossy_bsdf`" (controllable through `IMdl_configuration`)
without saying what it is; the MDL spec has no version note on the df.
g is a measured per-event table, and stays one.

## 4. Architectural glass isn't the 1.0 lobe at its formula's roughness

Calib-Furnace-ArchGlass G3 (refr_gloss 0.4, formula r 0.051) 0.936, 0.962
→ 0.846 = the 1.0 lobe at r 0.2 (T3 0.941, 0.969 → 0.856), not at 0.05
(T1 0.966, → 0.959); its thin twin (ArchGlass3 J4 0.936, 0.943 → 0.885) =
X5 (thin r 0.2: 0.939, 0.943 → 0.893). That is ~3.7 × the formula — the
export's fitted `ARCH_GLASS_ROUGH`. At refr_gloss 0.2 (r_t 0.5) G4 keeps
0.609 and H4 (thin) 0.506 flat, below anything the 1.0 lobe at r ≤ 0.4
showed. So architectural's lobe = the legacy lobe at a roughness ~3.7 ×
`architectural_gloss_to_rough`'s, while the same function on the
reflection reads as the source (#114: g 0 ≈ 0.33 v 0.354). Three
candidates, one scene: the `base::` call inside a transmit lobe, the
module's layer structure (`weighted_layer(transparency, weighted_layer(0,
diffuse_transmission, lobe), diffuse)` under the reflection's curve
layer), or neither (then the 3.7 is the lobe's own reading of r in
transmission, and our own constant-r modules would show it — they don't
at r 0.05: T1's rim is 0.959, G3's 0.846).

## 5. Second round (parked)

- **Calib-Furnace-MdlArch** (`orbis_arch_v10` / `_v13`): A1 – A3 1.0
  transmit lobe at `base::architectural_gloss_to_rough(0.4 / 0.2 / 0)`, A4
  1.3 at (0.2), A5 1.0 architectural's structure at (0.2), A6 the
  structure at the constant 0.134, A7 / A8 thin lobe / thin structure at
  (0.2). Reads: A1 = G3 → the call; A1 = T1 and A5 = G3-like (A2 v A5 for
  0.2) → the structure; A5 = A6 separates the call from the structure.
- **Calib-Furnace-MdlLegacy** (`orbis_legacy_v10` / `_v13`): the 1.0
  lobe closed at r 0.3 / 0.5 / 0.7 / 1, thin at 0.1 / 0.6 / 1, one 1.3
  twin (r 0.5, `libbsdf` predicts disk 0.843, 0.906 → 0.719) — g above r
  0.4, where architectural and resin live.

Both `.mi` (headless `daz-export-mi.dsa`) resolve all sixteen spheres to
`mdl::orbis::orbis_arch_v10 / _v13 / orbis_legacy_v10 / _v13::…` under
`pass_material`.

## Where it stands

No export change. The residual B is named (the 1.0 lobe) and has a shape
(per-event g(r, cos), IOR- and wall-independent), but g is known only to r
0.4 and architectural's roughness isn't settled; the Falloff Mix solve
(note 2026-11-13 § 3) waits for both, so it is solved once, from g, for
resin and architectural together.

## Process lessons

- The TIR guess was checked offline first (`--tir-dies` on the old
  model) and would have stayed "maybe" without the thin walls in the same
  scene: a variant that can't have the mechanism decides faster than more
  IORs.
- Cross-scene twins (U3 – U6 v R1 – R4) agree to 0.003; a hand-written
  twin of a store module's df answers "is it the module" in one frame.
