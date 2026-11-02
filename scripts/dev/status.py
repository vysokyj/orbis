#!/usr/bin/env python3
"""ORBIS: state of the long-running things, read-only.

  python3 scripts/dev/status.py [--sets]

Git / CI, the render lock, DAZ Studio and Octane X processes, the Iray
Server queue (waiting, active, done, ETA, a stalled server), the last Octane
job, the cache. --sets adds the per-set table of the full Iray run, if its
helper (~/Library/Caches/orbis/calib/sets-status.py) exists.
"""
import os, re, subprocess, sys, time

HOME = os.path.expanduser("~")
CACHE = f"{HOME}/Library/Caches/orbis"
QUEUE = f"{CACHE}/calib/queue"
DAZ_LOG = f"{HOME}/Library/Application Support/DAZ 3D/Studio6/log.txt"
OCTANE_DOCS = f"{HOME}/Library/Containers/com.otoy.rndrviewer/Data/Documents"
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def run(*cmd, cwd=None):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=20).stdout.strip()
    except Exception:
        return ""


def ago(path):
    try:
        s = time.time() - os.path.getmtime(path)
    except OSError:
        return "?"
    return f"{int(s)} s ago" if s < 120 else f"{int(s / 60)} min ago" if s < 7200 else f"{s / 3600:.1f} h ago"


def count(d, ext):
    try:
        return len([f for f in os.listdir(d) if f.endswith(ext)])
    except OSError:
        return 0


def section(title):
    print(f"\n## {title}")


section("Git / CI")
print(run("git", "status", "-sb", cwd=ROOT).splitlines()[0] if run("git", "status", "-sb", cwd=ROOT) else "no git")
dirty = len(run("git", "status", "--porcelain", cwd=ROOT).splitlines())
print(f"uncommitted files: {dirty}")
ci = run("gh", "run", "list", "-L", "1", "--json", "status,conclusion,displayTitle,createdAt",
         "-q", '.[0] | "\\(.status) \\(.conclusion) \\(.displayTitle) (\\(.createdAt))"', cwd=ROOT)
print("CI:", ci or "unknown (gh)")

section("Render lock")
owner = f"{CACHE}/render.lock/owner"
if os.path.exists(owner):
    pid, _, what = open(owner).read().strip().partition(" ")
    alive = subprocess.run(["kill", "-0", pid], capture_output=True).returncode == 0
    print(f"held by {pid} ({what}), {'alive' if alive else 'DEAD - taken over by the next script'}, since {ago(owner)}")
else:
    print("free")

section("Apps")
for label, pat in (("DAZ Studio", "DAZStudio"), ("Octane X", "Octane")):
    pids = run("pgrep", "-f", pat).split()
    print(f"{label}: {'running (' + ', '.join(pids) + ')' if pids else 'not running'}")
render = run("pgrep", "-fl", "instanceName orbis-render")
if render:
    print("headless/queue DAZ instance (orbis-render): running")

section("Iray Server queue")
log = f"{QUEUE}/queue.log"
if not os.path.exists(log):
    print("no queue directory / log")
else:
    lines = open(log, errors="replace").read().splitlines()
    started = any("started" in l for l in lines)
    waiting, active, done = count(QUEUE, ".duf"), count(f"{QUEUE}/active", ".duf"), count(f"{QUEUE}/done", ".duf")
    print(f"queue script in DAZ: {'started' if started else 'NOT started'}; log updated {ago(log)}")
    print(f"waiting {waiting}, active {active}, done {done}")
    stale = time.time() - os.path.getmtime(log) > 900
    if started and stale and (waiting or active) and not [l for l in run("pgrep", "-fl", "DAZStudio").splitlines() if "instanceName orbis-render" not in l]:
        print("!! STALLED: queue has work but DAZ Studio is not running and the log is quiet - the user starts the queue script (daz-render-queue.dsa) again.")
    elif started and stale and (waiting or active):
        print("!! quiet for over 15 min with work left - check the DAZ log / server connection.")
    times = []
    for l in lines[::-1]:
        if "started, watching" in l:
            break
        m = re.search(r"rendered (\S+?)\.duf in (\d+) s", l)
        if m and int(m.group(2)) >= 15:
            times.append(int(m.group(2)))
    if times and waiting + active:
        avg = sum(times) / len(times)
        print(f"avg {avg:.0f} s/scene over {len(times)} -> ETA {(waiting + active) * avg / 60:.0f} min")
    print("last log:", *lines[-3:], sep="\n  ")
    if os.path.exists(DAZ_LOG):
        tail = open(DAZ_LOG, errors="replace").read().splitlines()[-300:]
        if any("session handler rejected the session" in l for l in tail):
            print("!! DAZ log: 'session handler rejected the session' = the Iray Server reset its password. Tell the user; they fix it and restart the queue script (retries every 30 s).")
    black = [f for f in os.listdir(QUEUE) if f.endswith(".png") and os.path.getsize(f"{QUEUE}/{f}") < 2000]
    if black:
        print("!! suspiciously small renders:", ", ".join(black[:5]))

section("Octane X")
rep = f"{OCTANE_DOCS}/orbis_probe_render.txt"
if os.path.exists(rep):
    print(f"last render report ({ago(rep)}):")
    print("  " + "\n  ".join(open(rep, errors="replace").read().strip().splitlines()[-4:]))
else:
    print("no render report")
done = f"{OCTANE_DOCS}/orbis_probe.done"
print("last probe finished:", ago(done) if os.path.exists(done) else "never")

section("Cache")
print(run("du", "-sh", CACHE).split("\t")[0], f"in {CACHE}")
orbx = len(run("find", CACHE, "-name", "*.orbx").splitlines())
print(f".orbx exports: {orbx}" + ("  -> scripts/dev/clean-cache.sh" if orbx else ""))

if "--sets" in sys.argv:
    helper = f"{CACHE}/calib/sets-status.py"
    section("Per-set table (full Iray run)")
    print(run("python3", helper) if os.path.exists(helper) else "no sets-status.py in the cache")
