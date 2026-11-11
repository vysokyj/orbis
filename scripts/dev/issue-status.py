#!/usr/bin/env python3
"""ORBIS: where the issue loop stands, one table, read-only.

  python3 scripts/dev/issue-status.py [--hours N] [--all]

Reads the issues through `gh` and prints a Markdown table: what closed in
the last N hours (default 24), what can resume (its Iray references came
in), what waits for an Iray batch (`needs-iray`, with the scenes its last
`## Iray batch` comment lists), what waits for the user (`question`), and
the next issue the loop would pick. --all lists the untouched open issues
too. The states and the comment's shape: .claude/skills/issue-loop/SKILL.md.
"""
import datetime, json, re, subprocess, sys

HELD = {3: "signing needs the user", 19: "no constant tuning", 50: "big private scene", 57: "deferred"}  # issue-loop, "Pick"
ORDER = ["done", "resume", "needs-iray", "question", "open", "held"]


def batch(comments):
    """(scenes by sample count, references came in?) from the last `## Iray batch` comment."""
    last = None
    for i, c in enumerate(comments):
        if "## Iray batch" in c["body"]:
            last = i
    if last is None:
        return None, False
    scenes = {}
    for line in comments[last]["body"].splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) >= 2 and cells[0].endswith(".duf"):
            samples = re.sub(r"\D", "", cells[1]) or "?"
            scenes[samples] = scenes.get(samples, 0) + 1
    answered = any("Iray references in" in c["body"] for c in comments[last + 1:])
    return scenes, answered


def main():
    args = sys.argv[1:]
    hours = int(args[args.index("--hours") + 1]) if "--hours" in args else 24
    out = subprocess.run(["gh", "issue", "list", "--state", "all", "-L", "300", "--json",
                          "number,title,state,labels,closedAt,comments"], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(out.stderr.strip() or "gh failed")
    since = datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(hours=hours)
    rows, total = [], {}
    for it in json.loads(out.stdout):
        labels = {l["name"] for l in it["labels"]}
        scenes, answered = batch(it["comments"])
        note = ""
        if it["state"] == "CLOSED":
            closed = datetime.datetime.fromisoformat(it["closedAt"].replace("Z", "+00:00"))
            if closed < since:
                continue
            state, note = "done", closed.astimezone().strftime("closed %H:%M")
        elif "needs-iray" in labels:
            state = "needs-iray"
            note = ", ".join(f"{n} x {s}" for s, n in sorted(scenes.items())) if scenes else "!! no batch comment"
            for s, n in (scenes or {}).items():
                total[s] = total.get(s, 0) + n
        elif "question" in labels:
            state = "question"
        elif it["number"] in HELD:
            state, note = "held", HELD[it["number"]]
        elif scenes is not None and answered:
            state, note = "resume", "references in"
        else:
            state = "open"
            if scenes is not None:
                note = "!! batch comment without the label"
        rows.append((ORDER.index(state), state == "open" and "bug" not in labels, it["number"], state, it["title"], note))
    rows.sort()
    count = {s: sum(1 for r in rows if r[3] == s) for s in ORDER}
    show = [r for r in rows if "--all" in args or r[3] not in ("open", "held")]

    print("| # | State | Issue | Note |")
    print("|---|-------|-------|------|")
    for _, _, n, state, title, note in show:
        print(f"| {n} | {state} | {title[:70]} | {note} |")
    if not show:
        print("| - | - | nothing closed or parked yet | |")
    print()
    print(" · ".join(f"{s} {count[s]}" for s in ORDER) + f" (done = closed in the last {hours} h)")
    if total:
        scenes = ", ".join(f"{n} x {s} samples" for s, n in sorted(total.items()))
        print(f"Iray batch waiting: {sum(total.values())} scenes ({scenes}) in {count['needs-iray']} issues")
    nxt = next((r for r in rows if r[3] in ("resume", "open")), None)
    print(f"Next: #{nxt[2]} {nxt[4]}" if nxt else "Next: nothing left to pick")


if __name__ == "__main__":
    main()
