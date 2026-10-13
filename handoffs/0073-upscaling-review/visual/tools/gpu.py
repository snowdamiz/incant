#!/usr/bin/env python3
"""Guarded serial GPU runner for the 0073 upscaling visual stage. Standard library only.

Adapted from handoff 0068's committed, root-reviewed tools/gpu.py (read-only
reuse), with a pre-launch debit. For every child it takes the packet flock
(blocking) and then the shared native-gpu.lock (non-blocking; a busy lock refuses
before anything is charged). Holding both, it checks in this order: root's grant,
the release latch, the stop latch, the target lease, the committed visual tree,
every child-checkout file against its git blob, and the frozen binary (SHA-256
and execute bit). It then checks the budgets (<= 96 frames including failures,
<= 16 PNGs) against debits.jsonl, appends this child's full reserve there and
fsyncs it, and only then launches. Debits are never refunded.

Each child runs in its own process group with a constructed environment (HOME
passed through unrecorded; PATH, TMPDIR, RUST_BACKTRACE, the expected backend
and the review output directory as recorded overrides) and an empty signal mask.
INT, TERM and HUP stay blocked in the runner from the debit until the attempt is
recorded; it polls the child and the pending-signal set, so neither a timeout nor
an interrupt can skip the SIGKILL of the whole group, the reap or the ledger row.
A pending interrupt is delivered after recording. Every child goes to
gpu-ledger.jsonl, every refusal (once the grant exists) to gpu-refusals.jsonl,
and any failed child latches stop.json. Each case runs once; there are no
reruns in this stage.

  gpu.py list                    list the helper's ignored cases (no frames)
  gpu.py review <case>           probe, stationary, motion, transitions, in plan order
  gpu.py release                 after the last child: verify none remain, write the latch
"""

import fcntl
import json
import os
import re
import struct
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

LISTING = re.compile(r"^(.+): test$", re.M)
RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
                    r"(\d+) measured; (\d+) filtered out", re.M)


def ledger():
    return c.read_lines(c.LEDGER)


def debits():
    return c.read_lines(c.DEBITS)


def runs(kind, item):
    return [e for e in ledger() if e["kind"] == kind and e.get("item") == item]


def frozen():
    record = c.BUILD / "review.json"
    if not record.exists():
        c.refuse("no frozen review binary; run build.py review")
    entry = c.read_json(record)["frozen"]
    return Path(entry["path"]), entry["sha256"]


def failures(stdout):
    """Names in libtest's closing failures: list (FAILED can sit on its own line
    under --nocapture, so per-line status parsing is not trusted)."""
    if "\nfailures:\n" not in stdout:
        return []
    names = []
    for line in stdout.rsplit("\nfailures:\n", 1)[1].splitlines():
        if not line.strip():
            if names:
                break
            continue
        if not line.startswith("    "):
            break
        names.append(line.strip())
    return names


def libtest_summary(stdout):
    return {"results": [{"status": r[0], "passed": int(r[1]), "failed": int(r[2]),
                         "ignored": int(r[3]), "measured": int(r[4]), "filtered_out": int(r[5])}
                        for r in RESULT.findall(stdout)],
            "failures": failures(stdout)}


def latch_stop(reason, record=None):
    if not c.STOP.exists():
        c.write_new_json(c.STOP, {"at": c.now(), "reason": reason, "record": record})


def run_child(label, kind, item, args, timeout, frames=0, pngs=0, extra=None, after=None):
    """Debit, launch, wait, kill/reap and record one child under both locks.
    Interrupts stay pending from the debit until the attempt is recorded."""
    p = c.plan()
    c.LOGS.mkdir(parents=True, exist_ok=True)
    with open(c.LOCK, "a") as lock, open(c.GLOBAL_LOCK, "a") as global_lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        try:
            fcntl.flock(global_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            c.refuse("the shared native-gpu.lock is held by another runner; nothing was charged")
        _, grant_sha = c.gate()
        tools = c.tools_identity()
        source = c.verify_checkout()
        binary, expected = frozen()
        if c.sha(binary) != expected or not os.access(binary, os.X_OK):
            c.refuse(f"{binary} does not match its frozen hash or is not executable")
        charged = debits()
        used_frames = sum(d["frames"] for d in charged)
        used_pngs = sum(d["pngs"] for d in charged)
        budgets = p["budgets"]
        if used_frames + frames > budgets["frames_max"]:
            c.refuse(f"frame budget: {used_frames} debited, {frames} requested")
        if used_pngs + pngs > budgets["pngs_max"]:
            c.refuse(f"PNG budget: {used_pngs} debited, {pngs} requested")
        stdout_path = c.LOGS / f"gpu-{label}.stdout.txt"
        stderr_path = c.LOGS / f"gpu-{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            c.refuse(f"label {label} already used")
        overrides = {"PATH": c.SYSTEM_PATH, "TMPDIR": str(c.fresh_dir(c.SCRATCH / label)),
                     "RUST_BACKTRACE": "1", **p["review"]["backend_env"], **(extra or {})}
        argv = [str(binary), *args]
        cwd = c.CHECKOUT / p["review"]["cwd"]
        with c.blocked():
            c.append_synced(c.DEBITS, {"label": label, "kind": kind, "item": item,
                                       "frames": frames, "pngs": pngs,
                                       "binary_sha256": expected, "at": c.now()})
            started = c.now()
            with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
                result = c.launch_recorded(argv, cwd, c.child_env(overrides), out, err, timeout)
            binary_after = c.sha(binary)
            stdout = stdout_path.read_text(errors="replace")
            entry = {"label": label, "kind": kind, "item": item, "argv": argv, "cwd": str(cwd),
                     "env": c.env_receipt(overrides), "binary_sha256_before": expected,
                     "binary_sha256_after": binary_after, "grant_sha256": grant_sha,
                     "tools": tools, "source": source, "started": started, "ended": c.now(),
                     **result, "timeout_s": timeout, "frames_debited": frames,
                     "pngs_debited": pngs,
                     "scratch_leftovers": sorted(os.listdir(c.SCRATCH / label)),
                     "stdout": str(stdout_path), "stdout_sha256": c.sha(stdout_path),
                     "stderr": str(stderr_path), "stderr_sha256": c.sha(stderr_path),
                     "libtest": libtest_summary(stdout)}
            if after is not None:
                try:
                    entry.update(after(stdout))
                except Exception as error:  # parsing must not lose the attempt
                    entry["after_error"] = repr(error)
            entry["ok"] = (result["exit"] == 0 and not result["timed_out"]
                           and not result["interrupted_by"] and result["group_empty_after"]
                           and "launch_error" not in result and binary_after == expected
                           and entry.get("outputs_ok", True) and "after_error" not in entry)
            c.append_synced(c.LEDGER, entry)
            if not entry["ok"]:
                latch_stop(f"child {label} failed", {"exit": result["exit"],
                                                      "timed_out": result["timed_out"],
                                                      "interrupted_by": result["interrupted_by"]})
    # Any pending interrupt was delivered when the mask was restored, after recording.
    if result["interrupted_by"]:
        raise KeyboardInterrupt("interrupted by " + ", ".join(result["interrupted_by"]))
    print(json.dumps({k: entry.get(k) for k in ("label", "pid", "exit", "timed_out", "ok",
                                                 "started", "ended", "libtest")}, indent=1))
    if not result["group_empty_after"]:
        raise SystemExit("owned GPU process group remains after cleanup; see ledger")
    if not entry["ok"]:
        raise SystemExit(f"{label} failed; stop latched")
    return entry


def listing():
    p = c.plan()
    if runs("list", "review"):
        c.refuse("review cases already listed")

    def names(stdout):
        found = LISTING.findall(stdout)
        expected = sorted(case["test"] for case in p["review"]["cases"].values())
        return {"listed": found, "outputs_ok": sorted(found) == expected}

    return run_child("list-review", "list", "review", ["--list", "--ignored"],
                     p["budgets"]["listing_timeout_s"], after=names)


def png_size(path):
    with open(path, "rb") as source:
        head = source.read(24)
    if head[:8] != b"\x89PNG\r\n\x1a\n" or head[12:16] != b"IHDR":
        return None
    return list(struct.unpack(">II", head[16:24]))


def review(case):
    p = c.plan()
    spec = p["review"]
    if case not in spec["cases"]:
        c.refuse(f"unknown review case {case}")
    details = spec["cases"][case]
    if not any(e.get("ok") for e in runs("list", "review")):
        c.refuse("gpu.py list first")
    position = spec["order"].index(case)
    for earlier in spec["order"][:position]:
        if not any(e.get("ok") for e in runs("review", earlier)):
            c.refuse(f"{earlier} has no passing run; cases run once each in plan order")
    previous = runs("review", case) + [d for d in debits() if d["item"] == case]
    if previous:
        c.refuse(f"{case} was already attempted; there are no reruns in this stage")
    output = c.CAPTURES / f"{case}-r1"
    if output.exists():
        c.refuse(f"{output} exists; captures are never overwritten")
    c.CAPTURES.mkdir(parents=True, exist_ok=True)

    def outputs(_stdout):
        report = output / "report.json"
        body = c.read_json(report) if report.exists() else {}
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        sizes = {path.name: png_size(path) for path in pngs}
        names_ok = sorted(sizes) == sorted(details["pngs"])
        sizes_ok = all(size == p["budgets"]["png_size"] for size in sizes.values())
        frames_ok = body.get("frames_charged") == details["frames"]
        return {"case": case, "output": str(output),
                "pngs": {path.name: c.sha(path) for path in pngs}, "png_sizes": sizes,
                "report_sha256": c.sha(report) if report.exists() else None,
                "events_sha256": c.sha(output / "events.jsonl") if (output / "events.jsonl").exists() else None,
                "outcome": body.get("outcome"), "checks": body.get("checks"),
                "frames_charged": body.get("frames_charged"),
                "outputs_ok": body.get("outcome") == "pass" and names_ok and sizes_ok and frames_ok}

    return run_child(f"review-{case}-r1", "review", case,
                     [details["test"], *spec["args"]], p["budgets"]["case_timeout_s"],
                     frames=details["frames"], pngs=len(details["pngs"]),
                     extra={spec["output_env"]: str(output)}, after=outputs)


def owned_processes(target):
    """Running processes whose executable is one of ours: a frozen binary, the
    child checkout's cargo wrapper, or anything run from the target."""
    listing_text = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
    owned = []
    for line in listing_text.splitlines():
        parts = line.split(None, 1)
        if len(parts) == 2 and parts[1].startswith((str(c.BIN), str(c.CHECKOUT), str(target))):
            owned.append(line.strip())
    return owned


def release():
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        data, _ = c.grant()
        if c.LATCH.exists():
            raise SystemExit("already released")
        target = Path(data["target"])
        owned = owned_processes(target)
        if owned:
            c.refuse("owned processes still running: " + json.dumps(owned)[:400])
        entries = ledger()
        alive = []
        for entry in entries:
            if entry.get("pid") is None:
                continue  # an exec failure left no process
            found = subprocess.run(["ps", "-p", str(entry["pid"]), "-o", "args="],
                                   capture_output=True, text=True).stdout.strip()
            if found.startswith((str(c.BIN), str(c.CHECKOUT), str(target))):
                alive.append(entry["pid"])
        if alive:
            c.refuse(f"ledger PIDs still alive: {alive}")
        charged = debits()
        recorded = {e["label"] for e in entries}
        refusals = c.read_lines(c.REFUSALS)
        record = {"packet": c.PACKET, "stage": c.STAGE, "released_at": c.now(),
                  "children": len(entries),
                  "gpu_children": len([e for e in entries if e["kind"] != "build"]),
                  "debits": len(charged),
                  "debits_without_ledger_row": [d["label"] for d in charged
                                                if d["label"] not in recorded],
                  "frames_debited": sum(d["frames"] for d in charged),
                  "pngs_debited": sum(d["pngs"] for d in charged),
                  "frames_charged_by_helper": sum(e.get("frames_charged") or 0 for e in entries),
                  "pngs_written": sum(len(e.get("pngs", {})) for e in entries),
                  "stop_latched": c.STOP.exists(), "refusals": len(refusals),
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "debits_sha256": c.sha(c.DEBITS) if c.DEBITS.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT),
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "GPU released by 0073 for root; review-ready.json left untouched "
                          "(only root writes it). Run lease.py release next."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    c.install_interrupts()
    c.grant()
    args = sys.argv[1:]
    if args == ["list"]:
        c.gate()
        listing()
    elif len(args) == 2 and args[0] == "review":
        c.gate()
        review(args[1])
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
