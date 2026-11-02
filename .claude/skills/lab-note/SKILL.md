---
name: lab-note
description: Write the dated ORBIS calibration lab note in docs/notes and link it. Use at the end of (or during) any calibration session, or when asked for a lab note / session write-up.
---

# Lab note

Every calibration session gets a note `docs/notes/YYYY-MM-DD-<topic>.md` (`date +%F`), written **as findings land**, committed with the fix.

Content (not normative - the spec keeps only the winning rule):
- Goal and issue (#N), scenes used, variant (Look x mode).
- **All attempts, including failed ones** and why each was a dead end.
- Ratio tables (Octane / Iray, linear luminance, 1.00 = equal), timings, sample counts.
- Process lessons (what to do differently next time).
- Where it stands, what is open (link the issue; plans and to-dos live in GitHub issues, not docs).

Then:
1. Add a row to the table in `docs/notes/README.md` (`| [date topic](file.md) | topics |`).
2. Check the winning rule is in `docs/spec/octane-export.md` (same commit).
3. Keep third-party copyrighted text out (OTOY manual, NVIDIA docs, store content).
Everything in English.
