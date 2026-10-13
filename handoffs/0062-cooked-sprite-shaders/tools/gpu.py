#!/usr/bin/env python3
"""Guarded serial GPU runner for handoff 0062. Standard library only.

Every child is checked against Astra's release marker (must exist and name
0062-cooked-sprite-shaders), this packet's release latch (must be absent), and
the frozen binary SHA-256 from build/<variant>.json, before and after the run.
Children run one at a time under an exclusive lock in their own process group;
numerical tests get 300 s and one test thread, captures 120 s. Any survivors in
the group are killed and every child is reaped. All runs go to gpu-ledger.jsonl.

  gpu.py list <variant> <lib|model_gpu>
  gpu.py test <variant> <lib|model_gpu> <label> [libtest filter/--skip args...]
  gpu.py capture <variant> <case> [--source-free]
  gpu.py release            # after the last child: verify none remain, write the latch
"""

import datetime
import fcntl
import hashlib
import json
import os
import signal
import subprocess
import sys
from pathlib import Path

EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0062")
MARKER = EVIDENCE / "gpu-review-ready"
LATCH = EVIDENCE / "gpu-released.json"
LEDGER = EVIDENCE / "gpu-ledger.jsonl"
LOCK = EVIDENCE / "gpu.lock"
RUN = EVIDENCE / "fixtures/r2"
PACKET = "0062-cooked-sprite-shaders"
BINARIES = {"lib": "incant_render-lib-tests", "model_gpu": "model_gpu", "frames": "native_frames"}
MAX_CAPTURES, MAX_W, MAX_H = 32, 960, 540


def sha(path):
    digest = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def gate():
    if LATCH.exists():
        raise SystemExit("GPU slot already released by this packet; refusing")
    if not MARKER.is_file():
        raise SystemExit("gpu-review-ready is absent; refusing")
    marker = json.loads(MARKER.read_text())
    if marker.get("packet") != PACKET:
        raise SystemExit("gpu-review-ready does not name this packet; refusing")
    return sha(MARKER)


def frozen(variant, kind):
    record = json.loads((EVIDENCE / "build" / f"{variant}.json").read_text())["frozen"][BINARIES[kind]]
    return Path(record["path"]), record["sha256"]


def ledger_entries():
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]


def clean_env():
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(("INCANT_", "RUST_", "WGPU_", "CARGO_", "MTL_", "METAL_"))}
    env["RUST_BACKTRACE"] = "1"
    return env


def run_child(label, kind, variant, binary, expected, argv, timeout, outputs=None):
    logs = EVIDENCE / "gpu-logs"
    logs.mkdir(exist_ok=True)
    with open(LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        gate_sha = gate()
        if sha(binary) != expected:
            raise SystemExit(f"{binary} does not match its frozen hash")
        stdout_path, stderr_path = logs / f"{label}.stdout.txt", logs / f"{label}.stderr.txt"
        if stdout_path.exists() or stderr_path.exists():
            raise SystemExit(f"label {label} already used")
        started = now()
        with open(stdout_path, "x") as out, open(stderr_path, "x") as err:
            child = subprocess.Popen(argv, stdout=out, stderr=err, env=clean_env(), cwd=EVIDENCE,
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
        ended = now()
        after = sha(binary)
        entry = {"label": label, "kind": kind, "variant": variant, "argv": [str(a) for a in argv],
                 "binary_sha256_before": expected, "binary_sha256_after": after, "gate_sha256": gate_sha,
                 "pid": child.pid, "started": started, "ended": ended, "exit": code, "timed_out": timed_out,
                 "timeout_s": timeout, "stdout_sha256": sha(stdout_path), "stderr_sha256": sha(stderr_path)}
        if outputs is not None:
            entry.update(outputs())
        with open(LEDGER, "a") as ledger:
            ledger.write(json.dumps(entry) + "\n")
    print(json.dumps({k: entry[k] for k in ("label", "pid", "exit", "timed_out", "started", "ended")}))
    if after != expected:
        raise SystemExit("binary changed during the run")
    return entry


def tests(variant, which, label, extra, listing=False):
    binary, expected = frozen(variant, which)
    if listing:
        argv = [str(binary), "--list", "--ignored"]
        return run_child(f"{variant}-{which}-list", "list", variant, binary, expected, argv, 60)
    if any(a.startswith("--test-threads") or a in ("--include-ignored",) for a in extra):
        raise SystemExit("thread count and ignore mode are fixed by this runner")
    argv = [str(binary), "--ignored", "--test-threads=1", "--nocapture", *extra]
    return run_child(f"{variant}-{which}-{label}", "test", variant, binary, expected, argv, 300)


def capture(variant, case, source_free):
    build = json.loads((RUN / "build.json").read_text())
    spec = build["cases"][case]
    key = spec["board"]
    request_path = RUN / "requests" / f"{case}.json"
    if sha(request_path) != spec["request_sha256"]:
        raise SystemExit("request file changed since authoring")
    request = json.loads(request_path.read_text())
    if request["width"] > MAX_W or request["height"] > MAX_H:
        raise SystemExit("capture exceeds 960x540")
    used = sum(e.get("captures_reserved", 0) for e in ledger_entries())
    if used + len(request["ticks"]) > MAX_CAPTURES:
        raise SystemExit(f"capture budget: {used} used, {len(request['ticks'])} requested, {MAX_CAPTURES} max")
    marker = RUN / f"source-free-{key}.json"
    if source_free != marker.exists():
        raise SystemExit("source-free mode must match whether this board's sources were deleted")
    project_dir = RUN / key
    models = project_dir / ".incant/cache/models"
    if not models.is_dir():
        models = RUN / "empty-models"
    textures = project_dir / ".incant/cache/textures"
    scene = RUN / "cooked" / f"{key}.scene"
    if sha(scene) != build["boards"][key]["cooked"]["sha256"]:
        raise SystemExit("cooked scene changed since authoring")
    suffix = "-source-free" if source_free else ""
    output = EVIDENCE / "captures" / variant / f"{case}{suffix}"
    if output.exists():
        raise SystemExit("capture output exists; captures are never overwritten")
    output.parent.mkdir(parents=True, exist_ok=True)
    binary, expected = frozen(variant, "frames")
    argv = [str(binary), str(scene), str(models), str(textures), spec["camera_hex"], str(request_path),
            str(output)]
    inputs = {"scene_sha256": sha(scene), "request_sha256": sha(request_path),
              "textures": {p.name: sha(p) for p in sorted(textures.iterdir())},
              "models": {p.name: sha(p) for p in sorted(models.iterdir())},
              "project_dir_files": sorted(str(p.relative_to(project_dir)) for p in project_dir.rglob("*")
                                          if p.is_file())}

    def outputs():
        pngs = sorted(output.glob("*.png")) if output.exists() else []
        report = output / "report.json"
        return {"case": case, "source_free": source_free, "captures_reserved": len(request["ticks"]),
                "inputs": inputs, "pngs": {p.name: sha(p) for p in pngs},
                "report_sha256": sha(report) if report.exists() else None}

    return run_child(f"{variant}-{case}{suffix}", "capture", variant, binary, expected, argv, 120, outputs)


def release():
    with open(LOCK, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if LATCH.exists():
            raise SystemExit("already released")
        listing = subprocess.run(["ps", "-axo", "pid=,command="], capture_output=True, text=True).stdout
        owned = [line.strip() for line in listing.splitlines() if str(EVIDENCE / "bin") in line]
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
        record = {"packet": PACKET, "released_at": now(), "children": len(entries),
                  "captures": sum(e.get("captures_reserved", 0) for e in entries),
                  "first_start": entries[0]["started"] if entries else None,
                  "last_end": entries[-1]["ended"] if entries else None,
                  "ledger_sha256": sha(LEDGER) if LEDGER.exists() else None,
                  "marker_sha256_at_release": sha(MARKER) if MARKER.exists() else None,
                  "owned_processes": owned, "ledger_pids_alive": alive,
                  "note": "Release by 0062 for Astra; gpu-review-ready left untouched (only Astra writes it)."}
        if alive:
            raise SystemExit(f"ledger PIDs still alive: {alive}")
        with open(LATCH, "x") as out:
            json.dump(record, out, indent=1)
            out.write("\n")
    print(json.dumps(record, indent=1))


def main():
    args = sys.argv[1:]
    if len(args) == 3 and args[0] == "list":
        tests(args[1], args[2], "list", [], listing=True)
    elif len(args) >= 4 and args[0] == "test" and args[2] in ("lib", "model_gpu"):
        tests(args[1], args[2], args[3], args[4:])
    elif len(args) in (3, 4) and args[0] == "capture":
        capture(args[1], args[2], args[3:] == ["--source-free"])
    elif args == ["release"]:
        release()
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
