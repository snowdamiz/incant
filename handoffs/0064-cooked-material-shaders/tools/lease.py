#!/usr/bin/env python3
"""Exclusive Cargo target lease for handoff 0064. Standard library only.

  python3 -I -B lease.py acquire   # after Astra's review-ready.json grant
  python3 -I -B lease.py release   # after the last owned cargo/rustc exits

The lease records the grant hash and the build rules. Release leaves the target
in place as a warm cache; nothing is deleted.
"""

import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402


def git(*args):
    return subprocess.run(["git", *args], cwd=c.ROOT, capture_output=True, text=True, check=True).stdout.strip()


def owned_builds():
    """cargo/rustc/clippy processes whose arguments mention this target or worktree (by executable name,
    so a shell command line that merely mentions the paths does not match itself)."""
    listing = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
    found = []
    for line in listing.splitlines():
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        program = Path(parts[1].split(" ", 1)[0]).name
        if program in ("cargo", "rustc", "clippy-driver", "rustdoc", "cargo-clippy") and (
                str(c.TARGET) in parts[1] or str(c.ROOT) in parts[1]):
            found.append(line.strip())
    return found


def acquire():
    grant_sha = c.check_grant()
    if c.LEASE.exists():
        raise SystemExit("lease already recorded")
    c.write_new_json(c.LEASE, {
        "packet": c.PACKET, "holder": "claude (Opus 5.5, Max thinking) for handoff 0064",
        "target_dir": str(c.TARGET), "assigned_by": "Astra via review-ready.json (0063 released it)",
        "grant_sha256": grant_sha, "grant": c.read_json(c.GRANT),
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"), "worktree": str(c.ROOT),
        "worktree_head": git("rev-parse", "HEAD"), "acquired_at": c.now(),
        "processes_on_target_at_acquire": owned_builds(),
        "rules": ["./tools/cargo only", "--locked", "--profile ci", "at most -j2",
                  "binaries/evidence only under " + str(c.EVIDENCE), "no other agent's target or packet files"]})
    print(c.LEASE.read_text())


def release():
    lease = c.read_json(c.LEASE)
    if lease.get("released_at"):
        raise SystemExit("already released")
    remaining = owned_builds()
    if remaining:
        raise SystemExit(f"cargo/rustc still running on the target: {remaining}")
    lease["released_at"] = c.now()
    lease["release_note"] = ("No 0064 cargo/rustc process remains on the target. Contents left in place as "
                             "warm cache; nothing deleted. Frozen binaries live under " + str(c.EVIDENCE / "bin"))
    text = json.dumps(lease, indent=1) + "\n"
    temporary = c.LEASE.with_suffix(".tmp")
    with open(temporary, "x") as out:
        out.write(text)
    temporary.replace(c.LEASE)
    print(text)


if __name__ == "__main__":
    if sys.argv[1:] == ["acquire"]:
        acquire()
    elif sys.argv[1:] == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)
