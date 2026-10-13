#!/usr/bin/env python3
"""Exclusive Cargo target lease for handoff 0067. Standard library only.

Adapted from handoff 0066's tools/lease.py (read-only reuse).

  python3 -I -B lease.py acquire   # after Astra's review-ready.json grant
  python3 -I -B lease.py release   # after the last owned cargo/rustc exits

Acquisition re-verifies the prior holder's receipts pinned by the grant (0066's
gpu-released.json, target-lease.json and gpu-ledger.jsonl), that 0066 recorded
no surviving process, and that no cargo/rustc runs on the target now. Release
leaves the target in place as a warm cache; nothing is deleted.
"""

import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402


def owned_builds():
    """cargo/rustc/clippy processes whose arguments mention this target or worktree (matched by
    executable name, so a shell command line that merely mentions the paths does not match)."""
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
    files = {"release": (Path(grant["prior_release_path"]), grant["prior_release_sha256"]),
             "lease": (Path(grant["prior_lease_path"]), grant["prior_lease_sha256"]),
             "ledger": (Path(grant["prior_ledger_path"]), grant["prior_ledger_sha256"])}
    record = {name: {"path": str(path), "sha256": c.sha(path), "expected": expected}
              for name, (path, expected) in files.items()}
    release = c.read_json(files["release"][0])
    lease = c.read_json(files["lease"][0])
    pids = [json.loads(line)["pid"] for line in files["ledger"][0].read_text().splitlines() if line.strip()]
    alive = []
    for pid in pids:
        # A live PID counts only if it still runs one of 0066's frozen binaries (PIDs are reused).
        args = subprocess.run(["ps", "-p", str(pid), "-o", "args="], capture_output=True, text=True).stdout
        if args.strip().startswith(str(c.PRIOR)):
            alive.append({"pid": pid, "args": args.strip()})
    record["release_body"] = release
    record["lease_release"] = {k: lease.get(k) for k in ("packet", "target_dir", "released_at",
                                                           "processes_on_target_at_release", "release_note")}
    record["prior_ledger_pids"] = len(pids)
    record["prior_ledger_pids_alive_now"] = alive
    ok = (all(record[n]["sha256"] == record[n]["expected"] for n in files)
          and release.get("packet") == grant.get("prior_packet")
          and release.get("owned_processes") == [] and release.get("ledger_pids_alive") == []
          and release.get("ledger_sha256") == record["ledger"]["sha256"]
          and lease.get("target_dir") == str(c.TARGET) and lease.get("released_at")
          and lease.get("processes_on_target_at_release") == [] and alive == []
          and grant.get("owned_prior_processes") == [])
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
        "packet": c.PACKET, "holder": "claude (Opus 5.5, Max thinking) for handoff 0067",
        "target_dir": str(c.TARGET), "assigned_by": "Astra via review-ready.json (0066 released it)",
        "grant_sha256": grant_sha, "grant": grant, "prior_receipts": prior, "source": source,
        "brief_sha256": c.sha(c.PACKET_DIR / "brief.md"), "worktree": str(c.ROOT),
        "acquired_at": c.now(), "processes_on_target_at_acquire": running,
        "rules": ["./tools/cargo only", "--locked", "--offline", "--profile ci", "at most -j2",
                  "explicit minimal child environment; receipts record only allowlisted settings",
                  "serial offscreen GPU children under an exclusive lock",
                  "no native window, desktop/browser capture, audio device or accounts",
                  "<= 48 new numerical frame attempts and <= 24 appearance renders, each <= 960x540",
                  "GPU suites <= 300 s, cases/captures <= 120 s, listings <= 60 s",
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
    lease["release_note"] = ("No 0067 cargo/rustc process remains on the target. Contents left in place as "
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
