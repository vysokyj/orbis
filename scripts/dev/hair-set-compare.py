#!/usr/bin/env python3
"""Patch table for #161's hair set (Calib-Hair-Pixie / -Kevin / -Hudson / -Athena).

    hair-set-compare.py [--octane DIR] [--iray DIR] [--boxes OUT.png] [hair...]

Per hair and patch: linear means (PNG decoded with gamma 2.2, as
calib-compare.py) of Iray and of Octane in both Looks, base and -nogloss;
prints Octane / Iray per Look (R / G / B and luminance) and each renderer's
gloss share (base / -nogloss). The patches are boxes inside the hair on
the 1280 x 720 renders (the scenes' camera, make-hair-set-scenes.py).
`--boxes` writes a contact sheet of the Iray renders with the boxes drawn.
Iray references: calib/references/iray; Octane: ~/Library/Caches/orbis/
calib/octane/{faithful,enhanced}-final/denoised.
"""
import argparse
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# (x0, y0, x1, y1) on 1280 x 720
PATCHES = {
    "Pixie": {"crown": (560, 150, 720, 200), "fringe": (420, 300, 500, 390), "side": (765, 250, 820, 360)},
    "Kevin": {"crown": (640, 150, 740, 200), "front": (560, 270, 690, 330), "curls": (430, 310, 530, 380), "shaved": (725, 365, 795, 420)},
    "Hudson": {"crown": (560, 125, 740, 185), "front": (485, 190, 570, 260), "side": (725, 245, 795, 340)},
    "Athena": {"fringe": (520, 150, 560, 200), "temple": (680, 150, 730, 210), "lengths": (740, 300, 800, 380), "lower": (780, 430, 840, 490)},
}

# Athena's hair vanishes in Octane with its cutout map (#202): its Octane
# side is the Enhanced render of the -op1 probe (Cutout Opacity 1), against
# Iray's base - on fibres Iray's own cutout share is ~1 where the pile is
# dense (2026-11-15 note), so the ratio is a rough one.
OCTANE_STANDIN = {"Athena": ("enhanced", "-op1")}


def load(path):
    a = np.asarray(Image.open(path).convert("RGB"), dtype=np.float64) / 255.0
    return a ** 2.2


def mean(img, box):
    x0, y0, x1, y1 = box
    return img[y0:y1, x0:x1].reshape(-1, 3).mean(axis=0)


def lum(c):
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def fmt(r):
    return "%.2f / %.2f / %.2f (%.2f)" % tuple(r)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--octane", default=os.path.expanduser("~/Library/Caches/orbis/calib/octane"))
    ap.add_argument("--iray", default=os.path.join(ROOT, "calib/references/iray"))
    ap.add_argument("--boxes")
    ap.add_argument("hairs", nargs="*", default=list(PATCHES))
    a = ap.parse_args()
    sheet = []
    for h in a.hairs:
        imgs = {}
        for v in ("", "-nogloss"):
            name = "Calib-Hair-%s%s.png" % (h, v)
            imgs[("iray", v)] = load(os.path.join(a.iray, name))
            for look in ("faithful", "enhanced"):
                imgs[(look, v)] = load(os.path.join(a.octane, look + "-final/denoised", name))
        if h in OCTANE_STANDIN:
            look, v = OCTANE_STANDIN[h]
            st = load(os.path.join(a.octane, look + "-final/denoised", "Calib-Hair-%s%s.png" % (h, v)))
            print("## %s  (R / G / B (lum)); Octane = %s %s, see OCTANE_STANDIN" % (h, look, v))
            print("| patch | Iray lin | Octane%s / Iray | Iray gloss share |" % v)
            print("|---|---|---|---|")
            for p, box in PATCHES[h].items():
                mi, mo, mn = mean(imgs[("iray", "")], box), mean(st, box), mean(imgs[("iray", "-nogloss")], box)
                print("| %s | %.3f | %s | %s |" % (p, lum(mi), fmt(np.append(mo / mi, lum(mo) / lum(mi))), fmt(np.append(mi / mn, lum(mi) / lum(mn)))))
            print()
        else:
            print("## %s  (R / G / B (lum))" % h)
            print("| patch | Iray lin | Faithful / Iray | Enhanced / Iray | F / Iray -nogloss | Iray gloss share | Octane F gloss share |")
            print("|---|---|---|---|---|---|---|")
            for p, box in PATCHES[h].items():
                m = {k: mean(v, box) for k, v in imgs.items()}
                r = lambda x, y: np.append(m[x] / m[y], lum(m[x]) / lum(m[y]))
                print("| %s | %.3f | %s | %s | %s | %s | %s |" % (
                    p, lum(m[("iray", "")]),
                    fmt(r(("faithful", ""), ("iray", ""))),
                    fmt(r(("enhanced", ""), ("iray", ""))),
                    fmt(r(("faithful", "-nogloss"), ("iray", "-nogloss"))),
                    fmt(r(("iray", ""), ("iray", "-nogloss"))),
                    fmt(r(("faithful", ""), ("faithful", "-nogloss")))))
            print()
        if a.boxes:
            im = Image.open(os.path.join(a.iray, "Calib-Hair-%s.png" % h)).convert("RGB")
            d = ImageDraw.Draw(im)
            for p, box in PATCHES[h].items():
                d.rectangle(box, outline=(255, 0, 0), width=2)
                d.text((box[0] + 3, box[1] + 3), p, fill=(255, 0, 0))
            sheet.append(im.resize((640, 360)))
    if a.boxes and sheet:
        out = Image.new("RGB", (1280, 360 * ((len(sheet) + 1) // 2)))
        for i, im in enumerate(sheet):
            out.paste(im, ((i % 2) * 640, (i // 2) * 360))
        out.save(a.boxes)


if __name__ == "__main__":
    sys.exit(main())
