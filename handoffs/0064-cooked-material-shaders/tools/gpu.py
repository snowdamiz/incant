#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0064. Standard library only.

Adapted from handoff 0062's committed tools/gpu.py (read-only reuse). Before
every child it checks, under an exclusive flock: Astra's review-ready.json
grant names this packet and target; this packet's release latch is absent; the
frozen binary SHA-256 matches build/{tests,review}-<variant>.json (also after
the run). Children run one at a time in their own process group with a clean
environment (no INCANT_/RUST_/WGPU_/CARGO_/MTL_/METAL_ variables): 300 s for a
test suite, 120 s for a review capture, 60 s for a listing. Survivors in the
group are killed and every child is reaped. PNG frames are reserved from the
plan before the run and capped at 24 in total. Everything goes to the ledger.

  gpu.py list <variant> <suite>
  gpu.py test <variant> <suite> <label> [libtest filter/--skip args...]
  gpu.py review <variant> <plan>          # plan: main-dry | main | source-free | shadow-dry | shadow
  gpu.py release                          # after the last child: verify none remain, write the latch

Suites: lib (incant_render library tests), model_gpu, gpu_host-game, gpu_host-ui.
"""

import fcntl
import json
import os
import signal
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

RUN = c.EVIDENCE / "fixtures/r1"
RUNS = {"main-dry": RUN, "main": RUN, "source-free": RUN, "shadow-dry": c.EVIDENCE / "fixtures/r2",
        "shadow": c.EVIDENCE / "fixtures/r2"}
SUITES = {"lib": "incant_render-lib-tests", "model_gpu": "model_gpu", "gpu_host-game": "gpu_host-game",
          "gpu_host-ui": "gpu_host-ui"}


def gate():
    if c.LATCH.exists():
        raise SystemExit("GPU slot already released by this packet; refusing")
    return c.check_grant()


def frozen(variant, name):
    for step in ("tests", "review"):
        path = c.EVIDENCE / "build" / f"{step}-{variant}.json"
        if path.exists():
            entry = c.read_json(path)["frozen"].get(name)
            if entry:
                return Path(entry["path"]), entry["sha256"]
    raise SystemExit(f"no frozen {variant}/{name}")


def ledger_entries():
    if not c.LEDGER.exists():
        return []
    return [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]


def run_child(label, kind, variant, binary, expected, argv, timeout, frames=0, outputs=None):
    logs = c.EVIDENCE / "gpu-logs"
    logs.mkdir(exist_ok=True)
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        grant_sha = gate()
        if c.sha(binary) != expected:
            raise SystemExit(f"{binary} does not match its frozen hash")
        used = sum(e.get("frames_reserved", 0) for e in ledger_entries())
        if used + frames > c.MAX_FRAMES:
            raise SystemExit(f"frame budget: {used} used, {frames} requested, {c.MAX_FRAMES} max")
        stdout_path, stderr_path = logs / f"{label}.stdout.txt", logs / f"{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            raise SystemExit(f"label {label} already used")
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            child = subprocess.Popen(argv, stdout=out, stderr=err, env=c.run_env(), cwd=c.EVIDENCE,
                                     start_new_session=True)
            timed_out = False
            try:
                code = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                os.killpg(child.pid, signal.SIGKILL)
                code = child.wait()
            finally:
                try:
                    os.killpg(child.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        ended = c.now()
        after = c.sha(binary)
        entry = {"label": label, "kind": kind, "variant": variant, "argv": [str(a) for a in argv],
                 "binary_sha256_before": expected, "binary_sha256_after": after, "grant_sha256": grant_sha,
                 "pid": child.pid, "started": started, "ended": ended, "exit": code, "timed_out": timed_out,
                 "timeout_s": timeout, "frames_reserved": frames, "stdout_sha256": c.sha(stdout_path),
                 "stderr_sha256": c.sha(stderr_path)}
        if outputs is not None:
            entry.update(outputs())
        with open(c.LEDGER, "a") as ledger:
            ledger.write(json.dumps(entry) + "\n")
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "started", "ended")}))
    if after != expected:
        raise SystemExit("binary changed during the run")
    return entry


def tests(variant, suite, label, extra, listing=False):
    binary, expected = frozen(variant, SUITES[suite])
    if listing:
        argv = [str(binary), "--list", "--ignored"]
        return run_child(f"{variant}-{suite}-list", "list", variant, binary, expected, argv, 60)
    if any(a.startswith("--test-threads") or a in ("--include-ignored", "--ignored") for a in extra):
        raise SystemExit("thread count and ignore mode are fixed by this runner")
    argv = [str(binary), "--ignored", "--test-threads=1", "--nocapture", *extra]
    return run_child(f"{variant}-{suite}-{label}", "test", variant, binary, expected, argv, 300)


def review(variant, plan_name):
    index = c.read_json(RUNS[plan_name] / "plans/index.json")[plan_name]
    plan_path = Path(index["path"])
    if c.sha(plan_path) != index["sha256"]:
        raise SystemExit("plan changed since it was written")
    plan = c.read_json(plan_path)
    if plan_name == "source-free":
        for key in ("M", "L", "Z1", "R"):
            if not (RUN / f"source-free-{key}.json").exists():
                raise SystemExit(f"source-free plan needs author.py source-free for board {key} first")
    inputs = {}
    for step in plan["steps"]:
        for name in ("scene",):
            inputs[step[name]] = c.sha(step[name])
        for directory in (step["models"], step["textures"]):
            for path in sorted(Path(directory).iterdir()):
                inputs[str(path)] = c.sha(path)
    output = c.EVIDENCE / "captures" / variant / plan_name
    if output.exists():
        raise SystemExit("capture output exists; captures are never overwritten")
    output.parent.mkdir(parents=True, exist_ok=True)
    binary, expected = frozen(variant, "material_review")
    argv = [str(binary), str(plan_path), str(output)]

    def outputs():
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        report = output / "report.json"
        return {"plan": plan_name, "plan_sha256": index["sha256"], "inputs": inputs,
                "pngs": {p.name: c.sha(p) for p in pngs},
                "report_sha256": c.sha(report) if report.exists() else None}

    return run_child(f"{variant}-review-{plan_name}", "review", variant, binary, expected, argv, 120,
                     frames=len(index["pngs"]), outputs=outputs)


def release():
    with open(c.LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if c.LATCH.exists():
            raise SystemExit("already released")
        listing = subprocess.run(["ps", "-axo", "pid=,args="], capture_output=True, text=True).stdout
        # Owned children are frozen executables themselves (argv[0] under bin/), never a shell
        # whose command line merely mentions the path (the first release attempt self-matched).
        owned = [line.strip() for line in listing.splitlines()
                 if len(line.split(None, 1)) == 2 and line.split(None, 1)[1].startswith(str(c.EVIDENCE / "bin"))]
        if owned:
            raise SystemExit(f"owned processes still running: {owned}")
        entries = ledger_entries()
        alive = []
        for entry in entries:
            try:
                os.kill(entry["pid"], 0)
                alive.append(entry["pid"])
            except ProcessLookupError:
                pass
        if alive:
            raise SystemExit(f"ledger PIDs still alive: {alive}")
        record = {"packet": c.PACKET, "released_at": c.now(), "children": len(entries),
                  "frames_written": sum(len(e.get("pngs", {})) for e in entries),
                  "frames_reserved": sum(e.get("frames_reserved", 0) for e in entries),
                  "first_start": entries[0]["started"] if entries else None,
                  "last_end": entries[-1]["ended"] if entries else None,
                  "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT) if c.GRANT.exists() else None,
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "Release by 0064 for Astra; review-ready.json left untouched (only Astra writes it)."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    args = sys.argv[1:]
    if len(args) == 3 and args[0] == "list" and args[2] in SUITES:
        tests(args[1], args[2], "list", [], listing=True)
    elif len(args) >= 4 and args[0] == "test" and args[2] in SUITES:
        tests(args[1], args[2], args[3], args[4:])
    elif len(args) == 3 and args[0] == "review" and args[2] in RUNS:
        review(args[1], args[2])
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
