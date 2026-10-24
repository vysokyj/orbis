#!/usr/bin/env python3
"""Compare Octane and Iray renders of the primitives calibration scenes (#6).

    calib-compare.py regions <iray.png> <octane.png>
        Linear luminance (median) per region and the Octane / Iray ratio.
    calib-compare.py spot <iray.png> <octane.png>
        Calib-Spot only: intensity per degree off the spot's axis, I(θ) ∝
        luminance · d² / cos(incidence) over the floor, both renders.

Iray PNGs are decoded with gamma 2.2 (DAZ's tone mapping), Octane's with the
sRGB curve. 1920 × 1080 renders from the Calib-* scenes' camera. No numpy /
Pillow: PNGs are converted to BMP with macOS `sips`.
"""
import collections
import math
import os
import struct
import subprocess
import sys
import tempfile

REGIONS = {  # (y0, x0, y1, x1)
    "floor_front": (760, 700, 860, 1200),
    "floor_right": (600, 1250, 680, 1350),
    "cube1": (490, 715, 590, 815),
    "cube2": (490, 910, 590, 1010),
    "cube3": (490, 1095, 590, 1195),
    "sphere": (180, 990, 250, 1060),
    "skin_belly": (280, 470, 320, 505),
    "skin_thigh": (430, 450, 500, 475),
    "shadow": (640, 850, 660, 1000),
    "background": (300, 100, 400, 250),
}


def load(png):
    """Pixel accessor (x, y) → (r, g, b) 0–255, via a BMP from sips."""
    bmp = os.path.join(tempfile.mkdtemp(), "image.bmp")
    subprocess.run(["sips", "-s", "format", "bmp", png, "--out", bmp], check=True, capture_output=True)
    b = open(bmp, "rb").read()
    off = struct.unpack("<I", b[10:14])[0]
    w, h = struct.unpack("<ii", b[18:26])
    bpp = struct.unpack("<H", b[28:30])[0] // 8
    row = (w * bpp + 3) // 4 * 4

    def px(x, y):
        yy = y if h < 0 else abs(h) - 1 - y
        o = off + yy * row + x * bpp
        return (b[o + 2], b[o + 1], b[o])

    return px, w, abs(h)


def srgb(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def gamma22(c):
    return (c / 255) ** 2.2


def luminance(rgb, decode):
    r, g, b = (decode(v) for v in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def region(px, box, decode):
    y0, x0, y1, x1 = box
    values = sorted(luminance(px(x, y), decode) for y in range(y0, y1, 2) for x in range(x0, x1, 2))
    return values[len(values) // 2]


def regions(iray, octane):
    a, _, _ = load(iray)
    b, _, _ = load(octane)
    print(f"{'region':12s} {'iray':>8s} {'octane':>8s}  ratio")
    for name, box in REGIONS.items():
        la, lb = region(a, box, gamma22), region(b, box, srgb)
        ratio = lb / la if la > 1e-5 else float("nan")
        print(f"{name:12s} {la:8.4f} {lb:8.4f}  {ratio:5.2f}")


# Calib-Spot geometry (meters): the exported camera and the DAZ spot.
CAMERA = ((0, 1.2, 4.5), (0.000111, 0.459159, 0.482302), (0.000005, 0.983421, -0.181337), 68.725571)
SPOT = ((0.01004163, 3.017201, 2.496938), -46.0658, 0.3437746)  # position, rotation X, Y (degrees)


def spot_profile(png, decode):
    sub = lambda a, b: [a[i] - b[i] for i in range(3)]
    dot = lambda a, b: sum(a[i] * b[i] for i in range(3))
    cross = lambda a, b: [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    norm = lambda a: [x / math.sqrt(dot(a, a)) for x in a]
    pos, tgt, up, fov = CAMERA
    fw = norm(sub(tgt, pos))
    rt = norm(cross(fw, up))
    u2 = cross(rt, fw)
    s, ax, ay = SPOT[0], math.radians(SPOT[1]), math.radians(SPOT[2])
    d = (0, math.sin(ax), -math.cos(ax))  # Rx · (0, 0, -1)
    d = (d[0] * math.cos(ay) + d[2] * math.sin(ay), d[1], -d[0] * math.sin(ay) + d[2] * math.cos(ay))
    px, w, h = load(png)
    f = (w / 2) / math.tan(math.radians(fov) / 2)
    bins = collections.defaultdict(list)
    for y in range(560, h, 3):
        for x in range(0, w, 3):
            if 380 <= x <= 640 or (660 <= x <= 1260 and y <= 700):
                continue  # the figure, the cubes and their shadows
            r = norm([fw[i] + rt[i] * (x + 0.5 - w / 2) / f + u2[i] * (h / 2 - y - 0.5) / f for i in range(3)])
            if r[1] >= -1e-6:
                continue
            p = [pos[i] - pos[1] / r[1] * r[i] for i in range(3)]
            if abs(p[0]) > 1.95 or abs(p[2]) > 1.95:
                continue
            v = sub(p, s)
            dist = math.sqrt(dot(v, v))
            v = [c / dist for c in v]
            theta = math.degrees(math.acos(max(-1, min(1, dot(v, d)))))
            bins[int(theta)].append(luminance(px(x, y), decode) * dist * dist / -v[1])
    return {k: sorted(v)[len(v) // 2] for k, v in bins.items() if len(v) >= 8}


def spot(iray, octane):
    a, b = spot_profile(iray, gamma22), spot_profile(octane, srgb)
    print("deg     iray   octane  ratio")
    for k in sorted(set(a) | set(b)):
        ia, ib = a.get(k, float("nan")), b.get(k, float("nan"))
        print(f"{k:3d}  {ia:7.4f}  {ib:7.4f}  {ib / ia if ia else float('nan'):5.2f}")


if __name__ == "__main__":
    if len(sys.argv) != 4 or sys.argv[1] not in ("regions", "spot"):
        sys.exit(__doc__)
    {"regions": regions, "spot": spot}[sys.argv[1]](sys.argv[2], sys.argv[3])
