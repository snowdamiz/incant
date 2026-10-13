#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0063. Standard library only.

Every child is checked against Astra's grant (review-ready.json must exist and
name 0063-cooked-ui-text-shaders), this packet's release latch (must be absent)
and the frozen binary SHA-256 from build/<variant>.json, before and after the
run. Children run one at a time under an exclusive lock, each in its own
process group with a clean environment: numerical tests get 300 s and one test
thread, captures 120 s. Survivors in the group are killed and every child is
reaped. All runs go to gpu-ledger.jsonl. At most 24 PNG frames, each at most
960x540, are ever written (reserved before launch, counted after).

  gpu.py list <variant> <suite>
  gpu.py test <variant> <suite> <label> [libtest filter/--skip args...]
  gpu.py capture <variant> <case> [--source-free]
  gpu.py release            # after the last child: verify none remain, write the latch

Suites: lib (incant_render library tests), model_gpu, ui and game (incant_render_gpu_host).
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

LEDGER = c.EVIDENCE / "gpu-ledger.jsonl"
LOCK = c.EVIDENCE / "gpu.lock"
RUN = c.EVIDENCE / "fixtures/r3"
SUITES = {"lib": "incant_render-lib-tests", "model_gpu": "model_gpu", "ui": "gpu_host-ui",
          "game": "gpu_host-game", "review": "ui_text_review"}


def gate():
    if c.LATCH.exists():
        raise SystemExit("GPU slot already released by this packet; refusing")
    return c.grant()


def frozen(variant, kind):
    record = c.read_json(c.EVIDENCE / "build" / f"{variant}.json")["frozen"][SUITES[kind]]
    return Path(record["path"]), record["sha256"]


def ledger_entries():
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]


def frames_used():
    """Frames reserved by earlier children, or actually written if more."""
    return sum(max(e.get("frames_reserved", 0), len(e.get("pngs", {}))) for e in ledger_entries())


def run_child(label, kind, variant, binary, expected, argv, timeout, outputs=None, reserve=0):
    logs = c.EVIDENCE / "gpu-logs"
    logs.mkdir(exist_ok=True)
    with open(LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        gate_sha = gate()
        if c.sha(binary) != expected:
            raise SystemExit(f"{binary} does not match its frozen hash")
        if reserve and frames_used() + reserve > c.MAX_FRAMES:
            raise SystemExit(f"frame budget: {frames_used()} used, {reserve} requested, {c.MAX_FRAMES} max")
        stdout_path, stderr_path = logs / f"{label}.stdout.txt", logs / f"{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            raise SystemExit(f"label {label} already used")
        started = c.now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            child = subprocess.Popen(argv, stdout=out, stderr=err, env=c.clean_env({"RUST_BACKTRACE": "1"}),
                                     cwd=c.EVIDENCE, start_new_session=True)
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
                 "binary_sha256_before": expected, "binary_sha256_after": after, "grant_sha256": gate_sha,
                 "pid": child.pid, "started": started, "ended": ended, "exit": code, "timed_out": timed_out,
                 "timeout_s": timeout, "stdout_sha256": c.sha(stdout_path), "stderr_sha256": c.sha(stderr_path),
                 "frames_reserved": reserve}
        if outputs is not None:
            entry.update(outputs())
        with open(LEDGER, "a") as ledger:
            ledger.write(json.dumps(entry, ensure_ascii=False) + "\n")
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "started", "ended")}))
    if after != expected:
        raise SystemExit("binary changed during the run")
    return entry


def tests(variant, suite, label, extra, listing=False):
    binary, expected = frozen(variant, suite)
    if listing:
        return run_child(f"{variant}-{suite}-list", "list", variant, binary, expected,
                         [str(binary), "--list", "--ignored"], 60)
    if any(a.startswith("--test-threads") or a in ("--include-ignored", "--ignored") for a in extra):
        raise SystemExit("thread count and ignore mode are fixed by this runner")
    argv = [str(binary), "--ignored", "--test-threads=1", "--nocapture", *extra]
    return run_child(f"{variant}-{suite}-{label}", "test", variant, binary, expected, argv, 300)


def capture(variant, case, source_free):
    build = c.read_json(RUN / "build.json")
    extra = RUN / "cases-extra" / f"{case}.json"
    spec = build["cases"][case] if case in build["cases"] else c.read_json(extra)
    board = spec["board"]
    request_path = RUN / "requests" / f"{case}.json"
    if c.sha(request_path) != spec["request_sha256"]:
        raise SystemExit("request file changed since authoring")
    request = c.read_json(request_path)
    if request["size"][0] > c.MAX_W or request["size"][1] > c.MAX_H:
        raise SystemExit("capture exceeds 960x540")
    marker = RUN / f"source-free-{board}.json"
    if source_free != marker.exists():
        raise SystemExit("source-free mode must match whether this board's sources were deleted")
    project_dir = RUN / board
    cache = project_dir / ".incant/cache"
    caches = {name: (cache / name if (cache / name).is_dir() else RUN / "empty")
              for name in ("models", "textures", "fonts")}
    scene = RUN / "cooked" / f"{board}.scene"
    if c.sha(scene) != build["boards"][board]["cooked"]["sha256"]:
        raise SystemExit("cooked scene changed since authoring")
    suffix = "-source-free" if source_free else ""
    output = c.EVIDENCE / "captures" / variant / f"{case}{suffix}"
    if output.exists():
        raise SystemExit("capture output exists; captures are never overwritten")
    output.parent.mkdir(parents=True, exist_ok=True)
    binary, expected = frozen(variant, "review")
    argv = [str(binary), str(scene), str(caches["models"]), str(caches["textures"]), str(caches["fonts"]),
            str(request_path), str(output)]
    inputs = {"scene_sha256": c.sha(scene), "request_sha256": c.sha(request_path),
              "caches": {name: {p.name: c.sha(p) for p in sorted(path.iterdir())} for name, path in caches.items()},
              "project_dir_files": sorted(str(p.relative_to(project_dir)) for p in project_dir.rglob("*")
                                          if p.is_file())}

    def outputs():
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        report = output / "report.json"
        return {"case": case, "source_free": source_free, "inputs": inputs,
                "pngs": {p.name: c.sha(p) for p in pngs},
                "report_sha256": c.sha(report) if report.exists() else None}

    reserve = len(spec["captures"])
    return run_child(f"{variant}-{case}{suffix}-{RUN.name}", "capture", variant, binary, expected, argv, 120,
                     outputs, reserve)


def release():
    with open(LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if c.LATCH.exists():
            raise SystemExit("already released")
        listing = subprocess.run(["ps", "-axo", "pid=,command="], capture_output=True, text=True).stdout
        owned = [line.strip() for line in listing.splitlines() if str(c.EVIDENCE / "bin") in line]
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
                  "first_start": entries[0]["started"] if entries else None,
                  "last_end": entries[-1]["ended"] if entries else None,
                  "ledger_sha256": c.sha(LEDGER) if LEDGER.exists() else None,
                  "grant_sha256_at_release": c.sha(c.GRANT) if c.GRANT.exists() else None,
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "Release by 0063 for Astra; review-ready.json left untouched (only Astra writes it)."}
        c.write_new_json(c.LATCH, record)
    print(json.dumps(record, indent=1))


def main():
    args = sys.argv[1:]
    if len(args) == 3 and args[0] == "list" and args[2] in SUITES and args[2] != "review":
        tests(args[1], args[2], "list", [], listing=True)
    elif len(args) >= 4 and args[0] == "test" and args[2] in SUITES and args[2] != "review":
        tests(args[1], args[2], args[3], args[4:])
    elif len(args) in (3, 4) and args[0] == "capture" and args[3:] in ([], ["--source-free"]):
        capture(args[1], args[2], args[3:] == ["--source-free"])
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
