#!/usr/bin/env python3
"""ORBIS: talk to the Iray Server's web API (the REST calls its own web UI
makes, js/main.js; undocumented, #195). The login comes from the repo's
git-ignored .env (IRAY_SERVER_HOST, _PORT, _USER, _PASSWORD; see
.env.example) and is never printed.
  python3 scripts/dev/iray-server.py submit [-s samples] [-o out dir] <scene.duf>...
  python3 scripts/dev/iray-server.py info              # server info, queue state
  python3 scripts/dev/iray-server.py queue             # jobs in the queue
  python3 scripts/dev/iray-server.py idle              # block until no job waits or renders
  python3 scripts/dev/iray-server.py results [search]  # finished results
  python3 scripts/dev/iray-server.py get <path>        # raw GET, JSON printed
idle: a low-priority batch calls it before each scene, so jobs an issue
submits go first (one scene per submit; the issue's job waits at most for
the batch scene already rendering).
submit: renders scenes on the Iray Server with no DAZ window (#195). Each
scene is capped like iray-queue.sh does (Max Samples, default 400; Max Time
0; no quality stop), a headless DAZ (daz-script.sh, daz-server-submit.dsa)
uploads it into the server's job queue through the scene's current camera,
the server's queue is started, and each finished job's PNG lands in
<out dir>/<scene>.png (default ~/Library/Caches/orbis/calib/iray) with the
capped copy beside it (<scene>.duf) and <scene>.png.run.txt ("rendered in N
s", keep-refs.py reads both); the job and its result are then removed from
the server. Waits for every job; exit 1 if any failed.
"""
import datetime, gzip, http.cookiejar, io, json, os, shutil, subprocess, sys, time, urllib.parse, urllib.request, zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def env():
    vals = {}
    path = os.path.join(ROOT, '.env')
    if not os.path.exists(path):
        sys.exit('iray-server: no .env (copy .env.example and fill it in)')
    for line in open(path):
        line = line.strip()
        if line and not line.startswith('#') and '=' in line:
            k, v = line.split('=', 1)
            vals[k.strip()] = v.strip().strip('"').strip("'")
    return vals


class Server:
    def __init__(self):
        e = env()
        self.base = f"http://{e.get('IRAY_SERVER_HOST', 'localhost')}:{e.get('IRAY_SERVER_PORT', '9090')}/"
        self.opener = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        r = self.call('POST', 'session', {'user_name': e.get('IRAY_SERVER_USER', ''), 'password': e.get('IRAY_SERVER_PASSWORD', '')})
        if not isinstance(r, dict) or r.get('error'):
            sys.exit(f"iray-server: login refused: {r.get('error') if isinstance(r, dict) else r}")

    def call(self, method, path, body=None, raw=False):
        data = json.dumps(body).encode() if body is not None else None
        req = urllib.request.Request(self.base + path, data=data, method=method,
                                     headers={'Content-Type': 'application/json', 'Accept': 'application/json'})
        with self.opener.open(req, timeout=60) as resp:
            b = resp.read()
        if raw:
            return b
        try:
            return json.loads(b)
        except ValueError:
            return b.decode(errors='replace')[:500]


CACHE = os.path.expanduser('~/Library/Caches/orbis')
HERE = os.path.dirname(os.path.abspath(__file__))


def cap(src, dst, samples):
    """The capped copy iray-queue.sh makes: Max Samples, no time / quality stop."""
    raw = open(src, 'rb').read()
    d = json.loads(gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw)

    def walk(v):
        if isinstance(v, dict):
            if v.get('id') == 'Max Time': v['current_value'] = 0
            if v.get('id') == 'Max Samples': v['current_value'] = samples
            if v.get('id') == 'Rendering Quality Enable': v['current_value'] = False
            for x in v.values(): walk(x)
        elif isinstance(v, list):
            for x in v: walk(x)
    for e in d['scene'].get('extra', []):
        if e.get('type') == 'studio_render_settings':
            walk(e)
    open(dst, 'wb').write(gzip.compress(json.dumps(d).encode()))
    return (d['scene'].get('current_camera') or '#-')[1:]


def submit(argv):
    samples, out, scenes = 400, os.path.join(CACHE, 'calib', 'iray'), []
    while argv:
        a = argv.pop(0)
        if a == '-s': samples = int(argv.pop(0))
        elif a == '-o': out = os.path.abspath(argv.pop(0))
        else: scenes.append(os.path.abspath(a))
    if not scenes:
        sys.exit('iray-server: submit needs scenes')
    if subprocess.run([sys.executable, os.path.join(HERE, 'duf_camera.py')] + scenes).returncode:
        sys.exit('iray-server: fix the scenes first (duf_camera.py --fix, #97)')
    s = Server()
    stamp = datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
    work = os.path.join(CACHE, 'calib', 'server', stamp)
    os.makedirs(work, exist_ok=True)
    os.makedirs(out, exist_ok=True)
    jobs, args = {}, []
    for sc in scenes:
        name = os.path.splitext(os.path.basename(sc))[0]
        copy = os.path.join(work, name + '.duf')
        cam = cap(sc, copy, samples)
        job = f'{name}-{stamp}'
        jobs[job] = (name, copy)
        args += [copy, cam, job]
    done = os.path.join(work, 'done')
    env = dict(os.environ, DONE_FILE=done)
    subprocess.run([os.path.join(HERE, 'daz-script.sh'), os.path.join(HERE, 'daz-server-submit.dsa'),
                    work, os.path.join(ROOT, '.env'), str(len(scenes))] + args, env=env, stdout=subprocess.DEVNULL)
    report = open(os.path.join(work, 'submit.txt')).read().splitlines() if os.path.exists(os.path.join(work, 'submit.txt')) else ['failed: no report from DAZ']
    failed = [l for l in report if not l.startswith('ok ')]
    for l in failed:
        print('iray-server:', l)
    sent = [l[3:] for l in report if l.startswith('ok ')]
    if not sent:
        sys.exit(1)
    print(f'iray-server: {len(sent)} job(s) in the server queue at {samples} samples')
    s.call('POST', 'queue', {'command': 'start', 'nodes': 1})
    started, left, t0 = {}, set(sent), time.time()
    while left:
        time.sleep(5)
        for j in s.call('GET', 'queue').get('jobs', []):
            if j['name'] not in left:
                continue
            if j['status'] == 'active':
                started.setdefault(j['name'], time.time())
            elif j['status'] in ('done', 'failed', 'stopped'):
                name, copy = jobs[j['name']]
                secs = round(time.time() - started.get(j['name'], t0))
                if j['status'] == 'done':
                    z = zipfile.ZipFile(io.BytesIO(s.call('GET', f"results/{urllib.parse.quote(j['name'])}.zip?id={j['id']}", raw=True)))
                    pngs = [n for n in z.namelist() if n.endswith('-Beauty.png')] or [n for n in z.namelist() if n.endswith('.png')]
                    png = os.path.join(out, name + '.png')
                    # The copy first: keep-refs.py takes the latest copy saved before the png.
                    shutil.copy(copy, os.path.join(out, name + '.duf'))
                    open(png, 'wb').write(z.read(pngs[0]))
                    open(png + '.run.txt', 'w').write(f'Iray Server (headless submit, #195): rendered in {secs} s\n')
                    print(f'iray-server: rendered {name} in ~{secs} s -> {png}')
                else:
                    failed.append(j['name'])
                    print(f"iray-server: {name} {j['status']}: {j.get('statusMessage', '')}")
                s.call('DELETE', 'results', {'id': j['id']})
                s.call('DELETE', 'queue', {'ids': [j['id']]})
                left.discard(j['name'])
    sys.exit(1 if failed else 0)


def main(argv):
    if not argv or argv[0] in ('-h', '--help'):
        print(__doc__)
        return
    if argv[0] == 'submit':
        return submit(argv[1:])
    s = Server()
    cmd = argv[0]
    if cmd == 'info':
        print(json.dumps({'info': s.call('GET', 'info'), 'queue': s.call('GET', 'queue')}, indent=1)[:4000])
    elif cmd == 'idle':
        while any(j['status'] in ('waiting', 'active') for j in s.call('GET', 'queue').get('jobs', [])):
            time.sleep(15)
    elif cmd == 'queue':
        print(json.dumps(s.call('GET', 'queue'), indent=1)[:8000])
    elif cmd == 'results':
        q = urllib.parse.urlencode({'search': argv[1] if len(argv) > 1 else '', 'limit': 20, 'offset': 0})
        print(json.dumps(s.call('GET', 'results?' + q), indent=1)[:8000])
    elif cmd == 'get':
        print(json.dumps(s.call('GET', argv[1]), indent=1)[:8000])
    else:
        sys.exit(f'iray-server: unknown command {cmd}')


if __name__ == '__main__':
    main(sys.argv[1:])
