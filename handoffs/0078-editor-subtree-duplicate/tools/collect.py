#!/usr/bin/env python3
"""Assemble handoff 0078's committed evidence from a final verification run and a capture set.

Inputs: the final run directory (tsc, Vitest and build outputs, dist hashes,
mutations.json) and the capture set directory under artifacts/ (PNGs and report.json,
ignored by git). Outputs: evidence/*.txt|json, screenshots/*.png and
screenshots/manifest.json. Text logs are stored without ANSI codes or trailing
whitespace (git diff --check); the SHA-256 of every raw input is recorded beside it.

Usage (worktree root):
  python3 -I handoffs/0078-editor-subtree-duplicate/tools/collect.py <final-run-dir> <capture-set-dir>
"""
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HANDOFF = ROOT / "handoffs/0078-editor-subtree-duplicate"
DEPS = ROOT.parent.parent / "node_modules"
NODE = "/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin/node"
CHANGED = [
    "editor/ui/src/hierarchy/duplicate.ts",
    "editor/ui/src/components/HierarchyPanel.tsx",
    "editor/ui/src/shell/ShellContext.tsx",
    "editor/ui/src/components/ShortcutsDialog.tsx",
    "editor/ui/src/icons/Icon.tsx",
    "editor/ui/DESIGN.md",
    "editor/ui/src/components/HierarchyDuplicate.test.tsx",
    "editor/ui/src/components/HierarchyMenu.test.tsx",
    "editor/ui/src/components/ShortcutsDialog.test.tsx",
    "editor/ui/src/shell/ShellContext.test.tsx",
]
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def clean(text: str) -> str:
    lines = [ANSI.sub("", line).rstrip() for line in text.splitlines()]
    while lines and not lines[-1]:
        lines.pop()
    while lines and not lines[0]:
        lines.pop(0)
    return "\n".join(lines) + "\n"


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def version(package: str) -> str:
    return json.loads((DEPS / package / "package.json").read_text())["version"]


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("usage: collect.py <final-run-dir> <capture-set-dir>")
    final, captures = Path(sys.argv[1]), Path(sys.argv[2])
    evidence = HANDOFF / "evidence"
    shots = HANDOFF / "screenshots"
    for directory in (evidence, shots):
        if directory.exists() and any(directory.iterdir()):
            raise SystemExit(f"{directory} is not empty")

    head = (final / "head.txt").read_text().strip()
    if head != git("rev-parse", "HEAD"):
        raise SystemExit("the final run was made at another commit")
    write(evidence / "source.json", json.dumps({
        "commit": head,
        "parent": git("rev-parse", f"{head}^"),
        "branch": git("rev-parse", "--abbrev-ref", "HEAD"),
        "tree": git("rev-parse", f"{head}^{{tree}}"),
        "blobs": {path: git("rev-parse", f"{head}:{path}") for path in CHANGED},
        "brief_sha256": sha(HANDOFF / "brief.md"),
        "package_lock_sha256": {"worktree": sha(ROOT / "package-lock.json"), "parent_checkout": sha(ROOT.parent.parent / "package-lock.json")},
        "tools": {
            "node": subprocess.run([NODE, "--version"], capture_output=True, text=True, check=True).stdout.strip(),
            "typescript": version("typescript"),
            "vitest": version("vitest"),
            "vite": version("vite"),
            "react": version("react"),
            "axe-core": version("axe-core"),
            "playwright-core": version("playwright-core"),
            "chrome": json.loads((captures / "report.json").read_text())["browser"],
        },
        "machine": {
            "model": subprocess.run(["/usr/sbin/sysctl", "-n", "hw.model"], capture_output=True, text=True, check=True).stdout.strip(),
            "macos": subprocess.run(["/usr/bin/sw_vers", "-productVersion"], capture_output=True, text=True, check=True).stdout.strip(),
            "build": subprocess.run(["/usr/bin/sw_vers", "-buildVersion"], capture_output=True, text=True, check=True).stdout.strip(),
        },
    }, indent=2) + "\n")

    tsc = (final / "tsc.out").read_text()
    write(evidence / "typecheck.txt", "node ../../node_modules/typescript/bin/tsc -b editor/ui --noEmit\n"
          f"{(final / 'tsc.exit').read_text().strip()}\noutput: {len(tsc)} bytes\n{clean(tsc) if tsc.strip() else ''}")

    report = json.loads((final / "vitest-full.json").read_text())
    files = sorted(
        ({"file": str(Path(suite["name"]).relative_to(ROOT)), "tests": len(suite["assertionResults"]),
          "passed": sum(test["status"] == "passed" for test in suite["assertionResults"])} for suite in report["testResults"]),
        key=lambda item: item["file"],
    )
    focused = [
        {"file": str(Path(suite["name"]).relative_to(ROOT)), "test": test["fullName"], "status": test["status"]}
        for suite in report["testResults"] if Path(suite["name"]).name in ("HierarchyDuplicate.test.tsx", "HierarchyMenu.test.tsx", "ShortcutsDialog.test.tsx", "ShellContext.test.tsx")
        for test in suite["assertionResults"]
    ]
    write(evidence / "vitest-full-summary.json", json.dumps({
        "command": "node <checkout>/node_modules/vitest/vitest.mjs run --reporter=json (in editor/ui)",
        "exit": (final / "vitest-full.exit").read_text().strip(),
        "raw_json_sha256": sha(final / "vitest-full.json"),
        "totals": {key: report[key] for key in report if key.startswith("num")},
        "success": report["success"],
        "files": files,
        "focused_tests": focused,
    }, indent=2) + "\n")
    write(evidence / "vitest-focused.txt", f"{(final / 'vitest-focused.exit').read_text().strip()}\nraw sha256 {sha(final / 'vitest-focused.out')}\n\n{clean((final / 'vitest-focused.out').read_text())}")
    write(evidence / "build.txt", f"{(final / 'build.exit').read_text().strip()}\nraw sha256 {sha(final / 'build.out')}\n\n{clean((final / 'build.out').read_text())}\n"
          f"dist SHA-256 (editor/ui):\n{(final / 'dist-hashes.txt').read_text()}")
    shutil.copyfile(final / "mutations.json", evidence / "mutations.json")
    shutil.copyfile(captures / "report.json", evidence / "browser-report.json")

    capture_report = json.loads((captures / "report.json").read_text())
    shots.mkdir(parents=True, exist_ok=True)
    manifest = {
        "label": "FIXTURE captures: tools/fake-host.js (or handoff 0044's Play double) under the real editor/bridge/native.generated.js, relabelled as a fixture; headless Chrome; not live engine, native host or WebKit evidence",
        "set": capture_report["set"],
        "browser": capture_report["browser"],
        "captures": [],
    }
    for capture in capture_report["captures"]:
        source = captures / capture["file"]
        shutil.copyfile(source, shots / capture["file"])
        measured = capture["measured"]
        header = (shots / capture["file"]).read_bytes()[:24]
        manifest["captures"].append({
            "file": capture["file"],
            "sha256": sha(shots / capture["file"]),
            "state": capture["state"],
            "scenario": capture["scenario"],
            "viewport_css": [capture["width"], capture["height"]],
            # capture.mjs renders every state at device scale 2 except `menu-1x`.
            "device_scale": 1 if capture["state"] == "menu-1x" else 2,
            "png_pixels": [int.from_bytes(header[16:20], "big"), int.from_bytes(header[20:24], "big")],
            "status_line": measured["status"],
            "selected": [row["name"] for row in (measured.get("tree") or {}).get("selected", [])],
            "focus": measured["active"],
            "axe_violations": len(capture["axe"]),
            "page_errors": len(capture["errors"]),
            "step_error": capture["stepError"],
        })
    write(shots / "manifest.json", json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    print(f"evidence: {sorted(p.name for p in evidence.iterdir())}")
    print(f"screenshots: {len(manifest['captures'])}")


if __name__ == "__main__":
    main()
