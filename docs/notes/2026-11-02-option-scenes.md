# 2026-11-02 — a slow Iray render: Calib-RS-Bloom

Iray Server, 5000 samples, the batch of option scenes (#68). Most scenes took
105–112 s; the odd ones:

| Scene | Seconds | Why it differs |
|---|---|---|
| `Calib-Env-Ground` | 175 | ground plane drawn |
| `Calib-Env-GroundFog` | 302 | ground fog |
| `Calib-L-Opt-Temp3000` | 306 | not understood |
| **`Calib-RS-Bloom`** | **1433** | Bloom Filter Enable on, threshold 1.0 (default 2000) |

`Calib-RS-Bloom` took about 13 times the usual time for the same scene size
and samples. Calib-Glass (its base) is not slow otherwise. Guess: with the
threshold at 1.0 nearly every pixel blooms, so the filter runs over the whole
frame. Not tested (a higher threshold would show it).

Open: expect `Calib-RS-Caustics` and `Calib-RS-Spectral` to be slow as well;
the cause of `Calib-L-Opt-Temp3000` (306 s vs 110 s for the other light
options) is unknown.
