#!/usr/bin/env python3
"""Which pass of an Octane render a calibration compare reads (#139).

  python3 scripts/dev/octane_pass.py <render.png>...
      Prints, per render, the file a compare script reads and why.

Imported by the compare scripts (calib-compare-all.py, highlight-spread.py,
skin-compare.py, gloss-split.py, furnace-rings.py, point-highlight.py,
refr-volume-mc.py --measure):

  path, label = octane_pass.pick(png)

octane-render.sh saves the raw beauty as <out>/<scene>.png and, for an
export with the denoiser on, Octane's denoised pass as
<out>/denoised/<scene>.png. Iray's references are rendered with DAZ's Post
Denoiser on (renders.tsv's `denoiser` column, per reference), so the
export's own product is what is compared:

- the render directory is a variant (enhanced-final, faithful-preview...):
  src/tuning.rs's `denoiser` for that variant decides. Off -> the raw image
  IS the product (label `raw`); on -> the denoised twin (`denoised`), or the
  raw image marked `raw*` when the twin is missing or stale (rendered before
  4efd231, or by hand);
- any other directory: the twin when it is there (octane-render.sh writes it
  only for an export with the denoiser on), else `raw*`.

`raw*` = grain-sensitive numbers (texture, highlight peaks, p99, per-pixel
error) are not comparable with a denoised Iray reference; means over a
region barely move. A twin is stale when its time differs from the raw
image's by more than STALE seconds (both are saved by the same render).

ORBIS_PASS=raw | denoised forces a pass (to measure what the denoiser moves);
the label then says `raw (forced)` / `denoised (forced)`.
"""
import csv
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
TUNING = os.path.join(REPO, 'src', 'tuning.rs')
RENDERS_TSV = os.path.join(REPO, 'calib', 'references', 'iray', 'renders.tsv')
STALE = 120
VARIANT = re.compile(r'(enhanced|faithful)-(final|preview)')
_denoiser = {}
_iray = None


def variant_denoiser(directory):
    """src/tuning.rs's `denoiser` for the variant a render directory is
    named after, or None when the directory is not a variant."""
    m = VARIANT.fullmatch(os.path.basename(os.path.normpath(directory)))
    if not m:
        return None
    if m.group(0) not in _denoiser:
        look, mode = (s.capitalize() for s in m.groups())
        src = open(TUNING).read()
        hit = re.search(rf'\({look}, {mode}\)\s*=>\s*Tuning\s*\{{[^}}]*?denoiser:\s*(true|false)', src)
        if not hit:
            raise SystemExit(f'octane_pass: no denoiser for ({look}, {mode}) in {TUNING}')
        _denoiser[m.group(0)] = hit.group(1) == 'true'
    return _denoiser[m.group(0)]


def twin(png):
    d, f = os.path.split(png)
    return os.path.join(d, 'denoised', f)


def pick(png):
    """(path to read, label): `denoised`, `raw` (the variant renders without
    the denoiser) or `raw*` (a denoised pass was expected or can't be told,
    and isn't there)."""
    if os.path.basename(os.path.dirname(os.path.abspath(png))) == 'iray':
        # An Iray render (the cache's calib/iray, calib/references/iray):
        # read as it is, its Post Denoiser named.
        return png, f'Iray, denoiser {iray_denoiser(os.path.basename(png)[:-4].split("__")[0])}'
    den = twin(png)
    have = os.path.exists(den)
    fresh = have and abs(os.path.getmtime(den) - os.path.getmtime(png)) <= STALE
    force = os.environ.get('ORBIS_PASS', '')
    if force == 'raw':
        return png, 'raw (forced)'
    if force == 'denoised':
        return (den, 'denoised (forced)') if have else (png, 'raw* (no denoised pass)')
    on = variant_denoiser(os.path.dirname(png) or '.')
    if on is False:
        return png, 'raw'
    if fresh:
        return den, 'denoised'
    return png, 'raw* (stale denoised twin)' if have else 'raw* (no denoised pass)'


def short(label):
    return label.split(' ')[0]


def summary(labels):
    """One line for a report header: how many renders read which pass."""
    count = {}
    for lb in labels:
        count[short(lb)] = count.get(short(lb), 0) + 1
    parts = [f'{n} {k}' for k, n in sorted(count.items())]
    line = 'Octane pass read: ' + (', '.join(parts) or 'none')
    if any(short(lb).startswith('raw*') for lb in labels):
        line += ('. raw* = the export has the denoiser on (or it can\'t be told) but no denoised pass was saved: '
                 'grain-sensitive numbers (texture, peaks, p99) are not comparable with Iray\'s denoised references')
    return line


def iray_denoiser(scene):
    """renders.tsv's `denoiser` for an Iray reference (`on@<start iteration>`,
    `off`), '?' when it isn't recorded."""
    global _iray
    if _iray is None:
        _iray = {}
        if os.path.exists(RENDERS_TSV):
            for r in csv.DictReader(open(RENDERS_TSV), delimiter='\t'):
                _iray[r['scene']] = r.get('denoiser') or '?'
    return _iray.get(scene, '?')


if __name__ == '__main__':
    if len(sys.argv) < 2 or sys.argv[1] in ('-h', '--help'):
        print(__doc__)
        sys.exit(0)
    for p in sys.argv[1:]:
        path, label = pick(p)
        print(f'{p}\t{label}\t{path}')
