#!/usr/bin/env python3
"""Mutation checks for handoff 0077: each mutation removes one guard from the UI source,
runs the project-file and shell tests serially, records which tests fail, and restores the
file byte-exactly (verified by SHA-256). Run from the worktree root:

    /opt/homebrew/bin/python3 -I handoffs/0077-project-file-ui/tools/mutations.py <report.json>

Uses the parent checkout's vitest read-only; changes nothing else.
"""
import hashlib, json, re, subprocess, sys
from pathlib import Path

UI = Path("editor/ui")
VITEST = Path("../../../../node_modules/.bin/vitest")
TESTS = ["src/components/ProjectFile.test.tsx", "src/shell/ShellContext.test.tsx"]
CONTEXT = "src/projectFile/ProjectFileContext.tsx"
SHELL = "src/shell/ShellContext.tsx"
MODEL = "src/projectFile/model.ts"

MUTATIONS = [
    ("save-scope-guard", CONTEXT,
     "      const result = await request({ type: 'project.save' }, { quiet: true });\n      if (!belongs(run)) return;\n",
     "      const result = await request({ type: 'project.save' }, { quiet: true });\n"),
    ("apply-scope-guard", CONTEXT,
     "      if (!belongs(run)) return;\n      const shown = id === ticket.current;\n",
     "      const shown = id === ticket.current;\n"),
    ("review-scope-guard", CONTEXT,
     "      if (id !== ticket.current || !belongs(run)) return;\n",
     "      if (id !== ticket.current) return;\n"),
    ("release-slot-on-scope-change", CONTEXT,
     "      running.current = null;\n      setBusy(null);\n      setNotice(null);\n      setScope(next);\n",
     "      setNotice(null);\n      setScope(next);\n"),
    ("shell-request-catch", SHELL,
     "      try {\n        result = await bridge.request(hostRequest);\n      } catch (error) {\n        // A rejection is not a result: the caller gets a failure it can show, never a\n        // pending state that cannot end, and nothing is claimed about what the host did.\n        result = rejectedRequest(error);\n      }\n",
     "      result = await bridge.request(hostRequest);\n"),
    # Root's second revision: unknown must come from the caught rejection, not from a code.
    ("rejected-marker", MODEL,
     "  unknown: wasRejected(result),\n",
     "  unknown: result.error.code === 'request.rejected',\n"),
    ("stale-latch", CONTEXT,
     "    else if (pin.status === 'ready' && changed !== null) setPin({ status: 'stale', review: pin.review, reason: changed });\n",
     ""),
]

ANSI = re.compile(r"\x1b\[[0-9;]*m")

def run_tests():
    proc = subprocess.run([str(VITEST), "run", "--no-file-parallelism", *TESTS], cwd=UI, capture_output=True, text=True, timeout=600)
    out = ANSI.sub("", proc.stdout + proc.stderr)
    failed = sorted({line.strip()[2:].rsplit(" ", 1)[0].strip() for line in out.splitlines() if line.strip().startswith("× ")})
    summary = [line.strip() for line in out.splitlines() if line.strip().startswith(("Tests ", "Test Files "))]
    return proc.returncode, failed, summary

def main():
    report = {"baseline": None, "mutations": []}
    code, failed, summary = run_tests()
    report["baseline"] = {"exit": code, "failed": failed, "summary": summary}
    if code != 0:
        raise SystemExit(f"baseline fails: {failed}")
    for name, rel, old, new in MUTATIONS:
        path = UI / rel
        original = path.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode()
        if text.count(old) != 1:
            raise SystemExit(f"{name}: anchor found {text.count(old)} times")
        try:
            path.write_text(text.replace(old, new, 1))
            code, failed, summary = run_tests()
        finally:
            path.write_bytes(original)
        assert hashlib.sha256(path.read_bytes()).hexdigest() == digest, f"{name}: restore mismatch"
        report["mutations"].append({"name": name, "file": rel, "exit": code, "failed": failed, "summary": summary, "restored_sha256": digest})
        print(f"{name}: exit={code} failed={len(failed)}")
    Path(sys.argv[1]).write_text(json.dumps(report, indent=2) + "\n")

main()
