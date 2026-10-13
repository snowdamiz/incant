#!/usr/bin/env python3
"""Exclusive Cargo target lease for handoff 0063. Standard library only.

  python3 -I -B lease.py acquire   # after Astra's review-ready.json names 0063
  python3 -I -B lease.py release   # after the last build; no owned cargo/rustc may remain

The target directory is left in place as warm cache; nothing is deleted.
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


def acquire():
    grant_sha = c.grant()
    record = {
        "packet": c.PACKET,
        "holder": "claude (Opus 5.5, Max thinking) for handoff 0063",
        "target_dir": str(c.TARGET),
        "assigned_by": "Astra via review-ready.json (0062 released it)",
        "grant_sha256": grant_sha,
        "grant": c.read_json(c.GRANT),
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"),
        "worktree": str(c.ROOT),
        "worktree_head": git("rev-parse", "HEAD"),
        "acquired_at": c.now(),
        "rules": ["./tools/cargo only", "--locked", "--profile ci", "at most -j2",
                  f"binaries/evidence only under {c.EVIDENCE}",
                  "no other agent's target or packet files"],
        "released_at": None,
    }
    c.write_new_json(c.LEASE, record)
    print(json.dumps(record, indent=1))


def release():
    lease = c.read_json(c.LEASE)
    if lease.get("released_at"):
        raise SystemExit("already released")
    listing = subprocess.run(["ps", "-axo", "pid=,command="], capture_output=True, text=True).stdout
    owned = [line.strip() for line in listing.splitlines()
             if str(c.TARGET) in line and ("cargo" in line or "rustc" in line)]
    if owned:
        raise SystemExit(f"processes still use the target: {owned}")
    lease["released_at"] = c.now()
    lease["release_note"] = ("No 0063 cargo/rustc process remains on the target. Contents left in place as "
                             "warm cache; nothing deleted. Frozen binaries live under " + str(c.EVIDENCE / "bin"))
    c.LEASE.write_text(json.dumps(lease, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(lease, indent=1))


if __name__ == "__main__":
    if sys.argv[1:] == ["acquire"]:
        acquire()
    elif sys.argv[1:] == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)
