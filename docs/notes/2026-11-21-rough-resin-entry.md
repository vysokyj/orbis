# 2026-11-21 — Rough resin rims: `LEGACY_LOBE` 0.7 / 0.75 refit, the rough entry (#212)

Follow-up of #211 (`2026-11-21-resin-coat-falloff.md`, §§ 6, 8). Two
parts, no Iray: Iray's model over Iray at r 0.75 (U5's rim), and Octane
over its Smith model under the transmission falloff on rough spheres.
Octane X, Faithful / Enhanced final; probes on Calib-Furnace-Resin2's
Faithful export `--no-denoise`, re-pinned (`orbx-variants.py set=`), 1024
samples raw; furnace rings with `furnace-rings.py measure` (projected
discs).

## 1. `LEGACY_LOBE` under today's trace (`diag212::print_legacy_refit`)

The closed `mdl 1.0` spheres at r ≥ 0.6 (Iray Server 5000 references in
git, measured again): Calib-Furnace-MdlLegacy2 L1 / L3 / L4 (r 0.6, IOR
1.05 / 1.2 / 2), -MdlLegacy K3 (0.7, 1.5), -Resin2 U5 (0.75, rt 1.49),
K4 / L2 / L7 / U6 (r 1). `trace` now records each base event's incoming
cosine (`Path::lost`), so one trace per sphere (2^19 paths) is reweighted
by any g; Nelder – Mead (twice) on the summed squared ratio, rings centre
→ 0.8 – 0.9, equal weights (#191's method).

| row | #191's row | its error | refit | error | worst ring before → after |
|---|---|---|---|---|---|
| 0.6 | 0.907 / 0.735 / 1.522 | 0.00866 | 0.907 / 0.725 / 1.556 | 0.00818 | L4 rim 1.043 → 1.034, L1 0.963 → 0.954 |
| 0.7 | 0.885 / 0.628 / 1.454 | 0.00168 | **0.875 / 0.597 / 1.659** | 0.00014 | K3 1.020 → 1.007 |
| 0.75 | 0.862 / 0.629 / 1.374 | 0.00288 | **0.860 / 0.545 / 1.719** | 0.00004 | U5 rim 1.046 → 0.999, every ring within 0.5 % |
| 1 | 0.790 / 0.551 / 1.209 | 0.06199 | 0.790 / 0.537 / 1.230 | 0.05965 | L2 rim 0.887 → 0.870, L7 / K4 / U6 rims −1.5 % |

Rows 0.6 and 1 hold several IORs; the refit only trades rings within that
spread (−5 % / −4 % of the sum, the worst ring moving either way) and they
stand. Rows 0.7 / 0.75 (one sphere each) were off by what changed under
them since #191 (#205's coat loss and re-read, #207's |n·k2_refl|; the
uncoated paths barely move, so most of it is #191's harness) and are
refitted. The grazing values now fall with r (0.735 / 0.597 / 0.545; r
1's 0.551 is above 0.545 only at exactly 90°, at θ 81° r 1 is lower,
0.580 v 0.597). U5's Iray model (`print_resin_now`, with τ) 0.537 … 0.316
v Iray 0.538 … 0.312.

## 2. Export change, before / after (400 samples, denoised)

Changes one material per scene: Calib-Furnace-Resin2 / 3 U5 (falloff
skew 1.87 → 2.17, k 0.772 → 0.773), the architectural sphere at r_t 0.7
of Calib-Furnace-ArchGlass (G5), -ArchGlass2 (H3), Calib-ArchGlass (G5;
Enhanced grazing 0.460 → 0.388). Before = #211's shipped renders
(`c211/ba/after-*`, c885a70), after = this change. Octane / Iray, disk <
0.9 R, rim 0.8 – 0.9:

| sphere | Faithful disk | Faithful rim | Enhanced disk | Enhanced rim |
|---|---|---|---|---|
| Furnace-Resin2 U5 | 1.043 → 1.032 | 1.119 → 1.072 | 1.061 → 1.050 | 1.150 → 1.099 |
| Furnace-Resin3 U5 | 1.003 → 0.998 | 1.064 → 1.025 | 1.021 → 1.013 | 1.089 → 1.050 |
| Furnace-ArchGlass G5 | 1.050 → 1.049 | 1.055 → 1.033 | 1.035 → 1.033 | 0.999 → 0.979 |
| Furnace-ArchGlass2 H3 | 1.049 → 1.047 | 1.054 → 1.033 | 1.035 → 1.033 | 1.005 → 0.983 |
| Calib-ArchGlass G5 (lit) | 1.052 → 1.050 | 1.045 → 1.022 | 1.035 → 1.031 | 0.994 → 0.971 |

Untouched spheres move ≤ 0.3 % except ArchGlass2 H8 Faithful (rim 1.067
→ 1.084, its export unchanged: the denoiser's floor, as in #211 § 7).
p99 unchanged. Enhanced's arch rims land 2 – 3 % under Iray, Faithful's
2 – 3 % over: the change is Iray's model meeting Iray, and both stay
inside the noise of a rim ring. Shipped.

## 3. Octane under the falloff on a rough sphere

### The α of #211 § 8

Its spheres' roughness pin 0.840896 is `legacy_phong_pin(1)` = √α: the
Beckmann α is **0.707**, not 0.841 (#211 modelled the pin as α and listed
"α = pin²" as a variant). At α 0.707 the flat sphere is Octane / model
1.06 … 1.06 (OCTANE_SPHERE_PLAIN's α 0.7 row, 1.02 – 1.06), not 1.24; the
falloff residue stays: what U5 (q 0) keeps of U4 (flat), Octane's over
the model's, 1.00 … 1.10 1.19, U8 (q 0.5) 1.01 … 1.10 1.18, U7 (U6's
shape) 1.00 … 1.01 1.03.

### Readings and path kinds that don't explain it (`diag212_exit`)

- The falloff read at the microfacet's |v·h| instead of the macro
  cosine (both events, entry only, exit only, the larger of the two):
  Octane / model 0.67 – 1.81 across the rings, every one worse than the
  macro reading.
- Octane = a · (paths that never pass the surface) + b · (paths that
  pass), per ring, least squares over the four `fr` spheres (α 0.707 and
  0.841): the single reflections are 2 – 5 % of a ring, a comes out 1.6 –
  5.5, b 0.98 – 1.14, and the rim still misses one sphere by 8 – 9 %. With
  the passing paths split into direct (two events) and reflected inside:
  the same 9 %. No reweighting
  of path kinds holds the four falloffs at once: the falloff meets
  different exit angles in Octane, not different path weights.

### Probe `side`: one rough face at a time

A Polygon Side texture (`orbx-variants.py set=<mat>.roughness=s<a>,<b>`,
new: a Mix of two floats by `NT_TEX_SIDE`, a where it is black, b where
white) gives the outside and the inside of a closed sphere their own
roughness. Calib-Furnace-Resin2 re-pinned, specular 1, IOR 1.49, 1024
raw, 145 s. **White is the outside** (front face): U2 (black 0, white α)
is 0.999 to ring 0.6 and falls at the rim, as only a rough outside over a
smooth inside does in the model.

| sphere | roughness out / in | transmission | Octane rings centre → 0.8 – 0.9 |
|---|---|---|---|
| U1 | α / α (side Mix) | flat | 0.800 0.795 0.784 0.769 0.747 0.725 0.705 0.683 0.648 |
| U8 | α (plain pin) | flat | 0.808 0.802 0.792 0.775 0.756 0.734 0.713 0.694 0.668 |
| U2 | α / 0 | flat | 0.999 0.999 0.999 0.999 0.999 0.998 0.947 0.737 0.446 |
| U4 | α / 0 | q 0, s 1 | 0.841 0.781 0.708 0.630 0.550 0.471 0.380 0.263 0.165 |
| U6 | α / 0 | q 0.5, s 1 | 0.917 0.885 0.846 0.802 0.757 0.709 0.630 0.467 0.284 |
| U3 | 0 / α | flat | 0.852 0.847 0.836 0.817 0.792 0.764 0.734 0.705 0.688 |
| U5 | 0 / α | q 0, s 1 | 0.785 0.708 0.623 0.538 0.455 0.378 0.307 0.243 0.204 |
| U7 | 0 / α | q 0.5, s 1 | 0.817 0.774 0.724 0.669 0.611 0.553 0.496 0.444 0.409 |

U1 is U8 within 1 – 3 % (the Mix is the pin; 3 % at the rim between two
places in the row, the floor for any ratio of two spheres' rims). Against
the Smith model at α 0.707 (`print_side_readings`, falloff at the
incoming macro cosine):

| | flat, Octane / model | falloff keeps, Octane / model: q 0 | q 0.5 |
|---|---|---|---|
| smooth entry, rough inside (U3 / U5 / U7) | 1.09 1.08 1.08 1.06 1.05 1.03 1.02 1.00 0.99 | 1.00 … 1.00 0.98 0.98 | 1.00 … 1.00 |
| rough entry, smooth inside (U2 / U4 / U6) | 1.01 1.01 1.01 1.02 1.02 1.03 1.04 1.08 1.22 | 1.00 … 1.03 1.08 1.15 | 1.00 … 1.01 1.02 1.05 |

**The rough exit is read right**: with a smooth entry, everything after
it (the rough inner surface, its reflections, the exit) keeps under the
falloff exactly what the model says (q 0.5 within 0.3 % on every ring).
Octane's excess there is a flat-sphere level (1.09 at the centre → 1 at
the rim), no shape. **The residue is the rough entry at grazing**: rings
0.7 – 0.9 only, Octane lets 8 / 22 % more in than Beckmann + Smith, and
what it lets in leaves the far side (smooth: at the same angle it
entered at, by the sphere's symmetry) closer to head-on. Neither the
incident masking (G1(l) alone: flat rim 1.17, what the falloff keeps
unchanged at 1.15; no masking at all: flat rim 1.14, q 0 rim 1.23) nor
another entry α (0.4 –
0.707: a lower α fixes the rim and breaks the centre) is it.

Not fitted. The open question — what Octane does at a rough dielectric
met near grazing (more transmitted, bent nearer the normal) — goes to a
follow-up issue with this probe as its starting point.

## Where it stands

Export changed: `LEGACY_LOBE` rows 0.7 / 0.75. U5's rim 1.12 / 1.15 →
1.07 / 1.10 (Faithful / Enhanced, this note's disk < 0.9 reading); U6
(r 1, 1.13 / 1.18) is the rough-entry residue. `Path` carries the
diagnostics' fields (`lost`, `through_h`, `events`), unused by the export.

## Process lessons

- A Polygon Side Mix on the roughness splits a closed sphere's entry from
  everything after it in one Octane job; the null twin (the Mix at α / α
  beside the plain pin) proves the Mix first.
- Check a probe's α against the export's pin function before modelling
  it: #211 § 8 modelled the pin as α, and the flat-sphere "excess" of 1.24
  it reported was mostly that.
- A refit harness that reweights one trace per sphere (each event's
  cosine kept on the path) runs four rows in 77 s; keep it as an ignored
  test beside the table instead of a scratch script, so the next model
  change can rerun it.
