#!/usr/bin/env python3
"""Exclusive Cargo target lease for handoff 0066. Standard library only.

Adapted from handoff 0064's committed tools/lease.py (read-only reuse).

  python3 -I -B lease.py acquire   # after Astra's review-ready.json grant
  python3 -I -B lease.py release   # after the last owned cargo/rustc exits

Acquisition re-verifies the prior holder's receipts named by the grant (0065's
release.json and lease.json, and the ledger prefix its release recorded) and
that no cargo/rustc runs on the target. Release leaves the target in place as
a warm cache; nothing is deleted.
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

PRIOR = Path("/Users/andreyyurlov/.codex/incant-evidence/0065")


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


def prior_receipts(grant):
    """The grant pins 0065's release, lease and the ledger prefix up to its release event."""
    ledger = (PRIOR / "ledger.jsonl").read_bytes()
    lines = ledger.split(b"\n")
    prefix = None
    for count in range(len(lines), 0, -1):
        candidate = b"\n".join(lines[:count]) + b"\n"
        if hashlib.sha256(candidate).hexdigest() == grant["prior_release_ledger_prefix_sha256"]:
            prefix = {"lines": count, "bytes": len(candidate),
                      "suffix": ledger[len(candidate):].decode()}
            break
    record = {
        "release": {"path": str(PRIOR / "release.json"), "sha256": c.sha(PRIOR / "release.json"),
                    "expected": grant["prior_release_sha256"]},
        "lease": {"path": str(PRIOR / "lease.json"), "sha256": c.sha(PRIOR / "lease.json"),
                  "expected": grant["prior_lease_sha256"]},
        "ledger": {"path": str(PRIOR / "ledger.jsonl"), "sha256": hashlib.sha256(ledger).hexdigest(),
                   "prefix_expected": grant["prior_release_ledger_prefix_sha256"], "prefix": prefix},
        "prior_release_body": c.read_json(PRIOR / "release.json")}
    ok = (record["release"]["sha256"] == record["release"]["expected"]
          and record["lease"]["sha256"] == record["lease"]["expected"] and prefix is not None
          and record["prior_release_body"].get("owned_processes_alive") == []
          and record["prior_release_body"].get("target_users") == [])
    return record, ok


def acquire():
    grant_sha = c.check_grant()
    source = c.check_source()
    if c.LEASE.exists():
        raise SystemExit("lease already recorded")
    grant = c.read_json(c.GRANT)
    prior, ok = prior_receipts(grant)
    if not ok:
        print(json.dumps(prior, indent=1))
        raise SystemExit("the grant's prior-release receipts do not verify; refusing")
    running = owned_builds()
    if running:
        raise SystemExit(f"cargo/rustc already running on the target: {running}")
    c.write_new_json(c.LEASE, {
        "packet": c.PACKET, "holder": "claude (Opus 5.5, Max thinking) for handoff 0066",
        "target_dir": str(c.TARGET), "assigned_by": "Astra via review-ready.json (0065 released it)",
        "grant_sha256": grant_sha, "grant": grant, "prior_receipts": prior, "source": source,
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"), "worktree": str(c.ROOT),
        "acquired_at": c.now(), "processes_on_target_at_acquire": running,
        "rules": ["./tools/cargo only", "--locked", "--offline", "--profile ci", "at most -j2",
                  "serial offscreen GPU children under an exclusive lock", "no native window or desktop capture",
                  "<= 24 retained frames, each <= 960x540", "GPU suites <= 300 s, captures <= 120 s",
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
    lease["processes_on_target_at_release"] = remaining
    lease["release_note"] = ("No 0066 cargo/rustc process remains on the target. Contents left in place as "
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
