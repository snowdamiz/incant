#!/usr/bin/env python3
"""Exclusive Cargo target lease for handoff 0072 stage B. Standard library only.

Adapted from handoff 0068's committed tools/lease.py (read-only reuse); the
target comes from root's grant and is validated by common.target_allowed.

  python3 -I -B lease.py acquire   # only after root's review-ready.json grant
  python3 -I -B lease.py release   # after gpu.py release and the last owned build

Acquisition verifies every prior receipt the grant pins as prior_<name>_path /
prior_<name>_sha256, requires each pinned release body to list no live
processes, and requires that no cargo/rustc runs on the target or child
checkout and no binary runs from the target. Release leaves the target as a
warm cache; nothing is deleted.
"""

import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

BUILDERS = ("cargo", "rustc", "rustdoc", "clippy-driver", "cargo-clippy", "rustup")
ALIVE_FIELDS = ("owned_processes", "ledger_pids_alive", "processes_on_target_at_release")


def target_processes(target):
    """Builders mentioning the target or checkout, and executables run from the
    target or frozen binaries. Only matching lines are ever recorded."""
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


def prior_receipts(grant):
    pins = {}
    for key, value in grant.items():
        if key.startswith("prior_") and key.endswith("_path"):
            name = key[len("prior_"):-len("_path")]
            path = Path(value)
            pins[name] = {"path": value, "expected": grant.get("prior_" + name + "_sha256"),
                          "sha256": c.sha(path) if path.is_file() else None}
    ok = bool(pins) and all(p["expected"] and p["sha256"] == p["expected"] for p in pins.values())
    alive = {}
    for name, pin in pins.items():
        if ok and "release" in name:
            body = c.read_json(pin["path"])
            alive[name] = {k: body[k] for k in ALIVE_FIELDS if k in body}
    ok = ok and any("release" in name for name in pins) and all(
        value == [] for fields in alive.values() for value in fields.values())
    return {"pins": pins, "release_alive_fields": alive}, ok


def acquire():
    grant, grant_sha = c.grant()
    if c.STOP.exists() or c.LATCH.exists():
        c.refuse("stage already stopped or released")
    if c.LEASE.exists():
        c.refuse("lease already recorded")
    tools = c.tools_identity(grant)
    prior, ok = prior_receipts(grant)
    if not ok:
        print(json.dumps(prior, indent=1))
        c.refuse("the grant's prior receipts do not verify or list live processes")
    target = c.target(grant)
    running = target_processes(target)
    if running:
        c.refuse("processes already use the target: " + json.dumps(running)[:400])
    c.write_new_json(c.LEASE, {
        "packet": c.PACKET, "stage": c.STAGE,
        "holder": "claude (Opus 5.5, Max thinking) for handoff 0072 stage B",
        "target_dir": str(target), "assigned_by": "root via review-ready.json",
        "grant_sha256": grant_sha, "grant": grant, "prior_receipts": prior, "tools": tools,
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"), "worktree": str(c.ROOT),
        "child_checkout": str(c.CHECKOUT), "acquired_at": c.now(),
        "processes_on_target_at_acquire": running,
        "rules": ["./tools/cargo only, --locked --offline --profile ci -j2",
                  "builds from the git-archive child checkout of the granted commit only",
                  "one bounded process group at a time under the packet flock",
                  "GPU children only under the global native-gpu.lock, budget debited first",
                  "serial offscreen GPU children; no window, desktop capture, audio or accounts",
                  "<= 96 new submissions (<= 960x540), <= 16 PNGs; regression once, no retry",
                  "binaries and evidence only under " + str(c.EVIDENCE)]})
    print(c.LEASE.read_text())


def release():
    grant, _ = c.grant()
    if not c.LEASE.is_file():
        c.refuse("no lease to release")
    lease = c.read_json(c.LEASE)
    if lease.get("released_at"):
        raise SystemExit("already released")
    if not c.LATCH.exists():
        c.refuse("release the GPU (gpu.py release) before the target")
    remaining = target_processes(c.target(grant))
    if remaining:
        c.refuse("processes still use the target: " + json.dumps(remaining)[:400])
    lease["released_at"] = c.now()
    lease["processes_on_target_at_release"] = remaining
    lease["release_note"] = ("No 0072 stage B cargo/rustc process or frozen binary remains on "
                             "the target. Contents left as warm cache; nothing deleted.")
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
