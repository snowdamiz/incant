#!/usr/bin/env python3
"""Collect handoff 0077's run logs into evidence/runs as deterministic gzip (mtime 0),
byte-exact, with raw and retained SHA-256 in evidence/runs/manifest.json. Logs carry ANSI
colour and jsdom warnings, so they are kept compressed rather than edited.

Usage (worktree root): /opt/homebrew/bin/python3 -I handoffs/0077-project-file-ui/tools/collect.py
"""
import gzip, hashlib, json, shutil
from pathlib import Path

OUT = Path("handoffs/0077-project-file-ui/evidence/runs")
LOGS = [
    # (name, source, what it is, outcome as observed)
    ("existing-related-tests-before-new-tests.txt", "/tmp/0077-existing-1.txt", "7 existing titlebar/shortcut/menu/shell files after the first implementation", "exit 0: 7 files, 99 tests passed"),
    ("run2-projectfile.txt", "/tmp/0077-logs/run2-projectfile.txt", "new flow tests, successor of original-failures/run1", "exit 0: 17 passed"),
    ("run3-projectfile-shell.txt", "/tmp/0077-logs/run3-projectfile-shell.txt", "after root's first revision: project-file + shell tests", "exit 1: 1 failed, 26 passed (an expectation of the superseded project-change wording)"),
    ("run4-projectfile-shell.txt", "/tmp/0077-logs/run4-projectfile-shell.txt", "successor of run3", "exit 0: 27 passed"),
    ("run5-projectfile.txt", "/tmp/0077-logs/run5-projectfile.txt", "with the closed-dialog late-pin test", "exit 0: 24 passed"),
    ("run6-projectfile-shell.txt", "/tmp/0077-logs/run6-projectfile-shell.txt", "after the hint-line and retired-wording fixes", "exit 0: 28 passed"),
    ("full2.txt", "/tmp/0077-logs/full2.txt", "full UI suite, serial", "exit 0: 47 files, 965 tests passed"),
    ("rev1-typecheck.txt", "/tmp/0077-final-typecheck.txt", "tsc -b --noEmit, delivered state of root's first revision", "exit 0, no output"),
    ("rev1-tests.txt", "/tmp/0077-final-tests.txt", "full UI suite, serial, delivered state of root's first revision", "exit 0: 47 files, 965 tests passed"),
    ("rev1-build.txt", "/tmp/0077-final-build.txt", "tsc -b && vite build, delivered state of root's first revision", "exit 0"),
    ("rev1-mutations.txt", "/tmp/0077-final-mutations.txt", "tools/mutations.py console, root's first revision", "exit 0: 5 of 6 mutations detected"),
    ("rev1-mutations.json", "/tmp/0077-logs/mutations-run2-revision1-final.json", "tools/mutations.py report, root's first revision", "5 of 6 detected; review-scope-guard not observable"),
    ("mutations-run1.json", "/tmp/0077-logs/mutations-run1.json", "first mutation run, before the undo/redo stale test", "stale-latch and review-scope-guard survived; the stale test was then corrected"),
    ("browser-report-r4.json", "/tmp/0077-logs/browser-report-r4.json", "capture set r4 report, root's first revision", "28 captures, 5 flows, axe 0, no page errors"),
    ("run7-projectfile-shell.txt", "/tmp/0077-r2-run1.txt", "after root's second revision (structured rejection): project-file + shell tests", "exit 0: 30 passed"),
    ("final-typecheck.txt", "/tmp/0077-r2-final-typecheck.txt", "final tsc -b --noEmit (app, node and test projects)", "exit 0, no output"),
    ("final-tests.txt", "/tmp/0077-r2-final-tests.txt", "final full UI suite, serial (vitest run --no-file-parallelism)", "exit 0: 47 files, 967 tests passed"),
    ("final-build.txt", "/tmp/0077-r2-final-build.txt", "final tsc -b && vite build", "exit 0"),
    ("final-mutations.txt", "/tmp/0077-r2-mutations.txt", "tools/mutations.py console (report: evidence/mutations.json)", "exit 0: 6 of 7 mutations detected"),
    ("capture-r1.txt", "/tmp/0077-capture-r1.txt", "browser capture set r1", "exit 0; review found the truncated project name (iterations/)"),
    ("capture-r2.txt", "/tmp/0077-capture-r2.txt", "browser capture set r2", "exit 0; review found the run-on hint and merged emphasis (iterations/)"),
    ("capture-r3.txt", "/tmp/0077-capture-r3.txt", "browser capture set r3", "exit 0"),
    ("capture-r4.txt", "/tmp/0077-capture-r4.txt", "browser capture set r4, root's first revision", "exit 0: 28 captures, 5 flows, axe 0, no page errors"),
    ("capture-r5.txt", "/tmp/0077-capture-r5.txt", "final browser capture set r5 (screenshots/, evidence/browser-report.json)", "exit 0: 30 captures, 5 flows, axe 0, no page errors; the 31 PNGs shared with r4 are byte-identical (evidence/r4-r5-comparison.json)"),
]
OUT.mkdir(parents=True, exist_ok=True)
manifest = {}
for name, source, what, outcome in LOGS:
    raw = Path(source).read_bytes()
    gz = gzip.compress(raw, 9, mtime=0)
    (OUT / f"{name}.gz").write_bytes(gz)
    manifest[name] = {"what": what, "outcome": outcome, "raw_bytes": len(raw), "raw_sha256": hashlib.sha256(raw).hexdigest(), "gzip_sha256": hashlib.sha256(gz).hexdigest()}
(OUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")

ITER = Path("handoffs/0077-project-file-ui/evidence/iterations")
ITER.mkdir(parents=True, exist_ok=True)
for src, dst in [
    ("artifacts/0077-project-file-ui/r1/menu-desktop-titlebar.png", "r1-menu-desktop-titlebar-truncated-name.png"),
    ("artifacts/0077-project-file-ui/r2/apply-failed-desktop-dialog.png", "r2-apply-failed-run-on-hint.png"),
    ("artifacts/0077-project-file-ui/r2/retired-desktop-dialog.png", "r2-retired-merged-emphasis.png"),
]:
    shutil.copyfile(src, ITER / dst)
print(json.dumps({k: v["outcome"] for k, v in manifest.items()}, indent=2))
