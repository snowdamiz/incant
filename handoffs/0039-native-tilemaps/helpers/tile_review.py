#!/usr/bin/env python3
"""Public-path engine driver for handoff 0039. Standard library only.

Only the frozen artifacts/tools/incant_headless runs, and its SHA-256 is checked
against binary.json before and after every call. Projects are built and changed
only through init, import, rpc (command.execute, history.undo/redo, project.save,
project.read, history.read), validate and screenshot. No project or journal file
is written by this script; art and atlas descriptors are copied in as authoring
inputs, and source-free states delete those inputs only.

  build RUN                       art, the tile project (s0) and controls for s0 and s1
  shoot RUN PROJECT STATE VIEWS   capture named views (<= 16 per invocation)
  step RUN ACTION                 s1-edit | undo | redo | repack | reject | import-undo |
                                  import-redo | source-free

Packet bounds: 128 captures and 256 MiB of raw decoded RGBA in total (ledger),
16 frames per invocation. Screenshots wait for artifacts/tools/gpu-review-ready.

Usage (repo root):
  python3 -I -B handoffs/0039-native-tilemaps/helpers/tile_review.py build artifacts/0039-native-tilemaps/r1
"""

import argparse
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
import tile_scene as ts  # noqa: E402

CLI = ROOT / "artifacts/tools/incant_headless"
BINARY_JSON = ROOT / "artifacts/tools/binary.json"
READY = ROOT / "artifacts/tools/gpu-review-ready"
ARTIFACTS = ROOT / "artifacts/0039-native-tilemaps"
LEDGER = ARTIFACTS / "ledger.jsonl"
CALLS_LOG = ARTIFACTS / "engine-calls.jsonl"
PACKET_FRAMES, PACKET_RAW, CALL_FRAMES = 128, 256 << 20, 16
COMMITTED_ART = PACKET / "art"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify_binary():
    expected = json.loads(BINARY_JSON.read_text(encoding="utf-8"))["sha256"]
    actual = sha(CLI)
    if actual != expected:
        raise SystemExit(f"binary sha256 {actual} != binary.json {expected}")
    return actual


def engine(*args, check=True, stdin=None, label=None):
    before = verify_binary()
    argv = [str(CLI), *[str(a) for a in args]]
    result = subprocess.run(argv, capture_output=True, text=True, cwd=ROOT, input=stdin)
    after = verify_binary()
    with CALLS_LOG.open("a", encoding="utf-8") as log:
        log.write(json.dumps({"argv": [str(Path(a).relative_to(ROOT)) if a.startswith(str(ROOT)) else a
                                       for a in argv[1:]], "exit": result.returncode, "label": label,
                              "sha256_before": before, "sha256_after": after}) + "\n")
    if before != after:
        raise SystemExit("binary changed during a call")
    if check and result.returncode != 0:
        raise SystemExit(f"{argv[1:4]} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}")
    return result


def rel(path):
    return str(Path(path).resolve().relative_to(ROOT))


class Project:
    def __init__(self, directory, stem):
        self.dir = Path(directory)
        self.file = self.dir / f"{stem}.incant.json"
        self.journal = self.dir / f"{stem}.incant.journal.jsonl"

    def authored(self):
        return {p.name: sha(p) for p in (self.file, self.journal) if p.exists()}

    def rpc(self, requests, label=None):
        result = engine("rpc", self.file, "--journal", self.journal, label=label,
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

    def import_(self, source, usage=None, expect_error=False):
        prior, cache = self.authored(), self.cache_files()
        args = ["import", self.file, source] + (["--texture-usage", usage] if usage else [])
        result = engine(*args, check=not expect_error, label=f"import {source}")
        entry = {"source": source, "usage": usage, "exit": result.returncode,
                 "stderr": result.stderr.strip()[-20000:], "authored_unchanged": prior == self.authored(),
                 "new_cache_files": sorted(set(self.cache_files()) - set(cache))}
        if result.returncode == 0:
            reply = json.loads(result.stdout)
            entry.update(asset=reply["asset"], changed=reply["changed"], cache_hit=reply["cache_hit"],
                         details=reply["details"], revision=reply.get("revision"),
                         diagnostics=reply.get("diagnostics", []))
        return entry

    def fingerprints(self):
        return {aid: a["sha256"] for aid, a in self.document()["assets"].items()}

    def cache_files(self):
        cache = self.dir / ".incant/cache"
        return sorted(rel(p) for p in cache.rglob("*") if p.is_file()) if cache.exists() else []

    def validate(self, check=True):
        result = engine("validate", self.file, check=check, label="validate")
        return {"exit": result.returncode, "reply": json.loads(result.stdout) if result.stdout.strip() else None,
                "stderr": result.stderr.strip()[-4000:]}

    def history(self):
        return self.rpc([{"id": 1, "method": "history.read"}], label="history.read")[0]["result"]


def projects(run):
    return {"tile": Project(run / "tile", "tile"), "control-s0": Project(run / "control-s0", "control"),
            "control-s1": Project(run / "control-s1", "control")}


def manifest_of(run):
    return json.loads((run / "art/manifest.json").read_text(encoding="utf-8"))


def write_json(path, value):
    path = Path(path)
    if path.exists():
        raise SystemExit(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1) + "\n", encoding="utf-8")


def create_entities(project, entities, description):
    scene_id = project.scene_id()
    request = {"id": 2, "method": "command.execute", "params": {
        "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
        "expected_revision": project.revision(), "description": description}}
    replies = project.rpc([request, {"id": 3, "method": "project.save"}], label="create entities")
    return {"scene_id": scene_id, "transaction_id": replies[0]["result"]["transaction_id"],
            "revision": replies[0]["result"]["revision"], "entities": len(entities),
            "request_sha256": hashlib.sha256(json.dumps(request, sort_keys=True).encode()).hexdigest()}


# ------------------------------------------------------------------- build --

def build(run):
    if run.exists():
        raise SystemExit("run directory already exists; choose a new one")
    run.mkdir(parents=True)
    start = verify_binary()
    subprocess.run([sys.executable, "-I", "-B", str(HELPERS / "tile_art.py"), str(run / "art")], check=True,
                   cwd=ROOT, capture_output=True)
    manifest = manifest_of(run)
    committed = json.loads((COMMITTED_ART / "manifest.json").read_text(encoding="utf-8")) \
        if (COMMITTED_ART / "manifest.json").exists() else None
    record = {"binary_sha256": start, "art_manifest_sha256": sha(run / "art/manifest.json"),
              "art_matches_committed": committed is not None and committed["files"] == manifest["files"]}
    art = run / "art"

    tile = projects(run)["tile"]
    (tile.dir / "art").mkdir(parents=True)
    engine("init", tile.file, "--name", "Tilemap review", "--entities", "0", label="init tile")
    for name in ("tiles.png", "plates.png", "badge.png"):
        shutil.copyfile(art / name, tile.dir / "art" / name)
    shutil.copyfile(art / "ballast-v1.png", tile.dir / "art/ballast.png")
    shutil.copyfile(art / "tiles-v1.atlas.json", tile.dir / "art/tiles.atlas.json")
    imports = {"atlas-v1": tile.import_("art/tiles.atlas.json", usage="color"),
               "plates": tile.import_("art/plates.png", usage="color"),
               "badge": tile.import_("art/badge.png", usage="color")}
    ids = {"atlas": imports["atlas-v1"]["asset"]["id"], "plates": imports["plates"]["asset"]["id"],
           "badge": imports["badge"]["asset"]["id"]}
    record["atlas_ids"] = {"asset": ids["atlas"], "descriptor": manifest["atlas_id"],
                           "layout": imports["atlas-v1"]["details"]["atlas"].get("id")}
    s0 = ts.build("s0")
    ts.write_description(run / "scene-s0.json", s0)
    ts.write_description(run / "scene-s1.json", ts.build("s1"))
    entities = ts.tile_entities(s0, ts.Assets("tile", ids, manifest))
    write_json(run / "entities-tile-s0.json", entities)
    record["tile"] = {"imports": imports, "ids": ids,
                      "create": create_entities(tile, entities, "Tilemap review scene s0 (handoff 0039)"),
                      "validate": tile.validate(), "authored": tile.authored(), "fingerprints": tile.fingerprints(),
                      "cache_files": tile.cache_files()}

    for state in ("s0", "s1"):
        control = projects(run)[f"control-{state}"]
        (control.dir / "art").mkdir(parents=True)
        engine("init", control.file, "--name", f"Tilemap control {state}", "--entities", "0",
               label=f"init control-{state}")
        cimports = {}
        for name in ("tiles.png", "plates.png", "badge.png"):
            shutil.copyfile(art / name, control.dir / "art" / name)
            cimports[name] = control.import_(f"art/{name}", usage="color")
        cids = {"tiles": cimports["tiles.png"]["asset"]["id"], "plates": cimports["plates.png"]["asset"]["id"],
                "badge": cimports["badge.png"]["asset"]["id"]}
        scene = ts.build(state)
        centities = ts.control_entities(scene, ts.Assets("control", cids, manifest))
        write_json(run / f"entities-control-{state}.json", centities)
        record[f"control-{state}"] = {
            "imports": cimports, "ids": cids,
            "create": create_entities(control, centities, f"Tilemap control {state} (handoff 0039)"),
            "validate": control.validate(), "authored": control.authored(),
            "fingerprints": control.fingerprints()}
    record["binary_sha256_end"] = verify_binary()
    write_json(run / "build.json", record)
    print(json.dumps({"art_matches_committed": record["art_matches_committed"],
                      "tile_entities": len(entities), "atlas_frames": len(imports["atlas-v1"]["details"]["atlas"]
                                                                          ["frames"]),
                      "control_entities": {s: record[f"control-{s}"]["create"]["entities"] for s in ("s0", "s1")}}))


# ------------------------------------------------------------------- shoot --

def ledger_used():
    frames = raw = 0
    if LEDGER.exists():
        for line in LEDGER.read_text(encoding="utf-8").splitlines():
            entry = json.loads(line)
            frames += entry["frames"]
            raw += entry["raw"]
    return frames, raw


def shoot(run, project_name, state, views):
    if not READY.exists():
        raise SystemExit("artifacts/tools/gpu-review-ready is absent; GPU captures are not authorized yet")
    unknown = [v for v in views if v not in ts.VIEWS]
    if unknown or not views:
        raise SystemExit(f"unknown views {unknown}")
    plan = [(v, *ts.VIEWS[v]) for v in views]
    frames, raw = len(plan), sum(w * h * 4 for _, _, (w, h) in plan)
    used_frames, used_raw = ledger_used()
    if frames > CALL_FRAMES:
        raise SystemExit(f"{frames} frames exceed the {CALL_FRAMES}-frame invocation bound")
    if used_frames + frames > PACKET_FRAMES or used_raw + raw > PACKET_RAW:
        raise SystemExit(f"budget: need {frames} frames / {raw / 2**20:.2f} MiB; used {used_frames} / "
                         f"{used_raw / 2**20:.2f} MiB of {PACKET_FRAMES} / {PACKET_RAW >> 20} MiB")
    project = projects(run)[project_name]
    folder = run / "states" / f"{state}--{project_name}"
    if folder.exists():
        raise SystemExit(f"{folder} exists; states are never overwritten")
    folder.mkdir(parents=True)
    scene = json.loads((run / "scene-s0.json").read_text(encoding="utf-8"))
    record = {"project": project_name, "state": state, "authored_before": project.authored(),
              "fingerprints": project.fingerprints(), "shots": {}}
    for view, camera, (w, h) in plan:
        png = folder / f"{view}-{w}x{h}.png"
        camera_id = scene["ids"][f"camera:{camera}"]
        result = engine("screenshot", project.file, png, "--camera", camera_id, "--width", w, "--height", h,
                        check=False, label=f"screenshot {project_name} {state} {view}")
        entry = {"camera": camera, "camera_id": camera_id, "exit": result.returncode, "written": png.exists(),
                 "stderr": result.stderr.strip()[-2000:]}
        if png.exists():
            with LEDGER.open("a", encoding="utf-8") as f:
                f.write(json.dumps({"run": run.name, "path": rel(png), "frames": 1, "raw": w * h * 4}) + "\n")
            reply = json.loads(result.stdout)
            entry.update(sha256=sha(png), adapter=reply.get("adapter"), geometry=reply.get("geometry"),
                         shading=reply.get("shading"))
        record["shots"][view] = entry
    record["authored_after"] = project.authored()
    record["ledger_after"] = dict(zip(("frames", "raw_bytes"), ledger_used()))
    write_json(folder / "record.json", record)
    print(json.dumps({"state": state, "project": project_name, "shots": {k: v["exit"] for k, v in
                                                                         record["shots"].items()},
                      "ledger": record["ledger_after"]}))


# -------------------------------------------------------------------- steps --

def set_maps(project, scene, names, manifest, ids, description):
    by_name = {item["name"]: item for item in scene.items}
    assets = ts.Assets("tile", ids, manifest)
    commands = [{"op": "set_component", "scene_id": project.scene_id(), "entity_id": ts.eid(scene, n),
                 "component": "Tilemap", "value": ts.tilemap_value(by_name[n], assets)} for n in names]
    request = {"id": 2, "method": "command.execute", "params": {
        "commands": commands, "expected_revision": project.revision(), "description": description}}
    replies = project.rpc([request, {"id": 3, "method": "project.save"}], label="s1 edit")
    return {"request": request, "transaction_id": replies[0]["result"]["transaction_id"],
            "revision": replies[0]["result"]["revision"]}


def history_step(project, method):
    replies = project.rpc([{"id": 1, "method": method}, {"id": 2, "method": "project.save"}], label=method)
    return {"method": method, "transaction_id": replies[0]["result"]["transaction_id"],
            "revision": replies[0]["result"]["revision"]}


def step(run, action):
    project = projects(run)["tile"]
    build_record = json.loads((run / "build.json").read_text(encoding="utf-8"))
    ids = build_record["tile"]["ids"]
    manifest = manifest_of(run)
    steps = run / "steps"
    steps.mkdir(exist_ok=True)
    out = steps / f"{len(list(steps.glob('*.json'))) + 1:02d}-{action}.json"
    if any(p.name.endswith(f"-{action}.json") for p in steps.glob("*.json")):
        raise SystemExit(f"step {action} already ran in this run")
    art = run / "art"
    record = {"action": action, "authored_before": project.authored(), "fingerprints_before": project.fingerprints(),
              "cache_before": project.cache_files()}
    if action == "s1-edit":
        record["edit"] = set_maps(project, ts.build("s1"), ["A1", "A2", "B1"], manifest, ids,
                                  "Tile review s1: cells, map size and layer order (handoff 0039)")
        record["validate"] = project.validate()
    elif action in ("undo", "redo", "import-undo", "import-redo"):
        record["history"] = history_step(project, "history.undo" if action.endswith("undo") else "history.redo")
    elif action == "repack":
        shutil.copyfile(art / "tiles-v2.atlas.json", project.dir / "art/tiles.atlas.json")
        shutil.copyfile(art / "ballast-v2.png", project.dir / "art/ballast.png")
        record["import"] = project.import_("art/tiles.atlas.json")
        record["validate"] = project.validate()
    elif action == "reject":
        shutil.copyfile(art / "tiles-fail-missing.atlas.json", project.dir / "art/tiles.atlas.json")
        record["import"] = project.import_("art/tiles.atlas.json", expect_error=True)
        record["authored_unchanged"] = record["authored_before"] == project.authored()
        shutil.copyfile(art / "tiles-v2.atlas.json", project.dir / "art/tiles.atlas.json")
        record["descriptor_restored_sha256"] = sha(project.dir / "art/tiles.atlas.json")
        record["validate"] = project.validate()
    elif action == "source-free":
        removed = sorted(rel(p) for p in (project.dir / "art").iterdir() if p.is_file())
        for path in removed:
            (ROOT / path).unlink()
        record["removed"] = removed
        record["validate"] = project.validate()
        record["reimport_without_sources"] = project.import_("art/tiles.atlas.json", expect_error=True)
    else:
        raise SystemExit(f"unknown action {action}")
    record.update(authored_after=project.authored(), fingerprints_after=project.fingerprints(),
                  cache_after=project.cache_files(), history_after=project.history())
    write_json(out, record)
    summary = {"action": action, "fingerprints_changed": record["fingerprints_before"] != record["fingerprints_after"],
               "authored_changed": record["authored_before"] != record["authored_after"]}
    if "import" in record:
        summary["import_exit"] = record["import"]["exit"]
    print(json.dumps(summary))


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    b = sub.add_parser("build")
    b.add_argument("run", type=Path)
    s = sub.add_parser("shoot")
    s.add_argument("run", type=Path)
    s.add_argument("project", choices=["tile", "control-s0", "control-s1"])
    s.add_argument("state")
    s.add_argument("views", help="comma-separated view names, e.g. pp1-A,of25-B")
    t = sub.add_parser("step")
    t.add_argument("run", type=Path)
    t.add_argument("action")
    args = parser.parse_args()
    if not CLI.exists():
        raise SystemExit(f"missing {CLI}")
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    run = args.run.resolve()
    if ARTIFACTS.resolve() not in run.parents:
        raise SystemExit("runs live under artifacts/0039-native-tilemaps/")
    if args.command == "build":
        build(run)
    elif args.command == "shoot":
        shoot(run, args.project, args.state, args.views.split(","))
    else:
        step(run, args.action)


if __name__ == "__main__":
    main()
