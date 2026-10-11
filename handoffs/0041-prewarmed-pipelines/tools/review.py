#!/usr/bin/env python3
"""Bounded appearance regression runner for handoff 0041 (prewarmed pipelines).

Only public `incant_headless` commands are used: `init`, `import`, `rpc`
(`project.read`, `command.execute`, `history.undo`, `project.save`), `validate`
and `screenshot`. No project document or journal is written directly.

Rules enforced here:
  * artifacts/tools/binary.json is read and both frozen binaries are verified
    (sha256, size, source commit) before and after every execution.
  * Renderer processes (`screenshot`) require artifacts/tools/gpu-review-ready and
    are launched only through Astra's artifacts/tools/guarded-screenshot.py, which
    re-checks the gate and binary, caps launches at 40, serializes them, applies a
    60 s timeout and reaps its child.
  * At most 40 screenshots in total, each at most 960x540; every other subprocess
    has a 60 s timeout, runs in its own process group and is killed and reaped on
    timeout. Every execution is appended to the ledger.
  * `finish` removes the gate as Astra's release text instructs and confirms that
    no renderer process from either frozen binary is still running.
  * Baseline and candidate always render the same saved project and cache in
    separate processes.

Usage (worktree root):
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py plan
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py build
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py refuse-import
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py capture STATE VIEW... [--order candidate-first]
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py refuse-shadow
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py reimport
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py diagnostic
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py top-camera
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py strip PROJECT
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py status
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py finish
  python3 -I handoffs/0041-prewarmed-pipelines/tools/review.py package
VIEW is PROJECT/CAMERA, for example materials/board.
"""

import argparse
import datetime
import hashlib
import json
import os
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import fixtures  # noqa: E402

ROOT = TOOLS.parents[2]
ART = ROOT / "artifacts/0041-prewarmed-pipelines"
PROJECTS = ART / "projects"
SHOTS = ART / "shots"
LEDGER = ART / "ledger.jsonl"
BUILD = ART / "build.json"
MANIFEST = ROOT / "artifacts/tools/binary.json"
RELEASE = ROOT / "artifacts/tools/gpu-review-ready"
# Astra's mechanical guard: every screenshot must go through it (gate, binary
# identity, 40-launch counter, serialization, 60 s timeout, child reaping).
GUARD = ROOT / "artifacts/tools/guarded-screenshot.py"
GUARD_COUNT = ROOT / "artifacts/tools/capture-count"
GUARD_LEDGER = ROOT / "artifacts/tools/capture-ledger.jsonl"
SOURCES = {"baseline": "74238785fe92c50daea1d8dd056492359def61f2",
           "candidate": "21a2aa2fb9afcd5903453ca4c1cca32ebf72c6fb"}
MAX_CAPTURES, MAX_W, MAX_H, TIMEOUT = 40, 960, 540, 60
RENDERER_PATTERN = f"{ROOT}/artifacts/tools/((baseline|candidate)-incant_headless|guarded-screenshot)"
SHADOW_REFUSAL = "BLEND materials cannot cast directional shadows; disable cast_shadows or use MASK"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def rel(value):
    text = str(value)
    root = str(ROOT) + os.sep
    return text[len(root):] if text.startswith(root) else text


def verify_manifest():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    verified = {}
    for role in ("baseline", "candidate"):
        entry = manifest[role]
        data = (ROOT / entry["path"]).read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        if digest != entry["sha256"] or len(data) != entry["bytes"] or entry["source"] != SOURCES[role]:
            raise SystemExit(f"{role} binary does not match binary.json or the packet's source commit")
        verified[role] = {"path": entry["path"], "sha256": digest, "bytes": len(data), "source": entry["source"]}
    return verified


def release():
    if not RELEASE.is_file() or not RELEASE.read_text(encoding="utf-8").startswith("0041 approved"):
        raise SystemExit("artifacts/tools/gpu-review-ready is absent or not a 0041 release: GPU work is not released")
    return {"sha256": sha(RELEASE), "text": RELEASE.read_text(encoding="utf-8").strip()[:500]}


def ledger():
    if not LEDGER.exists():
        return []
    return [json.loads(line) for line in LEDGER.read_text(encoding="utf-8").splitlines() if line.strip()]


def captures_used():
    mine = sum(1 for entry in ledger() if entry["command"] == "screenshot")
    guard = int(GUARD_COUNT.read_text(encoding="utf-8")) if GUARD_COUNT.exists() else 0
    return max(mine, guard)


def png_dims(path):
    data = Path(path).read_bytes()[:24]
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return list(struct.unpack(">II", data[16:24]))


def engine(role, *args, phase, stdin=None, note=None):
    """Run one frozen binary with verified identity, a timeout and full reaping."""
    args = [str(a) for a in args]
    renderer = args[0] == "screenshot"
    gate = None
    if renderer:
        gate = release()
        used = captures_used()
        if used >= MAX_CAPTURES:
            raise SystemExit(f"capture budget exhausted ({used}/{MAX_CAPTURES})")
        w, h = int(args[args.index("--width") + 1]), int(args[args.index("--height") + 1])
        if w > MAX_W or h > MAX_H:
            raise SystemExit(f"capture {w}x{h} exceeds {MAX_W}x{MAX_H}")
    before = verify_manifest()
    if renderer:
        # screenshot PROJECT OUTPUT --camera ID --width W --height H, launched only by the guard.
        argv = [sys.executable, "-I", str(GUARD), role, *args[1:]]
    else:
        argv = [str(ROOT / before[role]["path"]), *args]
    started = time.time()
    proc = subprocess.Popen(argv, cwd=ROOT, stdin=subprocess.PIPE if stdin is not None else subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
    timed_out = False
    try:
        # The guard applies the 60 s engine timeout itself; allow it time to hash and log.
        stdout, stderr = proc.communicate(stdin, timeout=TIMEOUT + 30 if renderer else TIMEOUT)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(proc.pid, signal.SIGKILL)
        stdout, stderr = proc.communicate()
    duration = time.time() - started
    reaped = proc.returncode is not None
    after = verify_manifest()
    if after != before:
        raise SystemExit("binary changed during execution")
    record = {"seq": len(ledger()) + 1, "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
              "phase": phase, "role": role, "binary_sha256": before[role]["sha256"],
              "source": before[role]["source"], "manifest_verified_before_and_after": True,
              "command": args[0], "args": [rel(a) for a in args], "exit": proc.returncode, "timed_out": timed_out,
              "reaped": reaped, "duration_s": round(duration, 3), "note": note}
    if gate:
        record["gpu_release_sha256"] = gate["sha256"]
        record["via_guard"] = rel(GUARD)
        record["guard_sha256"] = sha(GUARD)
        lines = GUARD_LEDGER.read_text(encoding="utf-8").splitlines() if GUARD_LEDGER.exists() else []
        if lines:
            last = json.loads(lines[-1])
            record["guard_record"] = {k: last.get(k) for k in ("sequence", "returncode", "png_sha256",
                                                                "binary_sha256", "elapsed_seconds", "child_reaped")}
    try:
        parsed = json.loads(stdout) if stdout.strip() and args[0] != "rpc" else None
    except json.JSONDecodeError:
        parsed = None
    if args[0] == "rpc":
        record["rpc_replies"] = [summarize_reply(json.loads(line)) for line in stdout.splitlines() if line.strip()]
    elif parsed is not None:
        record["stdout"] = parsed
    else:
        record["stdout_tail"] = stdout[-2000:]
    record["stderr_tail"] = stderr[-2000:]
    if renderer:
        out = Path(args[2]) if Path(args[2]).is_absolute() else ROOT / args[2]
        record["output_exists"] = out.exists()
        if out.exists():
            record["output_sha256"] = sha(out)
            record["output_png_dims"] = png_dims(out)
        if parsed:
            record["adapter"] = parsed.get("adapter")
            record["reported_dims"] = [parsed.get("width"), parsed.get("height")]
    with LEDGER.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")
    return record, stdout, stderr


def summarize_reply(reply):
    result = reply.get("result")
    if isinstance(result, dict) and "project" in result:
        result = {k: v for k, v in result.items() if k != "project"}
        result["project_omitted"] = True
    return {"id": reply.get("id"), "result": result, "error": reply.get("error")}


def rpc(project, requests, phase, note=None):
    journal = project.with_suffix(".journal.jsonl")
    payload = "".join(json.dumps(r) + "\n" for r in requests)
    record, stdout, _ = engine("candidate", "rpc", rel(project), "--journal", rel(journal), phase=phase,
                               stdin=payload, note=note)
    replies = [json.loads(line) for line in stdout.splitlines() if line.strip()]
    if record["exit"] != 0 or any("error" in r for r in replies):
        raise SystemExit(f"rpc failed: {[r.get('error') for r in replies]} {record['stderr_tail']}")
    return replies


def authored(project):
    return {"project": sha(project), "journal": sha(project.with_suffix(".journal.jsonl"))}


# ------------------------------------------------------------------ steps --

def plan():
    report = {}
    for name, (_, imports, scene) in fixtures.PROJECTS.items():
        assets = {source: f"01JA41PWX0{index:016d}" for index, (source, _) in enumerate(imports())}
        report[name] = fixtures.framing(scene(assets))
    print(json.dumps(report, indent=1))


def build(names):
    ART.mkdir(parents=True, exist_ok=True)
    record = json.loads(BUILD.read_text(encoding="utf-8")) if BUILD.exists() else {}
    for name in names:
        if name in record:
            raise SystemExit(f"{name} already built")
        write_sources, imports, scene = fixtures.PROJECTS[name]
        directory = PROJECTS / name
        directory.mkdir(parents=True, exist_ok=False)
        written = write_sources(directory)
        project = directory / f"{name}.incant.json"
        result, _, _ = engine("candidate", "init", rel(project), "--name", f"0041 {name}", "--entities", "0",
                              phase="build")
        if result["exit"] != 0:
            raise SystemExit(result["stderr_tail"])
        scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))
        assets, receipts = {}, {}
        for source, usage in imports():
            args = ["import", rel(project), source] + (["--texture-usage", usage] if usage else [])
            result, stdout, _ = engine("candidate", *args, phase="build")
            if result["exit"] != 0:
                raise SystemExit(f"import {source} failed: {result['stderr_tail']}")
            reply = json.loads(stdout)
            assets[source] = reply["asset"]["id"]
            receipts[source] = {"asset": reply["asset"], "changed": reply.get("changed"),
                                "revision": reply.get("revision")}
        spec = scene(assets)
        revision = rpc(project, [{"id": 1, "method": "project.read"}], "build")[0]["result"]["revision"]
        rpc(project, [{"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in spec.entities],
            "expected_revision": revision, "description": f"0041 {name} appearance fixture"}},
            {"id": 3, "method": "project.save"}], "build")
        result, _, _ = engine("candidate", "validate", rel(project), phase="build")
        if result["exit"] != 0:
            raise SystemExit(f"validate failed: {result['stderr_tail']}")
        cache = directory / ".incant/cache"
        record[name] = {
            "project": rel(project), "scene": scene_id, "assets": assets, "receipts": receipts,
            "entities": spec.names, "meta": spec.meta,
            "views": {k: {"camera": spec.names[f"Camera {k}"], "eye": v.eye, "rotation": v.rotation, "fov": v.fov,
                          "ortho_half_height": v.ortho, "pixel": v.pixel, "size": list(v.size)}
                      for k, v in spec.views.items()},
            "sources": sorted(rel(p) for p in directory.iterdir()
                              if p.is_file() and not p.name.startswith(f"{name}.incant")),
            "authored": authored(project),
            "cache_files": {rel(p): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()},
            "framing": fixtures.framing(spec),
        }
        BUILD.write_text(json.dumps(record, indent=1, sort_keys=True), encoding="utf-8")
        print(f"built {name}: {len(spec.entities)} entities, {len(assets)} assets")


def built(name):
    return json.loads(BUILD.read_text(encoding="utf-8"))[name]


def refuse_import():
    """A deliberately unsupported material extension must be refused by both builds, atomically."""
    info = built("materials")
    project = ROOT / info["project"]
    source = project.parent / "refused_emissive_strength.gltf"
    document = json.loads((project.parent / "card_opaque_single.gltf").read_text(encoding="utf-8"))
    document["extensionsUsed"] = ["KHR_materials_emissive_strength"]
    document["materials"][0]["extensions"] = {"KHR_materials_emissive_strength": {"emissiveStrength": 4.0}}
    document["materials"][0]["emissiveFactor"] = [1.0, 0.5, 0.2]
    source.write_text(json.dumps(document, indent=1), encoding="utf-8")
    outcome = {}
    for role in ("baseline", "candidate"):
        before = authored(project)
        result, _, stderr = engine(role, "import", rel(project), source.name, phase="refusal",
                                   note="deliberately unsupported glTF material extension")
        outcome[role] = {"exit": result["exit"], "stderr": stderr.strip()[-400:],
                         "authored_unchanged": authored(project) == before}
    source.unlink()
    print(json.dumps(outcome, indent=1))


def capture(state, views, order):
    roles = ["candidate", "baseline"] if order == "candidate-first" else ["baseline", "candidate"]
    release()
    for view in views:
        name, camera = view.split("/")
        info = built(name)
        spec = info["views"][camera]
        w, h = spec["size"]
        for role in roles:
            out = SHOTS / state / f"{name}-{camera}-{role}.png"
            if out.exists():
                out = SHOTS / state / f"{name}-{camera}-{role}-repeat.png"
            if out.exists():
                raise SystemExit(f"{rel(out)} exists")
            out.parent.mkdir(parents=True, exist_ok=True)
            result, _, _ = engine(role, "screenshot", info["project"], rel(out), "--camera", spec["camera"],
                                  "--width", w, "--height", h, phase=f"capture:{state}")
            print(json.dumps({k: result.get(k) for k in ("seq", "role", "exit", "timed_out", "adapter",
                                                           "output_sha256", "output_png_dims", "duration_s")}))
            if result["exit"] != 0:
                raise SystemExit(f"capture failed: {result['stderr_tail']}")
    print(f"captures used: {captures_used()}/{MAX_CAPTURES}")


def refuse_shadow():
    info = built("shadows")
    project = ROOT / info["project"]
    entity = info["entities"]["BLEND single up (non-caster)"]
    asset = info["assets"]["card_blend_single.gltf"]
    before = authored(project)
    revision = rpc(project, [{"id": 1, "method": "project.read"}], "refusal")[0]["result"]["revision"]
    rpc(project, [{"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "set_component", "scene_id": info["scene"], "entity_id": entity,
                      "component": "MeshRenderer", "value": {"mesh": asset, "materials": [], "cast_shadows": True}}],
        "expected_revision": revision, "description": "0041 refusal: BLEND card casts directional shadows"}},
        {"id": 3, "method": "project.save"}], "refusal")
    release()
    outcome = {}
    for role in ("baseline", "candidate"):
        out = SHOTS / "refusal" / f"shadows-board-{role}.png"
        out.parent.mkdir(parents=True, exist_ok=True)
        result, _, stderr = engine(role, "screenshot", info["project"], rel(out), "--camera",
                                   info["views"]["board"]["camera"], "--width", 960, "--height", 540,
                                   phase="capture:refusal", note="expected TransparentShadowCaster refusal")
        outcome[role] = {"exit": result["exit"], "stderr": stderr.strip()[-400:], "png_written": out.exists(),
                         "expected_message": SHADOW_REFUSAL in stderr}
    rpc(project, [{"id": 4, "method": "history.undo"}, {"id": 5, "method": "project.save"}], "refusal",
        note="restore the non-caster through public undo")
    restored = authored(project)
    outcome["project_restored"] = restored["project"] == before["project"]
    print(json.dumps(outcome, indent=1))
    print(f"captures used: {captures_used()}/{MAX_CAPTURES}")


def reimport():
    info = built("materials")
    project = ROOT / info["project"]
    before = sha(project.parent / "card_opaque_single.gltf")
    fixtures.materials_sources(project.parent, reimport=True)
    after = sha(project.parent / "card_opaque_single.gltf")
    result, stdout, _ = engine("candidate", "import", rel(project), "card_opaque_single.gltf", phase="reimport",
                               note="accepted material change: OPAQUE single -> tinted double-sided MASK 0.3")
    if result["exit"] != 0:
        raise SystemExit(result["stderr_tail"])
    reply = json.loads(stdout)
    print(json.dumps({"source_before": before, "source_after": after, "asset": reply["asset"],
                      "changed": reply.get("changed"), "revision": reply.get("revision"),
                      "same_asset_id": reply["asset"]["id"] == info["assets"]["card_opaque_single.gltf"]}, indent=1))


def diagnostic():
    """Add one Transform-only authoring diagnostic cube (the moved diagnostic pipeline) to lights."""
    info = built("lights")
    project = ROOT / info["project"]
    ids = fixtures.Ids("01JA41PWG0")
    ids.counter = 99
    entity = {"id": ids.next(), "name": "Diagnostic cube", "parent": None,
              "components": {"Transform": fixtures.transform((-1.9, 0.45, 2.6), fixtures.quat_axis((0, 1, 0), 25),
                                                             (0.9, 0.9, 0.9))}}
    revision = rpc(project, [{"id": 1, "method": "project.read"}], "diagnostic")[0]["result"]["revision"]
    rpc(project, [{"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "create_entity", "scene_id": info["scene"], "entity": entity}],
        "expected_revision": revision, "description": "0041 diagnostic pipeline coverage"}},
        {"id": 3, "method": "project.save"}], "diagnostic")
    print(json.dumps({"entity": entity["id"], "authored": authored(project)}, indent=1))


def top_camera():
    """Add a top-down orthographic camera so each horizontal card sits beside its congruent shadow."""
    record = json.loads(BUILD.read_text(encoding="utf-8"))
    info = record["shadows"]
    project = ROOT / info["project"]
    ids = fixtures.Ids("01JA41PWS0")
    ids.counter = 99
    view = fixtures.View("top", (0.8, 20.0, 0.4), fixtures.quat_axis((1, 0, 0), -90), fov=40.0, far=60.0,
                         ortho_half_height=7.0)
    entity = {"id": ids.next(), "name": "Camera top", "parent": None,
              "components": {"Camera": view.camera_component(),
                             "Transform": fixtures.transform(view.eye, view.rotation)}}
    revision = rpc(project, [{"id": 1, "method": "project.read"}], "top-camera")[0]["result"]["revision"]
    rpc(project, [{"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "create_entity", "scene_id": info["scene"], "entity": entity}],
        "expected_revision": revision, "description": "0041 top-down shadow review camera"}},
        {"id": 3, "method": "project.save"}], "top-camera")
    info["views"]["top"] = {"camera": entity["id"], "eye": view.eye, "rotation": view.rotation, "fov": view.fov,
                            "ortho_half_height": view.ortho, "pixel": None, "size": list(view.size)}
    BUILD.write_text(json.dumps(record, indent=1, sort_keys=True), encoding="utf-8")
    print(json.dumps({"camera": entity["id"], "authored": authored(project)}, indent=1))


def strip(name):
    info = built(name)
    project = ROOT / info["project"]
    removed = []
    for path in sorted(project.parent.iterdir()):
        if path.is_file() and not path.name.startswith(f"{name}.incant"):
            removed.append({"path": rel(path), "sha256": sha(path)})
            path.unlink()
    remaining = sorted(rel(p) for p in project.parent.iterdir() if p.is_file())
    print(json.dumps({"removed": removed, "remaining": remaining}, indent=1))


def finish():
    """Release the GPU slot: remove Astra's gate, then prove no renderer child survives."""
    gate = release()
    running = subprocess.run(["pgrep", "-fl", RENDERER_PATTERN],
                             capture_output=True, text=True).stdout.strip()
    if running:
        raise SystemExit(f"renderer processes still running:\n{running}")
    RELEASE.unlink()
    after = subprocess.run(["pgrep", "-fl", RENDERER_PATTERN],
                           capture_output=True, text=True).stdout.strip()
    note = {"utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
            "gate_removed": not RELEASE.exists(), "gate_sha256": gate["sha256"], "running_after": after,
            "captures_used": captures_used()}
    (ART / "gpu-slot-released.json").write_text(json.dumps(note, indent=1), encoding="utf-8")
    print(json.dumps(note, indent=1))


RETAINED = {  # packet name: (state, capture); every candidate copy is byte-identical to its baseline twin
    "materials-board.png": ("initial", "materials-board"),
    "materials-normals.png": ("initial", "materials-normals"),
    "materials-board-reimported.png": ("reimport", "materials-board"),
    "shadows-board.png": ("initial", "shadows-board"),
    "shadows-top.png": ("initial", "shadows-top"),
    "lights-board.png": ("initial", "lights-board"),
    "lights-board-diagnostic-cube.png": ("diagnostic", "lights-board"),
    "sprites-pixel.png": ("initial", "sprites-pixel"),
    "sprites-perspective.png": ("initial", "sprites-perspective"),
}


def package():
    """Copy unedited retained captures and sanitized evidence into the handoff packet."""
    packet = TOOLS.parent
    screenshots, evidence = packet / "screenshots", packet / "evidence"
    screenshots.mkdir(exist_ok=True)
    evidence.mkdir(exist_ok=True)
    lines = []
    for name, (state, capture) in RETAINED.items():
        candidate, baseline = SHOTS / state / f"{capture}-candidate.png", SHOTS / state / f"{capture}-baseline.png"
        if sha(candidate) != sha(baseline):
            raise SystemExit(f"{name}: baseline and candidate differ; retain both")
        (screenshots / name).write_bytes(candidate.read_bytes())
        lines.append(f"{sha(screenshots / name)}  screenshots/{name}  (= {rel(candidate)} = {rel(baseline)})")
    (evidence / "screenshots.sha256").write_text("\n".join(lines) + "\n", encoding="utf-8")
    prefix = str(ROOT) + os.sep
    for source, target in ((LEDGER, "ledger.jsonl"), (BUILD, "fixtures.json"),
                           (ART / "gpu-slot-released.json", "gpu-slot-released.json"),
                           (ART / "strip-materials.json", "strip-materials.json"),
                           (ART / "strip-sprites.json", "strip-sprites.json")):
        text = source.read_text(encoding="utf-8").replace(prefix, "")
        if "/Users/" in text:
            raise SystemExit(f"{target} still contains an absolute path")
        (evidence / target).write_text(text, encoding="utf-8")
    print("\n".join(lines))


def status():
    entries = ledger()
    print(json.dumps({"executions": len(entries), "captures_used": captures_used(), "budget": MAX_CAPTURES,
                      "release_present": RELEASE.is_file(),
                      "failures": [e["seq"] for e in entries if e["exit"] != 0]}, indent=1))


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="step", required=True)
    sub.add_parser("plan")
    b = sub.add_parser("build")
    b.add_argument("projects", nargs="*", default=list(fixtures.PROJECTS))
    sub.add_parser("refuse-import")
    c = sub.add_parser("capture")
    c.add_argument("state")
    c.add_argument("views", nargs="+")
    c.add_argument("--order", choices=["baseline-first", "candidate-first"], default="baseline-first")
    sub.add_parser("refuse-shadow")
    sub.add_parser("reimport")
    sub.add_parser("diagnostic")
    sub.add_parser("top-camera")
    s = sub.add_parser("strip")
    s.add_argument("project")
    sub.add_parser("status")
    sub.add_parser("finish")
    sub.add_parser("package")
    args = parser.parse_args()
    if args.step == "plan":
        plan()
    elif args.step == "build":
        build(args.projects)
    elif args.step == "refuse-import":
        refuse_import()
    elif args.step == "capture":
        capture(args.state, args.views, args.order)
    elif args.step == "refuse-shadow":
        refuse_shadow()
    elif args.step == "reimport":
        reimport()
    elif args.step == "diagnostic":
        diagnostic()
    elif args.step == "top-camera":
        top_camera()
    elif args.step == "strip":
        strip(args.project)
    elif args.step == "finish":
        finish()
    elif args.step == "package":
        package()
    else:
        status()


if __name__ == "__main__":
    main()
