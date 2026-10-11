#!/usr/bin/env python3
"""Public-path driver for handoff 0042. Standard library only.

Only the frozen artifacts/tools executables run. Before and after every call the
SHA-256 of incant_headless, cook_scene and native_frames is checked against
binary.json, and every call is appended to artifacts/0042-native-flipbook/
engine-calls.jsonl. Projects are built only through incant_headless init,
import, rpc (command.execute, project.save, project.read) and validate; this
script never writes a project or journal file. Source PNG/atlas files and
native_frames request JSON are authored inputs. Scenes are cooked with
cook_scene. Every GPU capture goes through Astra's capture.py wrapper, which
checks the gate and the frozen native_frames hash per call and enforces the
packet budget; this script never runs native_frames itself.

  build RUN                 art, the animated project, one static control project per
                            moment, cooked scenes and request files (CPU only)
  preview RUN TICK CAMERA   approximate CPU layout preview (not evidence)
  capture RUN NAME          one capture.py invocation: main | ortho | subset |
                            subset-source-free | ctl-tNNNNN-pp | ctl-tNNNNN-ortho
  source-free RUN           copy, then delete the animated project's sources
                            (project, journal, PNG/atlas); keep cooked scene/caches

Usage (repo root):
  python3 -I -B handoffs/0042-native-flipbook/helpers/fb_review.py build artifacts/0042-native-flipbook/r1
"""

import argparse
import datetime
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HELPERS = Path(__file__).resolve().parent
PACKET = HELPERS.parent
ROOT = HELPERS.parents[2]
sys.path.insert(0, str(HELPERS))
import fb_scene as fs  # noqa: E402

TOOLS = ROOT / "artifacts/tools"
EXECUTABLES = {name: TOOLS / name for name in ("incant_headless", "cook_scene", "native_frames")}
BINARY_JSON = TOOLS / "binary.json"
READY = TOOLS / "gpu-review-ready"
CAPTURE = PACKET / "capture.py"
BUDGET = ROOT / "artifacts/flipbook-capture-budget.json"
ARTIFACTS = ROOT / "artifacts/0042-native-flipbook"
CALLS_LOG = ARTIFACTS / "engine-calls.jsonl"
LEDGER = ARTIFACTS / "ledger.jsonl"
PACKET_CAPTURES, PACKET_BYTES, PACKET_CALLS = 96, 256 << 20, 40
COMMITTED_ART = PACKET / "art"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).astimezone().isoformat(timespec="milliseconds")


def verify_tools():
    manifest = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["tools"]
    actual = {name: sha(path) for name, path in EXECUTABLES.items()}
    for name, digest in actual.items():
        if digest != manifest[name]["sha256"]:
            raise SystemExit(f"{name} sha256 {digest} != binary.json {manifest[name]['sha256']}")
    return actual


def rel(path):
    path = Path(path)
    try:
        return str(path.resolve().relative_to(ROOT))
    except ValueError:
        return str(path)


def log_call(entry):
    with CALLS_LOG.open("a", encoding="utf-8") as log:
        log.write(json.dumps(entry) + "\n")


def run_tool(tool, *args, check=True, stdin=None, label=None):
    before = verify_tools()
    argv = [str(EXECUTABLES[tool]), *[str(a) for a in args]]
    started = now()
    result = subprocess.run(argv, capture_output=True, text=True, cwd=ROOT, input=stdin)
    after = verify_tools()
    log_call({"tool": tool, "argv": [rel(a) if str(a).startswith(str(ROOT)) else str(a) for a in argv[1:]],
              "exit": result.returncode, "label": label, "started": started, "ended": now(),
              "sha256_before": before[tool], "sha256_after": after[tool]})
    if before != after:
        raise SystemExit("an executable changed during a call")
    if check and result.returncode != 0:
        raise SystemExit(f"{tool} {argv[1:4]} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}")
    return result


class Project:
    def __init__(self, directory, stem):
        self.dir = Path(directory)
        self.file = self.dir / f"{stem}.incant.json"
        self.journal = self.dir / f"{stem}.incant.journal.jsonl"

    def authored(self):
        return {p.name: sha(p) for p in (self.file, self.journal) if p.exists()}

    def rpc(self, requests, label=None):
        result = run_tool("incant_headless", "rpc", self.file, "--journal", self.journal, label=label,
                          stdin="".join(json.dumps(r) + "\n" for r in requests))
        replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        if any("error" in r for r in replies):
            raise SystemExit(f"RPC error: {[r for r in replies if 'error' in r]}")
        return replies

    def revision(self):
        return self.rpc([{"id": 1, "method": "project.read"}], label="project.read")[0]["result"]["revision"]

    def document(self):
        return json.loads(self.file.read_text(encoding="utf-8"))

    def scene_id(self):
        return next(iter(self.document()["scenes"]))

    def import_(self, source):
        prior = self.authored()
        result = run_tool("incant_headless", "import", self.file, source, "--texture-usage", "color",
                          label=f"import {source}")
        reply = json.loads(result.stdout)
        return {"source": source, "exit": result.returncode, "asset": reply["asset"], "changed": reply["changed"],
                "cache_hit": reply["cache_hit"], "details": reply["details"], "revision": reply.get("revision"),
                "diagnostics": reply.get("diagnostics", []), "authored_changed": prior != self.authored()}

    def cache(self, kind="textures"):
        return self.dir / ".incant/cache" / kind

    def cache_files(self):
        cache = self.dir / ".incant/cache"
        return {rel(p): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()} if cache.exists() else {}

    def validate(self):
        result = run_tool("incant_headless", "validate", self.file, label="validate")
        return {"exit": result.returncode, "reply": json.loads(result.stdout) if result.stdout.strip() else None}


def write_json(path, value):
    fs.write_json(path, value)


def manifest_of(run):
    return json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))


def make_project(run, name, entities_for, title):
    project = Project(run / name, name)
    (project.dir / "art").mkdir(parents=True)
    run_tool("incant_headless", "init", project.file, "--name", title, "--entities", "0", label=f"init {name}")
    for path in sorted((run / "art").iterdir()):
        if path.name != "manifest.json":
            shutil.copyfile(path, project.dir / "art" / path.name)
    imports = {key: project.import_(f"art/{source}") for key, source in
               (("atlas", "flip.atlas.json"), ("sheet", "sheet.png"), ("badge", "badge.png"),
                ("plates", "plates.png"))}
    ids = {key: entry["asset"]["id"] for key, entry in imports.items()}
    entities = entities_for(fs.Assets(ids, manifest_of(run)))
    scene = project.scene_id()
    request = {"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "create_entity", "scene_id": scene, "entity": e} for e in entities],
        "expected_revision": project.revision(), "description": f"{title} (handoff 0042)"}}
    replies = project.rpc([request, {"id": 3, "method": "project.save"}], label=f"create {name}")
    write_json(run / "entities" / f"{name}.json", entities)
    record = {"imports": imports, "ids": ids, "scene_id": scene,
              "transaction_id": replies[0]["result"]["transaction_id"], "revision": replies[0]["result"]["revision"],
              "entities": len(entities),
              "request_sha256": hashlib.sha256(json.dumps(request, sort_keys=True).encode()).hexdigest(),
              "validate": project.validate(), "authored": project.authored(), "cache": project.cache_files()}
    cooked = run / "cooked" / f"{name}.scene"
    cooked.parent.mkdir(exist_ok=True)
    result = run_tool("cook_scene", project.file, scene, cooked, label=f"cook {name}")
    data = cooked.read_bytes()
    record["cooked"] = {"path": rel(cooked), "sha256": sha(cooked), "bytes": len(data),
                        "version": int.from_bytes(data[8:12], "little"), "stdout": result.stdout.strip()}
    return record


def build(run):
    if run.exists():
        raise SystemExit("run directory already exists; choose a new one")
    run.mkdir(parents=True)
    start = verify_tools()
    subprocess.run([sys.executable, "-I", "-B", str(HELPERS / "fb_art.py"), str(run / "art")], check=True,
                   cwd=ROOT, capture_output=True)
    manifest = manifest_of(run)
    committed = COMMITTED_ART / "manifest.json"
    record = {"tools_sha256": start, "capture_py_sha256": sha(CAPTURE),
              "art_manifest_sha256": sha(run / "art/manifest.json"),
              "art_matches_committed": committed.exists() and json.loads(committed.read_text())["files"]
              == manifest["files"]}
    sc = fs.build()
    write_json(run / "scene.json", fs.describe(sc, manifest))
    record["anim"] = make_project(run, "anim", lambda assets: fs.animated_entities(sc, assets),
                                  "Flipbook review animated scene")
    for tick in fs.MOMENTS:
        name = f"ctl-t{tick:05d}"
        record[name] = make_project(run, name, lambda assets, t=tick: fs.control_entities(sc, assets, t),
                                    f"Flipbook static control at tick {tick}")
    requests = {"main": fs.request(sc, fs.MAIN_TICKS), "ortho": fs.request(sc, fs.ORTHO_TICKS),
                "subset": fs.request(sc, fs.SUBSET_TICKS), "control": fs.request(sc, [0], with_controls=False)}
    for name, value in requests.items():
        write_json(run / "requests" / f"{name}.json", value)
    record["requests"] = {name: sha(run / "requests" / f"{name}.json") for name in requests}
    (run / "empty-models").mkdir()
    record["tools_sha256_end"] = verify_tools()
    write_json(run / "build.json", record)
    print(json.dumps({"art_matches_committed": record["art_matches_committed"],
                      "anim_entities": record["anim"]["entities"],
                      "cooked_versions": {k: v["cooked"]["version"] for k, v in record.items()
                                          if isinstance(v, dict) and "cooked" in v},
                      "validate": {k: v["validate"]["reply"].get("valid") for k, v in record.items()
                                   if isinstance(v, dict) and "validate" in v}}))


# ----------------------------------------------------------------- capture --

def ledger_used():
    calls = captures = raw = 0
    if LEDGER.exists():
        for line in LEDGER.read_text(encoding="utf-8").splitlines():
            entry = json.loads(line)
            calls += 1
            captures += entry["captures"]
            raw += entry["raw_bytes"]
    return calls, captures, raw


def plan(run, name):
    """(cooked scene, project directory, camera item, request file) for a capture name."""
    if name in ("main", "ortho", "subset", "subset-source-free"):
        camera = "camera-ortho" if name == "ortho" else "camera-pp"
        request = "subset" if name.startswith("subset") else name
        return run / "cooked/anim.scene", run / "anim", camera, run / "requests" / f"{request}.json"
    parts = name.split("-")
    if len(parts) == 3 and parts[0] == "ctl" and parts[2] in ("pp", "ortho"):
        project = f"ctl-{parts[1]}"
        return (run / "cooked" / f"{project}.scene", run / project, f"camera-{parts[2]}",
                run / "requests/control.json")
    raise SystemExit(f"unknown capture {name}")


def capture(run, name):
    if not READY.is_file():
        raise SystemExit("artifacts/tools/gpu-review-ready is absent; GPU captures are not authorized yet")
    scene, project, camera_name, request_path = plan(run, name)
    source_free = (run / "source-free.json").exists()
    if name == "subset-source-free" and not source_free:
        raise SystemExit("run source-free first")
    if name != "subset-source-free" and source_free and project == run / "anim":
        raise SystemExit("animated sources were removed; only subset-source-free may run now")
    request = json.loads(request_path.read_text(encoding="utf-8"))
    count, raw = len(request["ticks"]), request["width"] * request["height"] * 4 * len(request["ticks"])
    calls, used, used_raw = ledger_used()
    if calls + 1 > PACKET_CALLS or used + count > PACKET_CAPTURES or used_raw + raw > PACKET_BYTES:
        raise SystemExit(f"budget: need {count} captures / {raw / 2**20:.2f} MiB; used {calls} calls, {used} "
                         f"captures, {used_raw / 2**20:.2f} MiB")
    output = run / "captures" / name
    if output.exists():
        raise SystemExit(f"{output} exists; captures are never overwritten")
    output.parent.mkdir(exist_ok=True)
    sc = fs.build()
    camera_hex = fs.ulid_hex(fs.eid(sc, camera_name))
    inputs = {"scene": sha(scene), "request": sha(request_path),
              "textures": {p.name: sha(p) for p in sorted((project / ".incant/cache/textures").iterdir())}}
    before = verify_tools()
    argv = [sys.executable, str(CAPTURE), str(scene), str(run / "empty-models"),
            str(project / ".incant/cache/textures"), camera_hex, str(request_path), str(output)]
    started = now()
    result = subprocess.run(argv, capture_output=True, text=True, cwd=ROOT)
    ended = now()
    after = verify_tools()
    written = sorted(output.glob("*.png")) if output.exists() else []
    entry = {"tool": "capture.py", "argv": [rel(a) if str(a).startswith(str(ROOT)) else a for a in argv[1:]],
             "exit": result.returncode, "label": f"capture {name}", "started": started, "ended": ended,
             "sha256_before": before["native_frames"], "sha256_after": after["native_frames"],
             "capture_py_sha256": sha(CAPTURE)}
    log_call(entry)
    with LEDGER.open("a", encoding="utf-8") as ledger:
        ledger.write(json.dumps({"run": run.name, "name": name, "captures": count, "raw_bytes": raw,
                                 "written": len(written), "exit": result.returncode}) + "\n")
    record = {"name": name, "camera": camera_name, "camera_hex": camera_hex, "inputs": inputs,
              "started": started, "ended": ended, "exit": result.returncode, "stdout": result.stdout.strip(),
              "stderr": result.stderr.strip()[-4000:], "tools_before": before, "tools_after": after,
              "pngs": {p.name: sha(p) for p in written},
              "report_sha256": sha(output / "report.json") if (output / "report.json").exists() else None,
              "budget_file": json.loads(BUDGET.read_text()) if BUDGET.exists() else None,
              "ledger_after": dict(zip(("calls", "captures", "raw_bytes"), ledger_used()))}
    write_json(run / "records" / f"{name}.json", record)
    if before != after:
        raise SystemExit("an executable changed during capture")
    if result.returncode != 0:
        raise SystemExit(f"capture failed ({result.returncode}): {result.stderr.strip()[-2000:]}")
    print(json.dumps({"name": name, "pngs": len(written), "wrapper": json.loads(result.stdout),
                      "ledger": record["ledger_after"]}))


# ------------------------------------------------------------- source-free --

def source_free(run):
    marker = run / "source-free.json"
    if marker.exists():
        raise SystemExit("source-free already ran")
    project = Project(run / "anim", "anim")
    cooked = run / "cooked/anim.scene"
    keep_before = {rel(cooked): sha(cooked), **project.cache_files()}
    sources = [project.file, project.journal] + sorted((project.dir / "art").iterdir())
    fixture = run / "private-fixture/anim"
    removed = {}
    for path in sources:
        target = fixture / path.relative_to(project.dir)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        if sha(target) != sha(path):
            raise SystemExit(f"fixture copy differs: {path}")
        removed[rel(path)] = sha(path)
    for path in sources:
        path.unlink()
    (project.dir / "art").rmdir()
    remaining = {rel(p): sha(p) for p in sorted(project.dir.rglob("*")) if p.is_file()}
    keep_after = {rel(cooked): sha(cooked), **project.cache_files()}
    record = {"removed": removed, "fixture_copy": rel(fixture), "remaining": remaining,
              "kept_unchanged": keep_before == keep_after, "kept": keep_after, "at": now()}
    write_json(marker, record)
    print(json.dumps({"removed": len(removed), "remaining": len(remaining),
                      "kept_unchanged": record["kept_unchanged"]}))


def preview(run, tick, camera_name):
    import fb_model as fm
    import fb_png
    sc = fs.build()
    w, h = fs.VIEW
    camera = fm.Camera(sc, camera_name, w, h)
    rows = fm.preview(sc, tick, camera, fm.load_art(run / "art"), manifest_of(run))
    out = run / "preview" / f"{camera_name}-t{tick:05d}.png"
    out.parent.mkdir(exist_ok=True)
    fb_png.write_rgba(out, w, h, rows)
    print(rel(out))


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("build", "source-free"):
        sub.add_parser(name).add_argument("run", type=Path)
    c = sub.add_parser("capture")
    c.add_argument("run", type=Path)
    c.add_argument("name")
    p = sub.add_parser("preview")
    p.add_argument("run", type=Path)
    p.add_argument("tick", type=int)
    p.add_argument("camera", choices=["camera-pp", "camera-ortho"])
    args = parser.parse_args()
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    run = args.run.resolve()
    if ARTIFACTS.resolve() not in run.parents:
        raise SystemExit("runs live under artifacts/0042-native-flipbook/")
    if args.command == "build":
        build(run)
    elif args.command == "capture":
        capture(run, args.name)
    elif args.command == "source-free":
        source_free(run)
    else:
        preview(run, args.tick, args.camera)


if __name__ == "__main__":
    main()
