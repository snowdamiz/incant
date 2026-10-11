#!/usr/bin/env python3
"""Bounded 0043 color-role appearance delta runner.

Every engine process goes through this packet's tools/engine.py configure(): the
original 0040 harness behind Astra's per-subprocess Guard. The identity-specific
grant and the frozen binary hash are rechecked immediately before each Popen, each
child has a hard timeout and is reaped, and both binaries append to one ledger.
This runner adds a 20-frame cap that also counts failed screenshot launches, a
wall-clock timeline per child, and a release lock after which it refuses every
engine call. Only public CLI commands are used: init, import, rpc (project.read,
command.execute, project.save), validate and screenshot.

One set of projects is built by the baseline, rendered by the baseline ("old") and
by the candidate on the same retained cache ("retained"), reimported through the
command path by the candidate ("recooked"), then rendered again after every source
is deleted ("sourcefree").

Usage (worktree root):
  python3 -I -B handoffs/0043-color-mip-roles/tools/delta.py build
  python3 -I -B .../delta.py capture STATE PROJECT/CAMERA...   STATE: old|retained|recooked|sourcefree
  python3 -I -B .../delta.py recook
  python3 -I -B .../delta.py strip PROJECT
  python3 -I -B .../delta.py validate PROJECT
  python3 -I -B .../delta.py release
  python3 -I -B .../delta.py status
"""

import argparse
import datetime
import hashlib
import json
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import engine as guard  # noqa: E402  (this packet's guard; never the 0040 runner)
import fixtures  # noqa: E402

ROOT = guard.ROOT
ART = guard.ART
PROJECTS = ART / "projects"
FRAMES = ART / "frames"
BUILD = ART / "build.json"
LEDGER = ART / "ledger.jsonl"
TIMELINE = ART / "timeline.jsonl"
LOCK = ART / "RELEASED"
COMPLETE = ART / "captures-complete.json"
MAX_FRAMES, WIDTH, HEIGHT, FRAME_TIMEOUT = 20, 960, 540, 60
STATES = {"old": "baseline", "retained": "candidate", "recooked": "candidate", "sourcefree": "candidate"}
ROLES = ("baseline", "candidate")
_harnesses = {}


def binary(role):
    return ROOT / f"artifacts/tools/incant_headless-color-{role}"


def metadata(role):
    return json.loads((ROOT / f"artifacts/tools/color-{role}.json").read_text(encoding="utf-8"))


def now():
    return datetime.datetime.now().astimezone().isoformat(timespec="milliseconds")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def rel(path):
    return str(Path(path).resolve().relative_to(ROOT))


def records(path=None):
    path = LEDGER if path is None else path
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def frame_launches():
    return sum(1 for r in records() if r.get("kind") == "screenshot")


def assert_open(kind):
    """Runner-side limits; the per-process grant/hash check stays in the guard."""
    if LOCK.exists():
        raise SystemExit("0043 slot released: this runner launches no further engine process")
    if kind == "screenshot" and frame_launches() >= MAX_FRAMES:
        raise SystemExit(f"frame cap reached: {MAX_FRAMES} launches, failures included")


def harness(role):
    if role not in _harnesses:
        _harnesses[role] = guard.configure(role)
    return _harnesses[role]


def timed(role, kind, label, run):
    """Run one guarded harness call and log its wall-clock window and ledger rows."""
    assert_open(kind)
    engine = harness(role)
    first = len(records())
    start = now()
    try:
        return run(engine)
    finally:
        rows = records()[first:]
        with TIMELINE.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"start": start, "end": now(), "role": role, "kind": kind, "label": label,
                                     "ledger_seq": [r["seq"] for r in rows],
                                     "pids": [r.get("pid") for r in rows]}) + "\n")


def import_source(role, project, source, usage, run_id):
    label = f"{project.parent.name} import {source}"
    args = ["import", project, source] + (["--texture-usage", usage] if usage else [])

    def run(engine):
        before = engine.project_state(project)
        code, out, err = engine.call(args, label=label, run_id=run_id, timeout=120,
                                     extra={"kind": "import", "source": source, "usage": usage,
                                            "state_before": before})
        return code, out, err, before, engine.project_state(project)

    code, out, err, before, after = timed(role, "import", label, run)
    if code != 0:
        raise SystemExit(f"{label}: exit {code}: {err.strip()}")
    return json.loads(out), before, after


def rpc(role, project, requests, run_id, label):
    replies = timed(role, "rpc", label,
                    lambda engine: engine.rpc(project, requests, label=label, run_id=run_id, timeout=60))
    errors = [r.get("error") for r in replies if "error" in r]
    if errors:
        raise SystemExit(f"{label}: {errors}")
    return replies


def validate(role, project, run_id):
    label = f"{project.parent.name} validate ({run_id})"
    result = timed(role, "validate", label, lambda engine: engine.validate(project, label=label, run_id=run_id))
    if result["exit"] != 0 or not (result["reply"] or {}).get("valid"):
        raise SystemExit(f"{label}: {result}")
    return result["reply"]


def authored(project):
    journal = project.with_suffix(".journal.jsonl")
    return {"project": sha(project), "journal": sha(journal) if journal.exists() else None}


def cache_files(project):
    cache = project.parent / ".incant"
    return {rel(p): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}


def load_build():
    return json.loads(BUILD.read_text(encoding="utf-8")) if BUILD.exists() else {}


def save_build(record):
    BUILD.write_text(json.dumps(record, indent=1, sort_keys=True) + "\n", encoding="utf-8")


# ------------------------------------------------------------------ steps --

def build():
    record = load_build()
    for name, (write_sources, imports, scene) in fixtures.PROJECTS.items():
        if name in record:
            raise SystemExit(f"{name} already built")
        directory = PROJECTS / name
        directory.mkdir(parents=True, exist_ok=False)
        write_sources(directory)
        sources = {p.name: sha(p) for p in sorted(directory.iterdir()) if p.is_file()}
        project = directory / f"{name}.incant.json"
        timed("baseline", "init", f"{name} init", lambda engine: engine.init(project, f"0043 {name}", run_id="build"))
        scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))
        assets, receipts = {}, {}
        for source, usage in imports:
            reply, _, _ = import_source("baseline", project, source, usage, "build")
            assets[source] = reply["asset"]["id"]
            receipts[source] = {"asset": reply["asset"], "changed": reply.get("changed"),
                                "revision": reply.get("revision")}
        spec = scene(assets)
        revision = rpc("baseline", project, [{"method": "project.read"}], "build",
                       f"{name} read revision")[0]["result"]["revision"]
        rpc("baseline", project, [
            {"method": "command.execute", "params": {
                "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in spec.entities],
                "expected_revision": revision, "description": f"0043 {name} color-role fixture"}},
            {"method": "project.save"}], "build", f"{name} author entities")
        valid = validate("baseline", project, "build")
        record[name] = {
            "project": rel(project), "scene": scene_id, "assets": assets, "receipts": receipts,
            "entities": spec.names, "meta": spec.meta, "sources": sources, "validate": valid,
            "views": {k: {"camera": spec.names[f"Camera {k}"], "eye": list(v.eye), "rotation": v.rotation,
                          "fov": v.fov, "ortho_half_height": v.ortho, "size": list(v.size)}
                      for k, v in spec.views.items()},
            "authored": authored(project), "cache_files": cache_files(project),
            "framing": fixtures.framing(spec),
        }
        save_build(record)
        print(f"built {name}: {len(spec.entities)} entities, {len(assets)} assets")


def capture(state, views):
    role = STATES[state]
    record = load_build()
    for view in views:
        name, camera = view.split("/")
        info = record[name]
        project = ROOT / info["project"]
        out = FRAMES / state / f"{name}-{camera}.png"
        label = f"{state} {view}"
        shot = timed(role, "screenshot", label, lambda engine: engine.screenshot(
            project, out, label=label, camera=info["views"][camera]["camera"], width=WIDTH, height=HEIGHT,
            run_id=state, timeout=FRAME_TIMEOUT))
        row = records()[-1]
        print(json.dumps({"seq": row["seq"], "pid": row["pid"], "role": role, "view": view, "exit": row["exit"],
                          "elapsed_s": row["elapsed_s"], "adapter": shot.get("adapter"),
                          "sha256": shot.get("sha256"), "frames_used": frame_launches()}))
        if not shot["ok"]:
            raise SystemExit(f"{label} failed: {shot}")


def recook():
    record = load_build()
    if "recook" in record:
        raise SystemExit("already recooked")
    out = {}
    for name, (_, imports, _) in fixtures.PROJECTS.items():
        project = ROOT / record[name]["project"]
        rows = []
        for source, usage in imports:
            reply, before, after = import_source("candidate", project, source, usage, "recook")
            old = record[name]["receipts"][source]["asset"]
            rows.append({"source": source, "usage": usage, "asset": reply["asset"]["id"],
                         "same_asset_id": reply["asset"]["id"] == old["id"],
                         "fingerprint_before": old["sha256"], "fingerprint_after": reply["asset"]["sha256"],
                         "changed": reply.get("changed"), "revision": reply.get("revision"),
                         "project_changed": before["project_sha256"] != after["project_sha256"]})
        out[name] = {"imports": rows, "validate": validate("candidate", project, "recook"),
                     "authored": authored(project), "cache_files": cache_files(project)}
        print(json.dumps({name: [(r["source"], r["changed"], r["fingerprint_before"][:12],
                                  r["fingerprint_after"][:12]) for r in rows]}))
    record["recook"] = out
    save_build(record)


def strip(name):
    """Delete this project's generated sources (scratch inputs only)."""
    record = load_build()
    project = ROOT / record[name]["project"]
    removed = []
    for path in sorted(project.parent.iterdir()):
        if path.is_file() and not path.name.startswith(f"{name}.incant"):
            removed.append({"path": rel(path), "sha256": sha(path)})
            path.unlink()
    remaining = sorted(rel(p) for p in project.parent.iterdir() if p.is_file())
    record.setdefault("strip", {})[name] = {"removed": removed, "remaining": remaining}
    save_build(record)
    print(json.dumps(record["strip"][name], indent=1))


def validate_step(name):
    record = load_build()
    project = ROOT / record[name]["project"]
    reply = validate("candidate", project, "sourcefree")
    record.setdefault("validate_sourcefree", {})[name] = reply
    save_build(record)
    print(json.dumps(reply))


def pgrep(path):
    found = subprocess.run(["pgrep", "-f", str(path)], capture_output=True, text=True)
    if found.returncode not in (0, 1):
        raise SystemExit(f"pgrep failed: {found.stderr.strip()}")
    return [int(p) for p in found.stdout.split()]


def release():
    """After the last child exits: prove none runs, write the ledger summary, lock."""
    if LOCK.exists():
        raise SystemExit("already released")
    running = {role: pgrep(binary(role)) for role in ROLES}
    rows = records()
    unexited = [r["seq"] for r in rows if r.get("exit") is None]
    if any(running.values()) or unexited:
        raise SystemExit(f"not complete: running={running} unexited={unexited}")
    identities = {role: metadata(role) for role in ROLES}
    by_hash = {identities[role]["sha256"]: role for role in ROLES}
    windows = {seq: w for w in records(TIMELINE) for seq in w["ledger_seq"]}
    children = []
    for r in rows:
        children.append({
            "seq": r["seq"], "run": r.get("run"), "label": r["label"], "kind": r.get("kind"), "pid": r["pid"],
            "exit": r["exit"], "timed_out": r["timed_out"], "timeout_s": r["timeout_s"],
            "elapsed_s": r["elapsed_s"], "role": by_hash.get(r["binary_sha256_before"]),
            "binary_sha256_before": r["binary_sha256_before"], "binary_sha256_after": r["binary_sha256_after"],
            "start": windows.get(r["seq"], {}).get("start"), "end": windows.get(r["seq"], {}).get("end"),
            "output": r.get("output"), "output_sha256": r.get("output_sha256"), "adapter": r.get("adapter")})
    frames = [c for c in children if c["kind"] == "screenshot"]
    result = {
        "written_at": now(),
        "children": children,
        "children_total": len(children),
        "frame_launches": len(frames),
        "frames_written": sum(1 for c in frames if c["output_sha256"]),
        "frame_cap": MAX_FRAMES,
        "frame_size": [WIDTH, HEIGHT],
        "all_exit_zero": all(c["exit"] == 0 for c in children),
        "timeouts": [c["seq"] for c in children if c["timed_out"]],
        "every_child_hash_matches_identity": all(
            c["role"] and c["binary_sha256_before"] == c["binary_sha256_after"] == identities[c["role"]]["sha256"]
            for c in children),
        "first_child_start": min((c["start"] for c in children if c["start"]), default=None),
        "last_child_end": max((c["end"] for c in children if c["end"]), default=None),
        "identities": {role: {"source_commit": identities[role]["source_commit"],
                              "sha256": identities[role]["sha256"], "binary": rel(binary(role))}
                       for role in ROLES},
        "grants_present_at_release": {role: (ROOT / f"artifacts/tools/color-{role}-ready").exists()
                                      for role in ROLES},
        "process_inventory": {role: running[role] for role in ROLES},
        "release": "All 0043 baseline/candidate children have exited and were reaped. The graphics slot "
                   "can be released; Claude launches no further renderer or engine process for 0043.",
    }
    COMPLETE.write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    LOCK.write_text(result["written_at"] + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in result.items() if k != "children"}, indent=1))
    print(f"ASTRA: 0043 captures complete at {result['written_at']}; ledger {rel(COMPLETE)}. "
          "No renderer will launch again; the slot can be withdrawn.")


def status():
    rows = records()
    print(json.dumps({"children": len(rows), "frame_launches": frame_launches(), "cap": MAX_FRAMES,
                      "released": LOCK.exists(), "failures": [r["seq"] for r in rows if r.get("exit") != 0],
                      "grants": {role: (ROOT / f"artifacts/tools/color-{role}-ready").exists() for role in ROLES}},
                     indent=1))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="step", required=True)
    sub.add_parser("build")
    c = sub.add_parser("capture")
    c.add_argument("state", choices=sorted(STATES))
    c.add_argument("views", nargs="+")
    sub.add_parser("recook")
    s = sub.add_parser("strip")
    s.add_argument("project", choices=sorted(fixtures.PROJECTS))
    v = sub.add_parser("validate")
    v.add_argument("project", choices=sorted(fixtures.PROJECTS))
    sub.add_parser("release")
    sub.add_parser("status")
    args = parser.parse_args()
    if args.step == "build":
        build()
    elif args.step == "capture":
        capture(args.state, args.views)
    elif args.step == "recook":
        recook()
    elif args.step == "strip":
        strip(args.project)
    elif args.step == "validate":
        validate_step(args.project)
    elif args.step == "release":
        release()
    else:
        status()


if __name__ == "__main__":
    main()
