# 2026-11-21 — The resin rim patch, checked in Octane (#205)

Resume of #205 (decomposition: note 2026-11-19-resin-rim-decomposition).
The prepared export patch — `RESIN_LOBE` re-read on the projected discs,
`legacy_loss` on the coat events of a legacy (`mdl 1.0`) `Libbsdf` stack
(resin, architectural glass), the solver's falloff shape drawn with Octane's
curve (`falloff_curve`) — applied on top of #206 (679479c, `ARCH_GLASS_LOBE`
re-read) and rendered: Faithful and Enhanced final, 400 samples, of
Calib-Furnace-Resin / -Resin2 / -Resin3 / -ArchGlass / -ArchGlass2 /
-ArchGlass3, Calib-Resin, Calib-ArchGlass. `calib-variants.sh`, both
variants ~15 min together.

Before: the 2026-11-19 renders (`raw` only, before #206 for the ArchGlass
scenes; no Faithful -ArchGlass3). To compare like with like every number
below is the raw pass (`ORBIS_PASS=raw`); the denoised product differs by
≤ 0.5 % in disk / rim means.

## Furnace spheres, Octane / Iray (Iray Server 5000), before → after

Faithful (rim = ring 0.8 – 0.9 of the silhouette, `furnace-rings.py`):

| scene | sphere | disk < 0.75 | ring 0.7 – 0.8 | rim |
|---|---|---|---|---|
| Resin | R1 grey r 0.25 | 1.011 → 0.998 | 1.000 → 0.980 | 1.039 → 1.006 |
| Resin | R3 white r 0.25 | 1.007 → 0.996 | 0.995 → 0.977 | 1.035 → 1.002 |
| Resin | R5 grey r 0.25 c0 | 1.023 → 1.016 | 1.028 → 1.020 | 1.024 → 1.015 |
| Resin | R7 grey r 0.5 | 1.031 → 0.992 | 1.060 → 0.995 | **1.243 → 1.162** |
| Resin | R8 grey r 0.1 | 1.007 → 0.998 | 1.015 → 1.002 | 1.032 → 1.016 |
| Resin2 | U3 white r 0.25 | 1.011 → 1.008 | 1.007 → 1.004 | 1.006 → 1.001 |
| Resin2 | U4 white r 0.5 | 0.994 → 1.012 | 1.009 → 1.027 | 1.023 → 1.044 |
| Resin2 | U5 white r 0.75 | 1.020 → 1.004 | 1.063 → 1.041 | **1.115 → 1.096** |
| Resin2 | U6 white r 1 | 1.066 → 1.047 | 1.123 → 1.103 | **1.126 → 1.107** |
| Resin2 | U7 grey r 0.5 | 1.000 → 1.016 | 1.020 → 1.037 | 1.049 → 1.062 |
| Resin2 | U8 white r 0.25 | 1.003 → 0.996 | 0.990 → 0.974 | 1.027 → 1.002 |

Enhanced moves the same way (R7 rim 1.225 → 1.145, U5 1.147 → 1.122, U6
1.177 → 1.153, U4 disk 0.997 → 1.013, U7 1.002 → 1.019). Resin3 (Resin2 at
Max Path Length 3, a diagnostic) follows Resin2, its U5 disk 0.993 → 0.972
and U8 rim 0.995 → 0.967 away. The ArchGlass furnaces move ≤ 1 % per sphere
(the coat loss is 1 at architectural's mirror coat; the rest is #206's
~0.4 %), most towards Iray; ArchGlass2's H8 rim unchanged (1.12 Faithful, 1.06
Enhanced), Calib-ArchGlass's G8 1.18 (#207).

Disc scenes, `calib-compare-all.py` (sphere middles), Faithful:

| scene | before | after |
|---|---|---|
| Calib-Resin | 1.06 1.02 1.05 1.02 1.05 1.03 1.13 1.01 | 1.05 1.02 1.04 1.02 1.04 1.02 1.10 1.00 |
| Calib-ArchGlass | 1.03 1.02 1.02 1.02 1.06 1.04 1.04 1.18 | 1.03 1.01 1.02 1.01 1.05 1.04 1.03 1.18 |

Enhanced: Calib-Resin 1.07 … 1.14 → 1.06 … 1.11, Calib-ArchGlass the same
or 0.01 down. No disc moves away.

## Decision: shipped

The resume's bar was "no sphere moves away from Iray by more than ~1 %".
Three things cross it, each explained, none a fit:

- **U4 / U7 (r 0.5) up 1.2 – 1.6 %**: the re-read table's r 0.5 entry is
  1.3 % higher (0.678 → 0.687), measured on U4 itself. The stale reading
  had hidden an Octane render / model excess of ~1 % there; it is a
  component of the Octane side, now visible (#211).
- **R1 / R3 ring 0.7 – 0.8 down 2 %** while their rims come 3 % closer:
  the coat's legacy loss grows towards grazing; the two rings together
  are closer than before.
- **Resin3** (path length 3) is a bounce-limit diagnostic, not a target.

The big residues all come down (R7's rim 8 %, its disk 4 %, U5 / U6 2 %),
every disc scene improves, so the patch ships; what is left goes to #211.

## Where it stands

R7's rim 1.16 (Octane's coating reflection has no falloff), U6 1.11 (the
per-ring excess under q ≈ 0), U5 1.09, U4 / U7's 1 % excess at r 0.5: #211.

## Process lessons

- Keep a scene check's "before" renders with their denoised twins (copy
  the `denoised/` directory too): without it the before / after compare
  has to force the raw pass on both sides.
- A re-measured table can make a sphere it was measured on worse: the old
  value was cancelling a residue elsewhere. Read the move as a component
  coming into view, not as the measurement being wrong.
