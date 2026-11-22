---
name: issue-loop
description: Work through open ORBIS issues unattended, one per iteration - generic Iray scenes go to the home Iray Server and are waited for; when the queue does not run, prepare the DAZ scenes, write the batch into the issue, label it needs-iray and move on instead of asking. Use with /loop ("/loop /issue-loop", optionally with issue numbers), when asked to "work through the issues", or when the user says the Iray Server runs and parked batches should be queued.
---

# Issue loop (one issue per iteration)

The user's rule (2026-11-11): don't call them for an Iray run per task. Since 2026-11-16 the Iray Server is a free home machine (4 GB VRAM, skill `iray-queue`): a worker renders its generic scenes itself, headless (`iray-server.py submit`, #195), and goes on with the compare; only when the server doesn't answer or a scene needs the user (Demo-* / private), the issue is parked and the batch waits.

**Standing permission inside this loop** (2026-11-11): headless DAZ (`daz-script.sh`, `daz-dump.sh`, `daz-build-*.dsa`, `make-*`) and Octane X renders / probes (`octane-render.sh`, `octane-run.sh`, `calib-variants.sh`) start **without a question**. Still:
- `python3 scripts/dev/status.py` before the first render of an iteration; the scripts take turns through the render lock, call them and let them wait.
- The user said they work in an app -> hands off that app until they say otherwise.
- **Never** local CPU Iray (`daz-render.sh`). A worker never starts the queue-side DAZ (the orchestrating session may, skill `iray-queue`). A worker sends only generic scenes (`calib/scenes`, fresh `Calib-*`), with `iray-server.py submit`; parked batches are handled by the orchestrating session (below).
- Headless DAZ and Octane may run at once (2026-11-16): `daz-script.sh` takes the `daz` lock, `octane-run.sh` / `daz-render.sh` the `render` lock.
- Octane failed -> leave Octane X open (#50), don't retry; park the issue (below, `question`) and go on with issues that need no Octane. Small `Calib-*` scenes or half resolution only.

## One iteration
The session that runs the loop only orchestrates; the work on an issue is done by the **`issue-worker` agent (Opus)**, so each issue starts with a clean context. One worker at a time, never two (parallel sessions fought over sources and Octane, #74).

1. `git pull --rebase`; `gh issue list -L 100 --json number,title,labels`.
2. **Pick** (issue numbers given as arguments win, in that order, except over a resume):
   1. **resumes first, always**: an issue whose `## Iray batch` comment is answered by a later `Iray references in` comment (label gone), oldest batch first. Its scenes, dumps and Octane side are ready, so it is only a compare - and a compare tends to settle more (and to tell what the next batch must hold) than starting a new issue without a render. Check for them at every iteration, also while the server queue still runs: an issue whose own references are in is resumed at once;
   2. `bug` issues, oldest first;
   3. the rest, oldest first.
   Skip issues labelled `needs-iray` or `question`, and what the project rules hold back: #3 (signing needs the user), #50 (a 1.2 M-face private scene; its renders froze the Mac), anything that needs private or store content in git.
3. **Start the worker**: Agent tool, `subagent_type: issue-worker`, prompt = the issue number and title, "resume" if it is one, and what earlier workers reported that concerns it. Wait for its report; don't work on anything that touches the sources or renders meanwhile.
4. **Check its end state** - the report is not proof. It must be exactly one of:
   - **Done**: the issue is closed, the commit is on `origin/master`, `cargo test` passes, `scripts/install.sh` ran if the export changed.
   - **Needs Iray**: only when the server does not answer or a scene needs the user; parked as in the next section (scenes in git, the comment, the label), and the report ends with its scene list - checked against the issue's `## Iray batch` table (same scenes, same samples).
   - **Needs the user** (a decision, missing knowledge, a failed Octane, content we may not use): a comment with what was found and the one question, label `question`; what is worth keeping committed (`refs #N`).
   In every end state: **each scene the worker generated is in `calib/scenes` and committed with its generator** (`git status` clean, the report's scene list against `git log --stat`). The generators write into "My Library/Scenes"; not all of them copy the scene into the repo, and a scene that lives only there is lost for the next compare. Exception: a scene with store or private content stays out of git - then the generator alone is committed and the issue says so.
   None of them (the worker died, left a dirty tree or failing tests): repair it here or park the issue with `question`; never start the next worker on a broken tree.
5. `scripts/dev/clean-cache.sh` if a compare loop ran.
6. **Show the table**: the `issue-status` skill (`python3 scripts/dev/issue-status.py`), after every iteration, unchanged in shape. Next iteration.

What the worker does in between: the usual skills - `docs-first` and `mi-fixture` before any render, decompose before fitting, `calib-loop` for Octane, `lab-note` for a calibration session, `ship-fix` to finish.

**Stop the loop** when nothing is left to pick. Final message (and the `notify` skill, if present): the `issue-status` table, and for the parked Iray batch the ETA from `status.py`'s s/scene and the DAZ restarts needed.

## Iray inside an issue
The worker's job. Generic scenes: steps 1 - 2 below, then **at once** `python3 scripts/dev/iray-server.py submit -s <samples> <scenes>` as a background command - the server renders while you dump and render the Octane side (steps 3 - 4; the user's rule, 2026-11-17: never hold Iray back for Octane) - then `wait-for.sh` on it, store the references (`iray-refs`) and go on with the compare in the same run. Hair scenes last or in their own `submit` (#217); `submit` exiting 3 = the server's GPU failed and it is on its CPU fallback: keep what rendered, park the rest as a `## Iray batch` with `needs-iray` and say the user must restart the Iray Server. Otherwise park it - everything that doesn't need the server first, so the resume is only a compare:

1. Generate the scenes (`make-*.py`, `daz-build-*.dsa`; DAZ built-ins / primitives / free DAZ add-ons only for `calib/scenes`). Include the variants the decomposition will ask for (w0, tcwhite, dwhite, gloss off...); keep each scene small (4 GB VRAM).
2. Check each scene: it opens, `python3 scripts/dev/duf_camera.py --fix` (a scene without `current_camera` is refused, #97), list the NVIDIA presets it uses (#94).
3. Dump (`daz-dump.sh`), `.mi` fixture if the conversion is in question, and render the **Octane side now**.
4. Copy the scenes into `calib/scenes` with `python3 scripts/dev/duf_io.py copy <My Library scene>...` (blanks the contributor details DAZ saved, in the library file too, #208; never a plain `cp`) - some generators do it, `make-calib-scenes.py` / `make-light-scenes.py` and the `daz-build-*.dsa` don't; commit and push them with the generator (`refs #N`); the batch must be reproducible from git.
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

## Queue parked batches at once
The orchestrating session's job. A parked batch goes into the queue right away; if the queue does not run, start the queue-side DAZ and its queue script (skill `iray-queue`) - the Iray Server itself is the user's.

1. After a worker parks an issue (step 4 checked), queue its batch at once: `scripts/dev/iray-queue.sh -n -s <samples> <scene.duf>...` per sample count, in the batch's order (a private scene by its My Library path). DAZ / the queue script need not run.
2. Before queueing, check NVIDIA preset / module repeats (`pbr_skin`, `dual_lobe_hair`, NVIDIA presets: one import per DAZ session, #94) against what is already queued and what the running queue DAZ rendered. A repeat is **held back**: list it in `issue-status`'s note; when the queue has rendered what is before it, quit the queue-side DAZ, queue it with `iray-queue.sh -n --held ...` - it refuses while the queue-side DAZ still runs (a running queue grabs a new scene at once, in the old session; #183: one rendered on the CPU because the server was already stopped). then start DAZ and the queue again. A refusal means DAZ still runs: never queue it without `--held`.
   `iray-queue.sh` also refuses any batch while a running queue's DAZ has lost its server connection (#183); then tell the user and queue after their restart.
3. Demo-* or private scenes in a batch: ask the user before queueing them (4 GB VRAM).
4. Meanwhile keep looping (Octane may render beside the server queue); `progress` instead of polling. `iray-queue.sh --collect` moves finished renders to their out dir.
5. Renders done -> `iray-refs` (`keep-refs.py`, commit as they come). Per issue: comment `Iray references in: <commit>` and `gh issue edit N --remove-label needs-iray`. The next iteration resumes it.

Asked "what waits for Iray?" at any time: the `issue-status` table, the queue folder (`ls ~/Library/Caches/orbis/calib/queue/*.duf`) and the held-back scenes.
