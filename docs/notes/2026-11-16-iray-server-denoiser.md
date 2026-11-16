# 2026-11-16 Does the Iray Server apply DAZ's Post Denoiser? (#171)

Goal: #140 found Calib-PBRSkin-Detail (Post Denoiser on@8) and -NoDN (off)
equal at 5000 samples, so it could not tell whether the Iray Server ignores
the denoiser or whether it moves nothing at 5000. Decide it from what is
already on disk; render only if that fails. No render was needed.

## The docs first

NVIDIA's Iray manual (3.10.2, Deep Learning-based Denoiser): the denoiser
runs only on GPU hardware (inferencing on CPUs is impractical); short of
device memory the post-processing falls back to the CPU and denoising is
disabled. `post_denoiser_available` / `_enabled` / `_start_iteration` are
the options DAZ's Post Denoiser Available / Enable / Start Iteration set.
The Iray Server (Boost for Daz) renders on rented GPUs; the local
`daz-render.sh` Iray on this Mac has the CPU only. So the server can, the
CPU cannot - whatever the scene's switch says.

The logs say nothing either way: DAZ's log on the client side shows the
bridge upload, "Received update to N iterations" and the streamed frames,
no post-processing line; the queue logs and `daz-render.sh`'s run logs
only the times.

## The pixels: one grain number, three render classes

Metric: luminance (8-bit, as saved) minus its 5 x 5 box blur, std over the
inner 70 % of a sphere's disc, plus the high-pass's correlation with the
neighbouring pixel (white Monte Carlo grain ~ 0, a denoised or textured
surface clearly positive). The Calib-Skin-*-front scenes (private G9
figures; renders only, numbers here) have a plain grey sphere and a skin
sphere beside the figure; every variant leaves the spheres alone. Three
classes of render of the same frame:

- **CPU 400**: `iray-400`, local CPU Iray via `daz-render.sh` (run log
  beside, 148 - 370 s), Max Samples 400, Post Denoiser Enable true.
- **Server 5000**: `iray-full`, the references.
- **Server 400**: `iray`, the 2026-11-07 batch (gloss0 / dlflat / coat0 /
  nb / nn variants), Post Denoiser Enable true.

| sphere (Victoria front) | CPU 400 | server 5000 | server 400 |
|---|---|---|---|
| grey, high-pass std (/255) | 0.22 - 0.38 | 0.14 - 0.15 | 0.14 |
| grey, neighbour corr. | -0.06 .. -0.01 | +0.02 .. +0.12 | +0.01 .. +0.03 |
| skin, high-pass std (/255) | 1.77 - 2.22 | 0.79 - 1.24 | 0.39 - 0.74 |
| skin, neighbour corr. | +0.06 .. +0.18 | +0.47 .. +0.58 | +0.64 .. +0.66 |

(Victoria's w0 variant left out: no translucency, its sphere's surface
structure dominates every class, 2.5 / 0.74 in both CPU and server.) Kayo,
Gabriela and Nishi, server 400, skin sphere: 0.47 - 1.46, neighbour +0.53
.. +0.72; their `iray-400` files turned out to be copies of the 5000
references (byte-identical stats, written 2026-10-28 13:4x after the CPU
runs), so they give no CPU class.

Reading:
- The CPU 400 grain is white (neighbour ~0) and 2 - 3 x the server 5000
  structure on the skin sphere, about the sqrt(5000 / 400) = 3.5 x that an
  undenoised 400 would show over a converged 5000 once the 5000's own
  surface structure is taken out. The grey sphere is converged at 400
  already (2 x the 8-bit floor on CPU).
- The server's 400-sample renders, same sampler and samples, have **no
  white grain**: the grey sphere sits on the 8-bit floor (0.14) and the
  skin sphere's high-pass is *below* the 5000-sample reference's (0.39 -
  0.74 against 0.79 - 1.24), with the strongest neighbour correlation of
  the three. An undenoised 400 can't be smoother than a 5000; a denoised
  one is (it flattens a little of the fine bump too).
- At 5000 the denoiser moves nothing: Calib-PBRSkin-Detail against -NoDN
  (both server, #140), the high-pass std 1.968 / 1.969, neighbour 0.758 /
  0.757, their difference 0.08 / 255 std and white (the PNGs' rounding).

**Answer: the Iray Server applies DAZ's Post Denoiser.** At 5000 samples it
is the identity within 8-bit rounding, at 400 it removes the grain. The
local CPU Iray never applies it (GPU only), whatever the scene says.

## What changed

- `renders.tsv`'s `renderer` column was wrong for 73 rows: `keep-refs.py`
  wrote "DAZ Studio Iray (CPU)" unless `--renderer` was passed, and from
  2026-11-03 on the queue's references were kept without it. Every one
  of them has its capped copy in the queue's `done/` folder minutes before
  the render and a server queue log; none has a `daz-render.sh` run log.
  They now read "Iray Server (Boost for Daz)". The one real CPU reference
  is Calib-Tone-Gladiator (2026-10-26, 400 samples, Max Time 1800, before
  the server was set up); its `denoiser` is now `off`.
- `keep-refs.py` tells the renderer from the copy it reads (queue `done/`
  = server, beside the png / `queue-local` = CPU; `--renderer` overrides)
  and records a CPU render's denoiser as `off`.
- So `renders.tsv`'s `denoiser` column describes the image, and
  `octane_pass.py`'s Iray label (read from it) stands. #139's rule
  (compare Octane's denoised pass with Iray) holds for every server
  reference; the 400 / 1000 / 2000-sample server references are denoised
  images, not raw ones.
- The 2026-11-13 note's "46 local CPU references, all on@8" is superseded
  by this: one CPU reference, raw.

## Process lessons

- A grain number needs a part of the frame no variant touches (here the
  spheres beside the figure) and a neighbour correlation beside the std:
  surface structure and denoised residue are correlated, Monte Carlo grain
  is not.
- Check the provenance of a cached render before reading it as a class:
  `iray-400` held 5000-sample copies under 400-sample names.
- A label nobody passes is a wrong default: infer it from what the run left.
