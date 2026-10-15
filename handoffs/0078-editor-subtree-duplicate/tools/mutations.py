#!/usr/bin/env python3
"""Mutation checks for handoff 0078's Duplicate guards.

Each mutation removes or weakens one guard in the source by an exact, unique string
replacement, reruns the focused UI test files with the pinned Node, records which tests
fail, and restores the file byte-exactly (its SHA-256 is checked before and after). A
mutation that no test catches is reported, not hidden. Writes the given JSON path.

Usage (worktree root): python3 -I handoffs/0078-editor-subtree-duplicate/tools/mutations.py <out.json>
"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
UI = ROOT / "editor/ui"
NODE = "/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin/node"
VITEST = ROOT.parent.parent / "node_modules/vitest/vitest.mjs"
TESTS = [
    "src/components/HierarchyDuplicate.test.tsx",
    "src/components/HierarchyMenu.test.tsx",
    "src/components/ShortcutsDialog.test.tsx",
    "src/shell/ShellContext.test.tsx",
]
DUP = "editor/ui/src/hierarchy/duplicate.ts"
PANEL = "editor/ui/src/components/HierarchyPanel.tsx"
SHELL = "editor/ui/src/shell/ShellContext.tsx"
CATALOG = "editor/ui/src/components/ShortcutsDialog.tsx"

MUTATIONS = [
    ("held-key repeat guard", PANEL, "if (!event.repeat && active !== null) duplicate.start(active);", "if (active !== null) duplicate.start(active);"),
    ("shortcut takes the key (preventDefault)", PANEL, "if (isDuplicateShortcut(event, platform)) {\n      event.preventDefault();", "if (isDuplicateShortcut(event, platform)) {"),
    ("platform-specific modifier", DUP, "if (event.shiftKey || event.altKey || (platform === 'mac' ? !event.metaKey || event.ctrlKey : !event.ctrlKey || event.metaKey)) return false;", "if (!(event.metaKey || event.ctrlKey)) return false;"),
    ("same-task in-flight guard", DUP, "const reason = running.current ? BUSY : blocked(id);", "const reason = blocked(id);"),
    ("pending-state guard", DUP, "return pending !== null ? BUSY : null;", "return null;"),
    ("pending state rendered", DUP, "    setPending(id);\n", ""),
    ("capability guard", DUP, "if (!capabilities.has('entity.duplicate')) return unavailableMessage('entity.duplicate', bridge);", ""),
    ("scene guard", DUP, "if (tree.nodes[id]?.kind === 'scene') return SCENE;", ""),
    ("Play world guard", DUP, "if (play?.stoppable) return PLAYING;", ""),
    ("Play stopping guard", DUP, "if (play?.state === 'stopping') return STOPPING;", ""),
    ("vanished-source pre-check", DUP, "if (project === null || !source) {\n      announce(GONE);\n      return;\n    }", ""),
    ("success without a copy", DUP, "if (copy === undefined) {\n      announce(NO_COPY);\n      return;\n    }", ""),
    ("bridge and project scope", DUP, "if (latest.current.bridge !== host || after.connection.status !== 'ready' || after.connection.project.id !== project) return;", ""),
    ("bridge scope only", DUP, "latest.current.bridge !== host || ", ""),
    ("copy present in the latest snapshot", DUP, "if (after.hierarchy.status !== 'ready' || !after.hierarchy.value.nodes[copy]) {", "if (after.hierarchy.status !== 'ready') {"),
    ("selection moved on", DUP, "if (now !== chosen && now !== null) return;", ""),
    ("focus only if the tree had it", DUP, "if (focus.owned()) focus.claim(copy);", "focus.claim(copy);"),
    ("focus moves to the copy", DUP, "if (focus.owned()) focus.claim(copy);", ""),
    ("asker unmounted", DUP, "if (!result.ok || !mounted.current) return;", "if (!result.ok) return;"),
    ("menu item acts", PANEL, "onSelect: () => duplicate.start(menu.id),", "onSelect: () => undefined,"),
    ("menu item explains", PANEL, "unavailable: duplicate.blocked(menu.id) ?? undefined,", ""),
    ("tree aria-busy", PANEL, "aria-busy={duplicate.pending !== null || undefined}", ""),
    ("rejected dispatch resolves", SHELL, "      try {\n        result = await bridge.dispatch(command);\n      } catch (error) {\n        // As for requests: a rejection says only that no result arrived.\n        result = rejectedRequest(error);\n      }", "      result = await bridge.dispatch(command);"),
    ("unknown result not called a failure", SHELL, "const label = result.outcomeUnknown\n          ? (UNKNOWN_LABEL[command.type] ?? `${command.type} result unknown`)\n          : (FAILURE_LABEL[command.type] ?? `${command.type} failed`);", "const label = FAILURE_LABEL[command.type] ?? `${command.type} failed`;"),
    ("plain Duplicate failure label", SHELL, ", 'entity.duplicate': 'Duplicate failed' };", " };"),
    ("shortcut reference row", CATALOG, "      { action: 'Duplicate', keys: [['Mod', 'D']] },\n", ""),
]


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run_tests() -> dict:
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "report.json"
        env = {"PATH": f"{Path(NODE).parent}:/usr/bin:/bin:/usr/sbin:/sbin", "HOME": os.environ.get("HOME", "/tmp")}
        proc = subprocess.run([NODE, str(VITEST), "run", *TESTS, "--reporter=json", f"--outputFile={out}"], cwd=UI, env=env, capture_output=True, text=True, timeout=600)
        if not out.exists():
            return {"exit": proc.returncode, "failed": None, "total": None, "stderr_tail": proc.stderr[-2000:]}
        report = json.loads(out.read_text())
    failed = []
    total = 0
    for suite in report.get("testResults", []):
        for test in suite.get("assertionResults", []):
            total += 1
            if test.get("status") != "passed":
                failed.append(f"{Path(suite['name']).name} > {test.get('fullName')}")
    # A suite that fails to load has no assertion results; report it as such.
    broken = [Path(s["name"]).name for s in report.get("testResults", []) if s.get("status") == "failed" and not s.get("assertionResults")]
    return {"exit": proc.returncode, "failed": failed, "total": total, "broken_suites": broken}


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: mutations.py <out.json>")
    results = {"baseline": run_tests(), "mutations": []}
    if results["baseline"]["failed"]:
        raise SystemExit(f"baseline fails: {results['baseline']['failed']}")
    for name, rel, old, new in MUTATIONS:
        path = ROOT / rel
        original = path.read_bytes()
        before = sha(original)
        text = original.decode()
        if text.count(old) != 1:
            raise SystemExit(f"{name}: expected one match in {rel}, found {text.count(old)}")
        try:
            path.write_bytes(text.replace(old, new).encode())
            outcome = run_tests()
        finally:
            path.write_bytes(original)
        if sha(path.read_bytes()) != before:
            raise SystemExit(f"{name}: {rel} was not restored byte-exactly")
        caught = bool(outcome["failed"]) or bool(outcome.get("broken_suites"))
        results["mutations"].append({"name": name, "file": rel, "caught": caught, **outcome, "restored_sha256": before})
        print(f"{'caught' if caught else 'NOT CAUGHT'}: {name} ({len(outcome['failed'] or [])} failing)", flush=True)
    Path(sys.argv[1]).write_text(json.dumps(results, indent=2) + "\n")


if __name__ == "__main__":
    main()
