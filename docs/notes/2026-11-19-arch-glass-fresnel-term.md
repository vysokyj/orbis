# 2026-11-19 Architectural glass with the Fresnel term: G8 decomposed

Issue #207 (found in #206). No render: the Iray references in git
(`furnace-rings.py measure`, projected discs), `libbsdf` read for
`curve_layer_sample`, and `mdl_layer.rs`'s tracer with switches
(`diag207`, ignored tests: `cargo test --release --lib diag207 --
--ignored --nocapture`).

Calib-Furnace-ArchGlass, white, closed IOR 1.5, transmit lobe at r_t 0.60
(refr_gloss 0.2), a mirror coat (refl_gloss 1) at reflectivity 0.6:

| Sphere | Iray disk | rings centre → 0.8 – 0.9 |
|---|---|---|
| G4 lobe alone | 0.609 | 0.717 0.703 0.689 0.667 0.638 0.601 0.556 0.500 0.431 |
| G7 Schlick (r0 0.2) | 0.648 | 0.749 0.735 0.723 0.704 0.676 0.642 0.597 0.545 0.484 |
| G8 Fresnel term | 0.412 | 0.459 0.459 0.448 0.438 0.423 0.404 0.387 0.376 0.374 |

## 1. `libbsdf` re-read (MDL SDK master, read, not copied)

`curve_layer_sample`: the coat is picked with w · c(n·k1) (n·k1 clamped),
weighted c(k1·h) w / p; the base 1 − w · max(c(n·k1), c(nk2_refl)) / (1 −
p), nk2_refl = 2 |k1·h| (n·h) − n·k1 on a refraction (`compute_half_vector`
from k1, k2 at the layer's IOR pair, oriented to k1's side: the sampled
microfacet), |n·k2| otherwise (reflection, thin wall, or a curve without
IOR: `custom_curve_layer` sets −1). `process_ior_fresnel_layer` takes the
side from which slot holds the material IOR: from inside the pair is
(1.5, 1), `ior_fresnel` returns 1 past the critical angle. All of it is
what `trace_n` does: the model is `libbsdf`'s layering.

## 2. Where the layer weight is taken (base weight variants)

Model / Iray rings, G8 (legacy lobe as the export traces it, τ 1.001):

| Base weight | G8 disk | rings |
|---|---|---|
| max(c(nk1), c(nk2_refl)) (`libbsdf`) | 0.396 | 0.94 0.96 0.96 0.96 0.96 0.95 0.98 0.98 0.97 |
| c(nk1) only (energy-conserving) | 0.617 | 1.55 … 1.24 |
| max with c(\|n·k2\|) of the refracted ray | 0.592 | 1.55 … 0.90 |
| nk2_refl at the other side's IOR | 0.381 | 0.96 … 0.78 |

The critical-angle weight is the bulk of G8's loss (0.617 → 0.396 v Iray
0.412); the `libbsdf` rule is the only variant near Iray. G4 / G7 don't
move with it (G7: no IOR; disk 0.651 v 0.648).

## 3. Where `legacy_loss` is drawn (τ re-solved so G4 stays 0.609)

| Legacy loss | τ | G4 rim | G7 disk | G8 disk | G8 rings |
|---|---|---|---|---|---|
| every base event, incoming angle (model) | 1.001 | 1.03 | 0.651 | 0.396 | 0.94 … 0.97 |
| refractions only | 0.989 | 1.09 | 0.648 | 0.393 | 0.92 … 1.01 |
| head-on value everywhere | 0.966 | 1.15 | 0.650 | 0.396 | 0.89 … 1.07 |
| exits at angle, entries head-on | 0.972 | 1.16 | 0.648 | 0.393 | 0.90 … 1.07 |
| **the same g, as the death of the widest microfacets** (u of the h sample > g) | 0.966 | 1.02 | 0.649 | **0.417** | 1.01 1.02 1.03 1.02 1.02 1.00 1.02 0.98 0.99 |

As a factor the loss doesn't touch G8 wherever it is drawn: the 4 % is
not in `legacy_loss`'s angle. As a *cut* of the widest microfacets - the
ones whose reflection lies past the critical angle and lose to the layer
weight anyway - the two losses overlap instead of multiplying, and G8
comes to 0.417 v 0.412 with every ring within 3 % (G4 and G7 hold, their
rims a little better).

## 4. The same hypothesis on resin (dead end as it stands)

Calib-Furnace-Resin / -Resin2 (lobe alone from #205's re-read table),
disk model / Iray, factor → tail (coat without / with #205's coat loss):

| Sphere | factor | tail | tail + coat loss |
|---|---|---|---|
| R1 grey r 0.25 coat 0.95 | 0.987 | 1.003 | 0.996 |
| R3 white r 0.25 | 1.000 | 1.016 | 1.010 |
| R7 grey r 0.5 | 1.000 | **1.065** | 1.037 |
| R8 grey r 0.1 | 0.990 | 0.996 | 0.993 |
| U5 / U6 uncoated r 0.75 / 1 | 1.007 / 1.000 (rim 1.07 / 1.00) | 1.001 / 1.002 (rim 1.02 / 0.96) | same |

The pure cut breaks R7 (+6.5 %) and needs τ 0.89 at r 1 (the table rows
were fitted as a factor; under a cut they would have to be refitted).
So neither extreme holds every sphere; a mix would be a fitted constant.
Not shipped: the model stays as it is, G8 3 % low (the export's k for it
is solved against 0.396, so Octane's G8 is ~3 % low).

## 5. What decides it: Calib-Furnace-MdlFresnel (parked for Iray)

`make-mdl-scenes.py`, our own modules `calib/mdl/orbis_fresnel_v10/13.mdl`
(G8's stack written out: fresnel_layer IOR 1.5 of a mirror over the closed
transmit lobe at a constant r). Predictions (disk r < 0.75):

| Sphere | factor (model) | cut |
|---|---|---|
| F1 1.0 w 0.6 r 0.6 (G8's twin) | 0.396 | 0.416 |
| F2 1.3 w 0.6 r 0.6 | 0.484 | 0.484 |
| F3 1.0 w 1 r 0.6 | 0.268 | 0.300 |
| F4 1.3 w 1 r 0.6 | 0.301 | 0.301 |
| F5 1.0 w 0.6 r 0.3 | 0.729 | 0.739 |
| F6 1.0 w 0.6 r 1 | 0.205 | 0.224 |
| F7 1.0 lobe alone r 0.6 | 0.607 | 0.607 |
| F8 1.0 Schlick F0 0.04 w 0.6 r 0.6 | 0.613 | 0.612 |

F2 / F4 test `libbsdf`'s fresnel_layer in Iray (1.3 has no legacy lobe);
F1 against G8 tests the twin; F3 and F6 separate factor and cut by 12 % /
9 %; F8 is the control without a critical angle.

## Coordination

#205's `c205-export.patch` doesn't move G8: its coat `legacy_loss` is 1
at the mirror coat (r 0), `RESIN_LOBE` and the Falloff curve are not in
the Iray model. It still applies (`git apply --check`) after this note's
code (a test module appended to `mdl_layer.rs`).

## Lesson

A per-event loss measured on uncoated spheres says how much is lost, not
which samples: wherever another weight depends on the same sample (here
the microfacet's reflection past the critical angle), the two can
multiply or overlap, and only a sphere that has both (G8, F3) tells.
