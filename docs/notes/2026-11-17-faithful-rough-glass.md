# 2026-11-17 — Faithful takes back Octane's rough transmission excess (#185)

The user's call on #185: **(b) Faithful only**. Faithful counters what
Octane renders over its own Smith model in closed rough glass (resin,
architectural, the Uber's rough glass), so those drop to Iray's level at
α ≥ 0.5; Enhanced keeps Octane's brighter rough glass. Follows the
decomposition of the same day (2026-11-17-octane-rough-transmission).

## The counter: a measured table, not a fit to a render

Decompose before fitting: the excess was measured on plain closed
Beckmann spheres with a known input (no falloff, white, no Iray), and the
counter is that measurement, folded into the existing solve — nothing is
fitted to a calibration scene.

- `OCTANE_SPHERE` (`src/mdl_layer.rs`): Octane's rings (0 – 0.1 … 0.8 –
  0.9) of Calib-Furnace-UberGlass's export re-pinned, α 0.25 / 0.36 / 0.45
  / 0.5 / 0.6 / 0.7 / 0.85 / 1, specular 0 (`tr`) and 1 (`rt`), at IOR
  1.5 (the first session's `tr` / `rt`) and **IOR 2** (new, this session:
  `tr2` / `rt2` = the same `set=` pins `&ior=2`, enhanced-final
  `--no-denoise`, 1024 samples, 533 / 793 s). #198's H7 (IOR 2) disk
  1.07 v H2 1.03 had said the excess grows with the IOR.
- `octane_excess_rows`: each ring / the Rust Smith model's ring (`trace`,
  2^18 paths so the ratio's ring noise is ~1 %; 2^16 left ±3 % wiggles),
  over the α 0.25 row's mean disk ratio per IOR and mode (1.01: the
  dome's normalisation, not the lobe).

Octane / model, rings centre → 0.8 – 0.9 (after the α 0.25 baseline):

| α | `tr` 1.5 | `rt` 1.5 | `tr` 2 | `rt` 2 |
|---|---|---|---|---|
| 0.36 | 1.01 → 1.00 | 1.00 → 1.04 | 1.01 → 1.02 | 1.02 → 1.05 |
| 0.5 | 1.01 → 1.02 | 1.04 → 1.12 | 1.03 → 1.07 | 1.08 → 1.16 |
| 0.6 | 1.01 → 1.03 | 1.06 → 1.22 | 1.10 → 1.13 | 1.16 → 1.29 |
| 0.7 | 1.01 → 1.05 | 1.08 → 1.32 | 1.09 → 1.21 | 1.29 → 1.44 |
| 0.85 | 1.04 → 1.09 | 1.16 → 1.51 | 1.21 → 1.37 | 1.50 → 1.68 |
| 1 | 1.09 → 1.16 | 1.28 → 1.71 | 1.29 → 1.49 | 1.74 → 1.94 |

At IOR 2 even the transmission alone keeps 1.1 – 1.5 × the model at α ≥
0.6 (inside, more facets reflect totally), and with the reflection on far
more. So the IOR has to be in the table: an IOR 1.5 table alone would
leave H7's excess half in.

`octane_excess(stack)`: linear in α from 1 at α 0.25, linear in the IOR
between 1.5 and 2 (held outside — #198's H6 at IOR 1.2 kept 1.03 like
H2's 1.5), mixed between `tr` and `rt` by the reflection's share (1 for
resin's reflect + transmit base, the coat's weight over a transmit-only
base: Universal's `specular` for the Uber, architectural's coating — the
coating's own α isn't measured, the base's stands for both). None for a
thin wall. `octane_trace` multiplies each Smith path's weight by it at the
path's entry ring; `falloff_rings` (resin, architectural) and
`tint_factor_octane` (Uber rough glass) solve against that when
`Tuning::rough_transmission_excess` is on — the falloff's shape, then k.

Effect on the solve, uncoated resin r 0.75 white: k 0.830 → 0.801,
grazing 0.786 → 0.567 (skew 0.92 → 1.55): the excess sits at the rim, so
the falloff takes most of it there.

## Enhanced unchanged

Enhanced exports of Calib-Furnace-Resin2, -ArchGlass2, -UberGlass,
Calib-Resin and Calib-ArchGlass from master (1ba388f, a scratch
worktree build) and from this change: `.orbx` byte-identical.

## Renders: not done

Faithful before (the flag off) and after exports of Calib-Furnace-Resin2,
-Resin, -ArchGlass, -ArchGlass2, -UberGlass, Calib-Resin and
Calib-ArchGlass are in `~/Library/Caches/orbis/calib/orbx/f185-before/`
and `f185-after/`. The before batch failed at once: the screen was locked
(Octane X stays in its simple mode, no Script menu), so no Faithful
render was compared against Iray in this session. Per the loop's rule
the render wasn't retried; #185 waits for the check (Iray references are
all in `calib/references/iray`).

## Process lessons

- A ring ratio of two Monte Carlo estimates needs the model at ≥ 4 × the
  paths of the solve, and a scalar baseline rather than a per-ring one —
  dividing by a noisy α 0.25 row ring for ring doubled the wiggle.
- `status.py` shows the render lock and Octane but not the screen lock;
  `ioreg -n Root -d1 -a | grep CGSSessionScreenIsLocked` does.
