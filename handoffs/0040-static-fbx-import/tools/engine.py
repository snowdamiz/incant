"""Bounded, hash-checked calls into the frozen incant_headless (handoff 0040).

Every call verifies the binary's SHA-256 against artifacts/tools/binary.json before
and after it runs, uses a hard timeout, reaps the child on timeout and appends one
record to a cumulative ledger. Screenshots additionally enforce the packet budget
(96 captures, 960x540 each, 256 MiB raw in total) across every run of these tools.
Only public CLI entry points are used: init, import, rpc, screenshot and validate.
"""
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

from pngio import dimensions

ROOT = Path(__file__).resolve().parents[3]
BINARY = ROOT / "artifacts/tools/incant_headless"
BINARY_JSON = ROOT / "artifacts/tools/binary.json"
ART = ROOT / "artifacts/0040-static-fbx-import"
LEDGER = ART / "ledger.jsonl"
MAX_CAPTURES, MAX_WIDTH, MAX_HEIGHT, MAX_BYTES = 96, 960, 540, 256 * 1024 * 1024


class EngineError(RuntimeError):
    pass


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def expected_hash():
    return json.loads(BINARY_JSON.read_text())["sha256"]


def check_binary():
    actual = sha256(BINARY)
    if actual != expected_hash():
        raise EngineError(f"frozen binary hash mismatch: {actual}")
    return actual


def ledger():
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text().splitlines() if line.strip()]


def _append(record):
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    with open(LEDGER, "a") as f:
        f.write(json.dumps(record, sort_keys=True) + "\n")


def project_state(project):
    project = Path(project)
    journal = project.with_suffix(".journal.jsonl")
    state = {"project_sha256": sha256(project) if project.exists() else None,
             "journal_sha256": sha256(journal) if journal.exists() else None}
    if project.exists():
        doc = json.loads(project.read_text())
        state["assets"] = {k: v.get("sha256") for k, v in sorted(doc.get("assets", {}).items())}
    return state


def call(args, *, label, timeout=120, stdin=None, run_id=None, extra=None):
    """Run the frozen binary once. Returns (exit_code, stdout, stderr)."""
    before = check_binary()
    started = time.time()
    proc = subprocess.Popen([str(BINARY), *map(str, args)], stdin=subprocess.PIPE if stdin is not None else subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=str(ART), text=True)
    timed_out = False
    try:
        out, err = proc.communicate(stdin, timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        proc.kill()
        out, err = proc.communicate()  # reap
    elapsed = time.time() - started
    after = check_binary()
    record = {"seq": len(ledger()) + 1, "run": run_id, "label": label, "pid": proc.pid,
              "args": [os.path.relpath(a, ROOT) if isinstance(a, Path) else str(a) for a in args],
              "binary_sha256_before": before, "binary_sha256_after": after, "exit": proc.returncode,
              "timed_out": timed_out, "timeout_s": timeout, "elapsed_s": round(elapsed, 3),
              "stderr": err[-4000:], "stdout_sha256": hashlib.sha256(out.encode()).hexdigest()}
    if extra:
        record.update(extra)
    _append(record)
    if timed_out:
        raise EngineError(f"{label}: timed out after {timeout}s (pid {proc.pid} killed and reaped)")
    return proc.returncode, out, err


def capture_totals():
    shots = [r for r in ledger() if r.get("kind") == "screenshot" and r.get("output_bytes")]
    return len(shots), sum(r["output_bytes"] for r in shots)


def screenshot(project, output, *, label, camera=None, width=960, height=540, run_id=None, timeout=180):
    if width > MAX_WIDTH or height > MAX_HEIGHT:
        raise EngineError("capture exceeds 960x540")
    count, total = capture_totals()
    if count >= MAX_CAPTURES:
        raise EngineError("capture budget exhausted (96)")
    # Worst case for one uncompressed RGBA frame plus PNG overhead.
    if total + width * height * 4 + 65536 > MAX_BYTES:
        raise EngineError("raw capture budget (256 MiB) would be exceeded")
    output = Path(output)
    if output.exists():
        raise EngineError(f"refusing to overwrite {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    args = ["screenshot", Path(project), output, "--width", width, "--height", height]
    if camera:
        args += ["--camera", camera]
    state = project_state(project)
    extra = {"kind": "screenshot", "project_state": state, "camera": camera, "width": width, "height": height}
    code, out, err = call(args, label=label, timeout=timeout, run_id=run_id, extra=extra)
    if code != 0:
        return {"ok": False, "exit": code, "stderr": err.strip()}
    data = output.read_bytes()
    w, h = dimensions(data)
    reply = json.loads(out)
    if (w, h) != (width, height):
        raise EngineError(f"{label}: wrote {w}x{h}, asked {width}x{height}")
    # Amend the ledger record with the output identity.
    records = ledger()
    records[-1].update({"output": os.path.relpath(output, ROOT), "output_sha256": hashlib.sha256(data).hexdigest(),
                        "output_bytes": len(data), "adapter": reply.get("adapter"), "geometry": reply.get("geometry"),
                        "shading": reply.get("shading")})
    LEDGER.write_text("".join(json.dumps(r, sort_keys=True) + "\n" for r in records))
    return {"ok": True, "path": output, "sha256": records[-1]["output_sha256"], "bytes": len(data),
            "adapter": reply.get("adapter"), "geometry": reply.get("geometry"), "shading": reply.get("shading")}


def init(project, name, *, run_id=None):
    code, out, err = call(["init", Path(project), "--name", name, "--entities", 0], label=f"init {name}", run_id=run_id, timeout=60)
    if code != 0:
        raise EngineError(f"init failed: {err}")
    return json.loads(out)


def import_source(project, source, *, label, run_id=None, timeout=180):
    """Public `import` CLI. Returns dict with ok/exit/reply/stderr and project state before/after."""
    before = project_state(project)
    code, out, err = call(["import", Path(project), source], label=label, run_id=run_id, timeout=timeout,
                          extra={"kind": "import", "source": str(source), "state_before": before})
    after = project_state(project)
    result = {"ok": code == 0, "exit": code, "stderr": err.strip(), "state_before": before, "state_after": after}
    if code == 0:
        result["reply"] = json.loads(out)
    return result


def rpc(project, requests, *, label, run_id=None, timeout=120):
    """Run one `rpc` session with the import journal; returns the parsed response list."""
    project = Path(project)
    journal = project.with_suffix(".journal.jsonl")
    lines = "".join(json.dumps({"id": i, **r}) + "\n" for i, r in enumerate(requests))
    code, out, err = call(["rpc", project, "--journal", journal], label=label, stdin=lines, run_id=run_id,
                          timeout=timeout, extra={"kind": "rpc", "methods": [r["method"] for r in requests]})
    if code != 0:
        raise EngineError(f"{label}: rpc exited {code}: {err}")
    responses = [json.loads(line) for line in out.splitlines() if line.strip()]
    if len(responses) != len(requests):
        raise EngineError(f"{label}: {len(responses)} responses for {len(requests)} requests")
    return responses


def validate(project, *, label, run_id=None):
    code, out, err = call(["validate", Path(project)], label=label, run_id=run_id, timeout=60, extra={"kind": "validate"})
    return {"exit": code, "reply": json.loads(out) if out.strip() else None, "stderr": err.strip()}
