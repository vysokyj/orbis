#!/usr/bin/env python3
"""Every channel of DAZ's main shaders — its label, group, default — and
whether the export reads it, with how many scenes in My Library set it.

  python3 scripts/dev/shader-coverage.py > coverage.md

Shaders: Iray Uber (from a scene's material library — DAZ writes the whole
definition there), PBRSkin (its .dsf in the content library), the DAZ
Default shader (from a scene that uses it). "Read" = the channel id appears
as a string literal in src/ (as in channel-audit.py); "scenes" = how many
scenes set it away from its default (channel-audit.py's count).
"""
import collections, glob, gzip, json, os, re, subprocess, sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../..')
scenes = '/Volumes/Data/DAZ 3D/Studio/My Library/Scenes'
content = '/Volumes/Data/DAZ 3D/Studio/My DAZ 3D Library'
handled = set()
for f in glob.glob(os.path.join(root, 'src/**/*.rs'), recursive=True):
    handled |= set(re.findall(r'"([A-Za-z][A-Za-z0-9 /_\-()]+)"', open(f).read()))


def load(p):
    raw = open(p, 'rb').read()
    return json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)


def channel_list(entry):
    out = []
    for e in entry.get('extra', []) or []:
        for c in e.get('channels', []) or []:
            ch = c.get('channel')
            if ch:
                out.append((ch, c.get('group', '')))
    for k, v in entry.items():
        if isinstance(v, dict) and isinstance(v.get('channel'), dict):
            out.append((v['channel'], v.get('group', '')))
    return out


# How often scenes set each channel: reuse the audit's report.
audit = subprocess.run([sys.executable, os.path.join(root, 'scripts/dev/channel-audit.py')], capture_output=True, text=True).stdout
set_in = collections.defaultdict(int)
for line in audit.splitlines():
    m = re.match(r'\| (.+?) \| (\d+) \|', line)
    if m:
        set_in[m.group(1)] = max(set_in[m.group(1)], int(m.group(2)))

shaders = {}
for p in sorted(glob.glob(os.path.join(scenes, '*.duf'))):
    try:
        d = load(p)
    except Exception:
        continue
    for m in d.get('material_library', []):
        kinds = [e.get('type', '') for e in m.get('extra', [])]
        if 'studio/material/uber_iray' in kinds and 'Iray Uber' not in shaders and len(channel_list(m)) > 100:
            shaders['Iray Uber'] = channel_list(m)
        if 'studio/material/daz_shader' in kinds and 'DAZ Default' not in shaders:
            shaders['DAZ Default'] = channel_list(m)
    if len(shaders) == 2:
        break
pbr = os.path.join(content, 'data/DAZ 3D/Built-in Content/Daz Iray PBRSkin/PBRSkin.dsf')
if os.path.exists(pbr):
    shaders['PBRSkin'] = channel_list(load(pbr)['material_library'][0])

print('# Shader channels vs. the ORBIS export\n')
for name, chans in shaders.items():
    read = sum(1 for ch, _ in chans if ch['id'] in handled)
    print(f'## {name} — {read} of {len(chans)} read\n')
    print('| channel | label | group | default | read | scenes setting it |\n|---|---|---|---|---|---|')
    for ch, group in sorted(chans, key=lambda cg: (cg[0]['id'] in handled, -set_in.get(cg[0]['id'], 0), cg[1], cg[0]['id'])):
        default = json.dumps(ch.get('value'))
        if len(default) > 30:
            default = default[:27] + '…'
        print(f"| {ch['id']} | {ch.get('label') or ch.get('name', '')} | {group} | {default} | "
              f"{'yes' if ch['id'] in handled else '**no**'} | {set_in.get(ch['id'], '')} |")
    print()
