---
name: issue-status
description: One uniform table of where the ORBIS issue loop stands - closed lately, resumable, waiting for an Iray batch (scene counts), waiting for the user, next pick. Use after every issue-loop iteration, when the loop stops, and when the user asks how the issues stand / what waits for Iray ("stav issue", "co ceka na Iray").
---

# Issue status

Run `python3 scripts/dev/issue-status.py` (read-only, through `gh`; `--hours N` widens the "done" window from 24 h, `--all` lists the untouched open issues too) and show its output **as it is**: the same table, columns and order every time, so two iterations can be compared at a glance. Don't reword, reorder or drop rows.

Below it, at most:
- one line for the iteration that just ended: `#N -> done | needs-iray | question` and the reason in a few words (in the user's language);
- one line per `!!` in the Note column: a `needs-iray` issue without a `## Iray batch` comment, or a batch comment without the label - repair it (the `issue-loop` skill has the comment's shape) before the next iteration.

The long-running side (Iray queue, Octane, the render lock) is the `progress` skill; don't mix the two tables.
