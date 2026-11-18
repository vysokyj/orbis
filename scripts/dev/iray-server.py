#!/usr/bin/env python3
"""ORBIS: talk to the Iray Server's web API (the REST calls its own web UI
makes, js/main.js; undocumented, #195). The login comes from the repo's
git-ignored .env (IRAY_SERVER_HOST, _PORT, _USER, _PASSWORD; see
.env.example) and is never printed.
  python3 scripts/dev/iray-server.py submit [-s samples] [-o out dir] [--upload-stall s] [--render-stall s] <scene.duf>...
  python3 scripts/dev/iray-server.py info              # server info, queue state
  python3 scripts/dev/iray-server.py queue             # jobs in the queue
  python3 scripts/dev/iray-server.py idle              # block until no job waits or renders
  python3 scripts/dev/iray-server.py cancel [prefix]   # remove queued / rendering jobs
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
Stall guards (#200), so a batch goes on: an upload whose DAZ log shows no
progress for --upload-stall s (default 600; the server once sat 3 h on
"Waiting for server result...") kills the headless DAZ, the scene is
failed and a new DAZ uploads the rest; a job that never shows up in the
server queue (300 s) or renders with no progress (iterations, frame
progress, snapshot loading) for --render-stall s (default 1800) is failed,
stopped and removed.
"""
import datetime, gzip, re, http.cookiejar, io, json, os, shutil, subprocess, sys, time, urllib.parse, urllib.request, zipfile

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


DAZ_LOG = os.path.expanduser('~/Library/Application Support/DAZ 3D/Studio6 [orbisrender]/log.txt')
# DAZ's log line without its timestamp and the snapshot's elapsed seconds:
# "snapshot_cloud(1): 6.0 s Waiting for server result... Uploaded 1.6 mb"
# repeats every 0.5 s while the server sits on an upload (#200).
_STAMP = re.compile(r'^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d+ ')
_ELAPSED = re.compile(r'(snapshot_cloud\(\d+\): )\d+(\.\d+)? s ')


def log_signature(text):
    """The last log line with the timestamp and elapsed time taken out: it
    stays the same only while nothing moves (no bytes uploaded, no pending
    request answered, no other log line)."""
    lines = [l for l in text.splitlines() if l.strip()]
    return _ELAPSED.sub(r'\1', _STAMP.sub('', lines[-1])) if lines else None


def job_signature(j):
    """What moves while a server job renders: iterations, frame progress,
    the hosts' snapshot loading (the fields the server's web UI shows)."""
    return json.dumps([j.get('status'), j.get('iterations'), j.get('progress'), j.get('quality'),
                       [(t.get('snapshotProgress'), t.get('iterations'), t.get('progress')) for t in j.get('tasks') or []]],
                      sort_keys=True, default=str)


def read_report(work):
    path = os.path.join(work, 'submit.txt')
    return [l for l in open(path).read().splitlines() if l.strip()] if os.path.exists(path) else []


def upload(work, triples, stall):
    """One headless DAZ uploading the (copy, camera, job) triples. A job whose
    upload makes no progress for `stall` seconds (DAZ's log, #200) is
    reported failed and DAZ is killed (daz-script.sh's DONE_FILE: kill, put
    the menus back, unlock); the triples after it go to a new DAZ. Returns
    the report lines ("ok <job>" / "failed <job>: ...")."""
    report = []
    while triples:
        for f in ('submit.txt', 'current.txt', 'done'):
            if os.path.exists(os.path.join(work, f)):
                os.remove(os.path.join(work, f))
        done = os.path.join(work, 'done')
        args = [x for t in triples for x in t]
        off = os.path.getsize(DAZ_LOG) if os.path.exists(DAZ_LOG) else 0
        # Popen, not run: this loop watches the upload while DAZ blocks in it.
        p = subprocess.Popen([os.path.join(HERE, 'daz-script.sh'), os.path.join(HERE, 'daz-server-submit.dsa'),
                              work, os.path.join(ROOT, '.env'), str(len(triples))] + args,
                             env=dict(os.environ, DONE_FILE=done), stdout=subprocess.DEVNULL)
        sig, since, job, stalled = None, time.time(), None, None
        while p.poll() is None:
            time.sleep(2)
            cur = os.path.join(work, 'current.txt')
            now_job = open(cur).read().strip() if os.path.exists(cur) else None
            text = ''
            if os.path.exists(DAZ_LOG):
                if os.path.getsize(DAZ_LOG) < off:
                    off = 0  # a new log (DAZ rotates it at start)
                with open(DAZ_LOG, 'rb') as f:
                    f.seek(off)
                    data = f.read()
                # Keep the tail from the last full line on.
                cut = data.rfind(b'\n') + 1
                off += cut
                text = data[:cut].decode(errors='replace')
            new = (now_job, log_signature(text) if text else sig and sig[1])
            if new != sig or now_job != job:
                sig, since, job = new, time.time(), now_job
            elif job and not any(l.split(':')[0] in (f'ok {job}', f'failed {job}') for l in read_report(work)) \
                    and time.time() - since > stall:
                stalled = job
                print(f'iray-server: {job}: upload stalled, no progress for {stall} s (last: {(sig[1] or "")[:160]}); killing DAZ', flush=True)
                open(done, 'w').write('stalled')
                p.wait()
                break
        lines = read_report(work)
        if not stalled and not os.path.exists(done):
            lines.append('failed: DAZ quit without a report')
        report += lines
        finished = {l.split(':')[0].split(' ', 1)[1] for l in lines if ' ' in l.split(':')[0]}
        if stalled:
            report.append(f'failed {stalled}: upload stalled for {stall} s, the server never answered (#200)')
            finished.add(stalled)
            # A new DAZ for what this one never got to.
            triples = [t for t in triples if t[2] not in finished]
        else:
            triples = []
    return report


def submit(argv):
    samples, out, scenes = 400, os.path.join(CACHE, 'calib', 'iray'), []
    upload_stall, render_stall, appear = 600, 1800, 300
    while argv:
        a = argv.pop(0)
        if a == '-s': samples = int(argv.pop(0))
        elif a == '-o': out = os.path.abspath(argv.pop(0))
        elif a == '--upload-stall': upload_stall = int(argv.pop(0))
        elif a == '--render-stall': render_stall = int(argv.pop(0))
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
    jobs, triples = {}, []
    for sc in scenes:
        name = os.path.splitext(os.path.basename(sc))[0]
        copy = os.path.join(work, name + '.duf')
        cam = cap(sc, copy, samples)
        job = f'{name}-{stamp}'
        jobs[job] = (name, copy)
        triples.append((copy, cam, job))
    report = upload(work, triples, upload_stall)
    failed = [l for l in report if not l.startswith('ok ')]
    for l in failed:
        print('iray-server:', l)
    # A stalled upload may still turn into a job later: never let it render.
    for j in s.call('GET', 'queue').get('jobs', []):
        if any(l.startswith(f"failed {j['name']}:") for l in failed):
            s.call('DELETE', 'queue', {'ids': [j['id']]})
            print(f"iray-server: removed {j['name']} from the server queue")
    sent = [l[3:] for l in report if l.startswith('ok ')]
    if not sent:
        sys.exit(1)
    print(f'iray-server: {len(sent)} job(s) in the server queue at {samples} samples', flush=True)
    s.call('POST', 'queue', {'command': 'start', 'nodes': 1})
    # Per job: when it started, its last progress and when that moved, when
    # it was last seen in the queue. A job that never shows up (`appear` s)
    # or renders without progress (`render_stall` s) is failed and removed,
    # so the batch goes on (#200).
    started, moved, seen, killed, left, t0 = {}, {}, {}, {}, set(sent), time.time()
    while left:
        time.sleep(5)
        now = time.time()
        try:
            queue = s.call('GET', 'queue').get('jobs', [])
        except OSError as e:
            # The server not answering is not a job's stall: pause the clocks.
            print(f'iray-server: queue not answering ({e}); waiting', flush=True)
            seen = {k: v + 5 for k, v in seen.items()}
            moved = {k: (v[0], v[1] + 5) for k, v in moved.items()}
            t0 += 5
            continue
        for j in queue:
            if j['name'] not in left:
                continue
            seen[j['name']] = now
            if j['name'] in killed:
                if j['status'] in ('done', 'failed', 'stopped'):
                    s.call('DELETE', 'results', {'id': j['id']})
                    s.call('DELETE', 'queue', {'ids': [j['id']]})
                    left.discard(j['name'])
            elif j['status'] == 'active':
                started.setdefault(j['name'], now)
                sig = job_signature(j)
                if moved.get(j['name'], (None,))[0] != sig:
                    moved[j['name']] = (sig, now)
                elif now - moved[j['name']][1] > render_stall:
                    failed.append(j['name'])
                    print(f"iray-server: {jobs[j['name']][0]} stalled: no render progress for {render_stall} s; stopping and removing it", flush=True)
                    # The web UI's Stop (POST job, now); once it reads
                    # "stopped", the branch below removes it and its result
                    # (deleted at once, the result came back afterwards).
                    s.call('POST', 'job', {'id': j['id'], 'command': 'stop', 'now': True})
                    killed[j['name']] = (j['id'], now)
            elif j['status'] in ('done', 'failed', 'stopped'):
                name, copy = jobs[j['name']]
                secs = round(now - started.get(j['name'], t0))
                if j['status'] == 'done':
                    z = zipfile.ZipFile(io.BytesIO(s.call('GET', f"results/{urllib.parse.quote(j['name'])}.zip?id={j['id']}", raw=True)))
                    pngs = [n for n in z.namelist() if n.endswith('-Beauty.png')] or [n for n in z.namelist() if n.endswith('.png')]
                    png = os.path.join(out, name + '.png')
                    # The copy first: keep-refs.py takes the latest copy saved before the png.
                    shutil.copy(copy, os.path.join(out, name + '.duf'))
                    open(png, 'wb').write(z.read(pngs[0]))
                    open(png + '.run.txt', 'w').write(f'Iray Server (headless submit, #195): rendered in {secs} s\n')
                    print(f'iray-server: rendered {name} in ~{secs} s -> {png}', flush=True)
                else:
                    failed.append(j['name'])
                    print(f"iray-server: {name} {j['status']}: {j.get('statusMessage', '')}", flush=True)
                s.call('DELETE', 'results', {'id': j['id']})
                s.call('DELETE', 'queue', {'ids': [j['id']]})
                left.discard(j['name'])
        for name, (jid, since) in killed.items():
            if name in left and now - since > 120:
                # Never reached "stopped": remove it as it is.
                s.call('DELETE', 'queue', {'ids': [jid]})
                s.call('DELETE', 'results', {'id': jid})
                left.discard(name)
        for name in list(left):
            if name in killed:
                continue
            if now - seen.get(name, t0) > appear:
                failed.append(name)
                print(f'iray-server: {jobs[name][0]}: not in the server queue for {appear} s; given up', flush=True)
                left.discard(name)
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
    elif cmd == 'cancel':
        # Remove waiting / rendering jobs (all, or those whose name starts with
        # the argument): after killing a submit, its job would render on.
        for j in s.call('GET', 'queue').get('jobs', []):
            if len(argv) < 2 or j['name'].startswith(argv[1]):
                s.call('DELETE', 'queue', {'ids': [j['id']]})
                print(f"iray-server: cancelled {j['name']} ({j['status']})")
    elif cmd == 'results':
        q = urllib.parse.urlencode({'search': argv[1] if len(argv) > 1 else '', 'limit': 20, 'offset': 0})
        print(json.dumps(s.call('GET', 'results?' + q), indent=1)[:8000])
    elif cmd == 'get':
        print(json.dumps(s.call('GET', argv[1]), indent=1)[:8000])
    else:
        sys.exit(f'iray-server: unknown command {cmd}')


if __name__ == '__main__':
    main(sys.argv[1:])
