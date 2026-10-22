#!/usr/bin/env python3
"""Compare an Octane render of an ORBIS export with DAZ's Iray render.

    octane-light-fit.py compare <daz.png> <octane.png>
        Mean colour and linear-luminance ratio over a few regions + RMSE.
        The regions are pixel boxes for the Octane1 test scene at 1920x1080.

    octane-light-fit.py decompose <daz.png> <probe-prefix> [exposure]
        <probe-prefix>only_{env,spot,point,emissive}.png are the Lua probe's
        renders of one light group each (the others' power at 0) at
        `exposure` (default 0.5), and <probe-prefix>all_e05.png is the full
        render. Prints each group's share and the factor its power needs,
        relative to the 1.4 exposure base, where the group dominates; writes
        dominance.png (blue = env, yellow = spot, red = point, green =
        emissive).

Needs Pillow and numpy. The PNGs are sRGB (Octane's imager response 401).
"""
import sys

import numpy as np
from PIL import Image

EXPOSURE_BASE = 1.4
GROUPS = ["env", "spot", "point", "emissive"]
REGIONS = {  # (y0, x0, y1, x1)
    "wall_top_left": (0, 200, 250, 550),
    "wall_mid": (300, 600, 1000, 1200),
    "wall_right": (600, 1700, 750, 1900),
    "floor_left": (780, 260, 880, 560),
    "floor_center": (1000, 850, 1080, 1100),
    "floor_right": (1000, 1400, 1080, 1700),
    "floor_shadow_stool": (880, 1200, 920, 1350),
    "sofa": (650, 20, 780, 180),
    "rug": (950, 100, 1050, 450),
    "skin_belly": (380, 730, 430, 790),
    "skin_thigh": (600, 690, 700, 740),
    "vanity_drawer": (600, 1490, 700, 1590),
}


def load(path):
    return np.asarray(Image.open(path).convert("RGB")).astype(float) / 255


def lin(x):
    return np.where(x <= 0.04045, x / 12.92, ((x + 0.055) / 1.055) ** 2.4)


def luma(x):
    return x @ [0.2126, 0.7152, 0.0722]


def compare(daz_path, oct_path):
    a, b = load(daz_path), load(oct_path)
    la, lb = lin(a), lin(b)
    print(f"{'region':20s} {'DAZ sRGB':>16s} {'Octane sRGB':>16s}  linY Octane/DAZ")
    for name, (y0, x0, y1, x1) in REGIONS.items():
        ma = a[y0:y1, x0:x1].reshape(-1, 3).mean(0) * 255
        mb = b[y0:y1, x0:x1].reshape(-1, 3).mean(0) * 255
        r = luma(lb[y0:y1, x0:x1]).mean() / luma(la[y0:y1, x0:x1]).mean()
        print(f"{name:20s} {str(ma.round().astype(int)):>16s} {str(mb.round().astype(int)):>16s}  {r:.2f}")
    ya, yb = luma(la), luma(lb)
    print(f"global linY ratio {yb.mean() / ya.mean():.3f}, median {np.median(yb) / np.median(ya):.3f}")
    print(f"RMSE (sRGB, 0-255) {np.sqrt(((a - b) ** 2).mean()) * 255:.1f}")


def decompose(daz_path, prefix, exposure):
    daz = load(daz_path)
    comps = {g: load(f"{prefix}only_{g}.png") for g in GROUPS}
    full = load(f"{prefix}all_e05.png")
    lum = {g: luma(lin(c)) for g, c in comps.items()}
    total = sum(lum.values()) + 1e-9
    unclipped = full.max(2) < 0.97
    print(f"additivity full/sum: {luma(lin(full))[unclipped].sum() / total[unclipped].sum():.3f}")
    for g in GROUPS:
        print(f"share of {g}: {lum[g][unclipped].sum() / total[unclipped].sum():.3f}")
    # Where one group dominates, Iray / that group alone is its exposure.
    target = luma(lin(daz))
    usable = (daz.max(2) < 0.95) & unclipped & (daz.min(2) > 0.03)
    for g in GROUPS:
        for share in (0.9, 0.8, 0.7):
            m = usable & (lum[g] / total > share)
            if m.sum() > 2000:
                break
        if not m.any():
            print(f"{g:9s} never dominant")
            continue
        e = exposure * np.median(target[m] / lum[g][m])
        print(f"{g:9s} share > {share}: {m.sum():7d} px, exposure-equivalent {e:.2f} -> power x{e / EXPOSURE_BASE:.2f}")
    colours = {"env": [0, 0, 1], "spot": [1, 1, 0], "point": [1, 0, 0], "emissive": [0, 1, 0]}
    rgb = sum(np.array(colours[g]) * (lum[g] / total)[..., None] for g in GROUPS)
    Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8)).save("dominance.png")


if __name__ == "__main__":
    args = sys.argv[1:]
    if len(args) == 3 and args[0] == "compare":
        compare(args[1], args[2])
    elif len(args) in (3, 4) and args[0] == "decompose":
        decompose(args[1], args[2], float(args[3]) if len(args) == 4 else 0.5)
    else:
        sys.exit(__doc__)
