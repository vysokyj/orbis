---
name: issue-loop
description: Work through open ORBIS issues unattended, one per iteration - when an issue needs Iray renders, prepare the DAZ scenes, write the batch into the issue, label it needs-iray and move on instead of asking. Use with /loop ("/loop /issue-loop", optionally with issue numbers), when asked to "work through the issues", or when the user says the Iray Server runs and parked batches should be queued.
---

# Issue loop (one issue per iteration, Iray parked in the issue)

The user's rule (2026-11-11): don't call them for an Iray run per task. Work goes on without them; what needs the Iray Server waits in the issues and is rendered in one go when they choose to start the server.

**Standing permission inside this loop** (2026-11-11): headless DAZ (`daz-script.sh`, `daz-dump.sh`, `daz-build-*.dsa`, `make-*`) and Octane X renders / probes (`octane-render.sh`, `octane-run.sh`, `calib-variants.sh`) start **without a question**. Still:
- `python3 scripts/dev/status.py` before the first render of an iteration; the scripts take turns through the render lock, call them and let them wait.
- The user said they work in an app -> hands off that app until they say otherwise.
- **Never** local CPU Iray (`daz-render.sh`), never queue the Iray Server on our own, never start the queue-side DAZ.
- Octane failed -> leave Octane X open (#50), don't retry; park the issue (below, `question`) and go on with issues that need no Octane. Small `Calib-*` scenes or half resolution only.

## One iteration
The session that runs the loop only orchestrates; the work on an issue is done by the **`issue-worker` agent (Opus)**, so each issue starts with a clean context. One worker at a time, never two (parallel sessions fought over sources and Octane, #74).

1. `git pull --rebase`; `gh issue list -L 100 --json number,title,labels`.
2. **Pick** (issue numbers given as arguments win, in that order):
   1. an issue whose `## Iray batch` comment is answered by a later `Iray references in` comment (label gone) - a resume;
   2. `bug` issues, oldest first;
   3. the rest, oldest first.
   Skip issues labelled `needs-iray` or `question`, and what the project rules hold back: #57 / #19 (PBRSkin: no constant tuning), #3 (signing needs the user), #50 (a 1.2 M-face private scene; its renders froze the Mac), anything that needs private or store content in git.
3. **Start the worker**: Agent tool, `subagent_type: issue-worker`, prompt = the issue number and title, "resume" if it is one, and what earlier workers reported that concerns it. Wait for its report; don't work on anything that touches the sources or renders meanwhile.
4. **Check its end state** - the report is not proof. It must be exactly one of:
   - **Done**: the issue is closed, the commit is on `origin/master`, `cargo test` passes, `scripts/install.sh` ran if the export changed.
   - **Needs Iray**: parked as in the next section (scenes in git, the comment, the label).
   - **Needs the user** (a decision, missing knowledge, a failed Octane, content we may not use): a comment with what was found and the one question, label `question`; what is worth keeping committed (`refs #N`).
   None of them (the worker died, left a dirty tree or failing tests): repair it here or park the issue with `question`; never start the next worker on a broken tree.
5. `scripts/dev/clean-cache.sh` if a compare loop ran.
6. **Show the table**: the `issue-status` skill (`python3 scripts/dev/issue-status.py`), after every iteration, unchanged in shape. Next iteration.

What the worker does in between: the usual skills - `docs-first` and `mi-fixture` before any render, decompose before fitting, `calib-loop` for Octane, `lab-note` for a calibration session, `ship-fix` to finish.

**Stop the loop** when nothing is left to pick. Final message (and the `notify` skill, if present): the `issue-status` table, and for the parked Iray batch the ETA from `status.py`'s s/scene and the DAZ restarts needed.

## Parking an issue that needs Iray
The worker's job. Do everything that doesn't need the server first, so the resume is only a compare:

1. Generate the scenes (`make-*.py`, `daz-build-*.dsa`; DAZ built-ins / primitives / free DAZ add-ons only for `calib/scenes`). Include the variants the decomposition will ask for (w0, tcwhite, dwhite, gloss off...) and what will probably be needed next - a second round costs the user another server start.
2. Check each scene: it opens, `python3 scripts/dev/duf_camera.py --fix` (a scene without `current_camera` is refused, #97), list the NVIDIA presets it uses (#94).
3. Dump (`daz-dump.sh`), `.mi` fixture if the conversion is in question, and render the **Octane side now**.
4. Commit and push the generator and the scenes (`refs #N`); the batch must be reproducible from git.
5. Comment on the issue, this exact shape (the collector reads it), then `gh issue edit N --add-label needs-iray`:

   ```markdown
   ## Iray batch

   | Scene | Samples | Why |
   |-------|---------|-----|
   | calib/scenes/Calib-Foo.duf | 5000 | reference for ... |
   | calib/scenes/Calib-Foo-w0.duf | 400 | translucency off: splits ... |

   NVIDIA presets: none | <preset> (Calib-Foo)
   Octane side: rendered, ~/Library/Caches/orbis/calib/octane/enhanced-final/ | pending (why)

   Resume: <what to compare, which hypothesis each scene decides, what to do on either outcome>
   ```

   Paths relative to the repo; a private scene by its name only (never its content). A newer `## Iray batch` comment replaces an older one.

## The user says the server runs ("server bezi", "pust Iray davku")
The orchestrating session's job, never a worker's. The comments are the announcement: their go-ahead covers **every batch parked under `needs-iray` at that moment**, nothing else.

1. `gh issue list --label needs-iray --json number,title`; read each issue's last `## Iray batch` comment.
2. Merge into one ordered list (issue order, references before variants; identical scene + samples once). Check NVIDIA preset repeats across the whole batch and against what the running queue DAZ already rendered: a repeat needs a queue-side DAZ restart - say where to split **before** queueing.
3. Show the list (scene, samples, issue) and the ETA, then queue it in one go per sample count: `scripts/dev/iray-queue.sh -s <samples> <scene.duf>...`. A refusal (no queue script, DAZ not running, rejected session) = tell the user, don't retry.
4. Meanwhile keep looping on issues that don't wait (Octane may render beside the server queue); `progress` instead of polling.
5. Renders done -> `iray-refs` (`keep-refs.py`, commit as they come). Per issue: comment `Iray references in: <commit>` and `gh issue edit N --remove-label needs-iray`. The next iteration resumes it.

Asked "what waits for Iray?" at any time: the `issue-status` table, and steps 1-2 for the scene list; nothing queued.
