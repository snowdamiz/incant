#!/usr/bin/env python3
"""0073 owner_quality_blend: build and check reconstruction_r3.patch against exact f075b3900.

Standard library and read-only git only (run from the child worktree):

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_blend/controls/patch_check.py \
        --out handoffs/0073-upscaling-review/owner_quality_blend/controls/patch_check.json

1. For every file under ../src, read the f075b3900 blob with `git show` and emit a
   git-style unified diff (three context lines) into ../reconstruction_r3.patch.
2. Re-apply each hunk in pure Python to the base text and require the exact proposed
   bytes.
3. Run `git apply --cached --check` against a temporary index of f075b3900, kept
   inside the packet folder and deleted afterwards. No object or tree is written.
"""
import argparse
import difflib
import hashlib
import json
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True
PRODUCTION = "76be9608e99bf089fcb1e0908827330f70c28ac1"
HERE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "controls")
PACKET = os.path.dirname(HERE)
SRC = os.path.join(PACKET, "src")
PATCH = os.path.join(PACKET, "reconstruction_r3.patch")


def git(*args, data=None, env=None):
    return subprocess.run(["git", *args], input=data, capture_output=True, check=True, env=env).stdout


def sha(data):
    return hashlib.sha256(data).hexdigest()


def proposed_files():
    out = []
    for root, _, files in os.walk(SRC):
        for name in sorted(files):
            full = os.path.join(root, name)
            out.append(os.path.relpath(full, SRC).replace(os.sep, "/"))
    return sorted(out)


def apply_hunks(original, diff_lines):
    """Strict application of one file's hunks; context and removals must match."""
    src = original.splitlines(keepends=True)
    out, pos = [], 0
    i = 0
    while i < len(diff_lines):
        m = re.match(r"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", diff_lines[i])
        if not m:
            i += 1
            continue
        start = int(m.group(1)) - (0 if m.group(2) == "0" else 1)
        out.extend(src[pos:start])
        pos = start
        i += 1
        while i < len(diff_lines) and not diff_lines[i].startswith("@@"):
            line = diff_lines[i]
            tag, body = line[:1], line[1:] + "\n"
            if tag == " ":
                assert src[pos] == body, f"context mismatch at {pos + 1}"
                out.append(body)
                pos += 1
            elif tag == "-":
                assert src[pos] == body, f"removal mismatch at {pos + 1}"
                pos += 1
            elif tag == "+":
                out.append(body)
            i += 1
    out.extend(src[pos:])
    return "".join(out)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    top = git("rev-parse", "--show-toplevel").decode().strip()
    assert git("cat-file", "-t", PRODUCTION).strip() == b"commit"
    chunks, rows = [], []
    for rel in proposed_files():
        path = f"crates/{rel.split('crates/', 1)[1]}" if rel.startswith("crates/") else rel
        old = git("show", f"{PRODUCTION}:{path}").decode()
        with open(os.path.join(SRC, rel), "rb") as fh:
            new_bytes = fh.read()
        new = new_bytes.decode()
        old_blob = git("rev-parse", f"{PRODUCTION}:{path}").decode().strip()
        new_blob = git("hash-object", "--stdin", data=new_bytes).decode().strip()
        diff = list(difflib.unified_diff(old.splitlines(), new.splitlines(), f"a/{path}", f"b/{path}", lineterm="", n=3))
        if not diff:
            rows.append({"path": path, "changed": False})
            continue
        assert old.endswith("\n") and new.endswith("\n")
        body = [f"diff --git a/{path} b/{path}", f"index {old_blob[:12]}..{new_blob[:12]} 100644"] + diff
        chunks.append("\n".join(body) + "\n")
        assert apply_hunks(old, diff[2:]) == new, f"{path}: hunks do not reproduce the proposed file"
        rows.append({"path": path, "changed": True, "base_blob": old_blob, "base_sha256": sha(old.encode()),
                     "proposed_sha256": sha(new_bytes), "proposed_blob": new_blob,
                     "added": sum(1 for l in diff[2:] if l.startswith("+")),
                     "removed": sum(1 for l in diff[2:] if l.startswith("-"))})
    patch = "".join(chunks)
    with open(PATCH, "w") as fh:
        fh.write(patch)
    index = os.path.join(PACKET, ".r3-76be-check-index")
    env = dict(os.environ, GIT_INDEX_FILE=index)
    try:
        git("read-tree", PRODUCTION, env=env)
        check = subprocess.run(["git", "-C", top, "apply", "--cached", "--check", PATCH],
                               capture_output=True, env=env)
        applies = check.returncode == 0
        detail = check.stderr.decode().strip()
    finally:
        for leftover in (index, index + ".lock"):
            if os.path.exists(leftover):
                os.remove(leftover)
    result = {
        "production": PRODUCTION,
        "patch": {"path": "reconstruction_r3.patch", "sha256": sha(patch.encode()), "bytes": len(patch.encode()),
                  "files": sum(1 for r in rows if r["changed"])},
        "python_reapply_reproduces_proposed_files": True,
        "git_apply_cached_check_against_76be": applies,
        "git_apply_stderr": detail,
        "files": rows,
        "scope": "patch construction and applicability only; no build, test or GPU",
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(text)
    return 0 if applies else 1


if __name__ == "__main__":
    sys.exit(main())
