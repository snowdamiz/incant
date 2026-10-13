#!/usr/bin/env python3
"""Exclusive Cargo target lease for the 0073 supervised CPU build. Standard library only.

Adapted from handoff 0068's committed, root-reviewed tools/lease.py (read-only reuse).

  python3 -I -B lease.py acquire   # only after root's build-ready.json grant
  python3 -I -B lease.py release   # after the last owned build (review, optional lint)

The lease covers only the CPU build: it is taken under build-ready.json, every
build step requires it held and unreleased under that same grant, and it is
released before any GPU step (gpu.py list refuses while it is held). GPU children
run the frozen binary and never use the target.

Acquisition verifies every prior receipt the build grant pins as a
prior_<name>_path / prior_<name>_sha256 pair (pinned release bodies must list no
live process) and requires that no cargo/rustc runs on the target or child
checkout and no binary runs from the target. Release leaves the target in place
as a warm cache; nothing is deleted.
"""

import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

BUILDERS = ("cargo", "rustc", "rustdoc", "clippy-driver", "cargo-clippy", "rustup")


def target_processes(target):
    """Builders mentioning the target or checkout, and executables run from the
    target or the frozen-binary directory. Matching is by executable name/path, so
    a shell that only mentions the paths does not match itself. Only matching
    lines are ever recorded."""
    listing = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
    found = []
    for line in listing.splitlines():
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        program = parts[1].split(" ", 1)[0]
        builder = Path(program).name in BUILDERS and (
            str(target) in parts[1] or str(c.CHECKOUT) in parts[1])
        if builder or program.startswith((str(target), str(c.BIN))):
            found.append(line.strip())
    return found


def acquire():
    grant, grant_sha = c.build_grant()
    if c.LATCH.exists():
        c.refuse("GPU and target already released by this packet")
    if c.LEASE.exists():
        c.refuse("lease already recorded; it is never re-acquired")
    tools = c.tools_identity(grant["visual_tree"])
    prior, ok = c.prior_receipts(grant, required=False)
    if not ok:
        print(json.dumps(prior, indent=1))
        c.refuse("the build grant's prior receipts do not verify")
    target = Path(grant["target"])
    running = target_processes(target)
    if running:
        c.refuse("processes already use the target: " + json.dumps(running)[:400])
    c.write_new_json(c.LEASE, {
        "packet": c.PACKET, "stage": c.BUILD_STAGE,
        "holder": "claude (Opus 5.5, Max thinking) for handoff 0073 supervised CPU build",
        "target_dir": str(target), "assigned_by": "root via build-ready.json",
        "grant_sha256": grant_sha, "grant": grant, "prior_receipts": prior, "tools": tools,
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"), "worktree": str(c.ROOT),
        "child_checkout": str(c.CHECKOUT), "acquired_at": c.now(),
        "processes_on_target_at_acquire": running,
        "rules": ["./tools/cargo only, --locked --offline --profile ci -j2, default features",
                  "builds from the git-archive child checkout of the granted commit only",
                  "every build input must equal root's granted source freeze",
                  "one bounded process group at a time under the packet flock",
                  "no GPU child while the lease is held; release it before gpu.py list",
                  "binaries and evidence only under " + str(c.EVIDENCE)]})
    print(c.LEASE.read_text())


def release():
    grant, _ = c.build_grant()
    if not c.LEASE.is_file():
        c.refuse("no lease to release")
    lease = c.read_json(c.LEASE)
    if lease.get("released_at"):
        raise SystemExit("already released")
    remaining = target_processes(Path(grant["target"]))
    if remaining:
        c.refuse("processes still use the target: " + json.dumps(remaining)[:400])
    lease["released_at"] = c.now()
    lease["processes_on_target_at_release"] = remaining
    lease["release_note"] = ("No 0073 cargo/rustc process or frozen binary remains on the target. "
                             "Contents left in place as warm cache; nothing deleted. Frozen "
                             "binaries live under " + str(c.BIN))
    temporary = c.LEASE.with_suffix(".tmp")
    with open(temporary, "x") as out:
        out.write(json.dumps(lease, indent=1) + "\n")
    temporary.replace(c.LEASE)
    print(c.LEASE.read_text())


if __name__ == "__main__":
    if sys.argv[1:] == ["acquire"]:
        acquire()
    elif sys.argv[1:] == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)
