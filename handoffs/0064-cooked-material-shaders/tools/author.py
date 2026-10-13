#!/usr/bin/env python3
"""CPU-only fixture authoring for handoff 0064 through the public command bus.

Adapted from handoff 0062's committed tools/author.py (read-only reuse). Projects
are built only with the frozen incant_headless `init`, `import`, `rpc`
(`project.read`, `command.execute`, `project.save`) and `validate` commands;
scenes are cooked with the frozen cook_scene example. This script never writes
a project or journal file itself and never starts a renderer. Source art files
are copied into each project's art/ directory; the R board's panel image is
then replaced on disk and reimported through `incant_headless import`. Every
engine call is appended to <run>/engine-calls.jsonl with the executable SHA-256
checked before and after the call.

  python3 -I -B author.py build <new-run-dir> [shadow]   # shadow: only board S (run r2)
  python3 -I -B author.py source-free <run-dir> <board>   # copy privately, then delete sources
"""

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import boards as bd  # noqa: E402
import common as c  # noqa: E402


def tools():
    frozen = c.read_json(c.EVIDENCE / "build" / "tools.json")["frozen"]
    return {name: (Path(v["path"]), v["sha256"]) for name, v in frozen.items()}


def verify():
    actual = {name: c.sha(path) for name, (path, _) in tools().items()}
    for name, (_, expected) in tools().items():
        if actual[name] != expected:
            raise SystemExit(f"{name} hash {actual[name]} != frozen {expected}")
    return actual


class Run:
    def __init__(self, root):
        self.root = Path(root)
        self.calls = self.root / "engine-calls.jsonl"

    def call(self, tool, *args, stdin=None, label=None, cwd=None):
        c.check_grant()
        before = verify()
        argv = [str(tools()[tool][0]), *[str(a) for a in args]]
        started = c.now()
        result = subprocess.run(argv, capture_output=True, text=True, input=stdin, cwd=cwd or self.root,
                                timeout=120, env=c.run_env())
        after = verify()
        with self.calls.open("a") as log:
            log.write(json.dumps({"tool": tool, "argv": argv[1:], "cwd": str(cwd or self.root),
                                  "exit": result.returncode, "label": label, "started": started,
                                  "ended": c.now(), "sha256_before": before[tool], "sha256_after": after[tool],
                                  "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
                                  "stdout_head": result.stdout[:600], "stderr": result.stderr[-2000:]}) + "\n")
        if before != after or result.returncode != 0:
            raise SystemExit(f"{tool} {argv[1:3]} failed ({result.returncode}): {result.stderr[-3000:]}")
        return result


class Project:
    def __init__(self, run, key):
        self.run, self.key = run, key
        self.dir = run.root / key
        self.file = self.dir / f"{key}.incant.json"
        self.journal = self.dir / f"{key}.incant.journal.jsonl"

    def rpc(self, requests, label):
        result = self.run.call("incant_headless", "rpc", self.file, "--journal", self.journal, label=label,
                               stdin="".join(json.dumps(r) + "\n" for r in requests))
        replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        if len(replies) != len(requests) or any("error" in r for r in replies):
            raise SystemExit(f"RPC failure: {replies}")
        return replies

    def read(self):
        return self.rpc([{"id": 1, "method": "project.read"}], "project.read")[0]["result"]

    def import_source(self, source, usage=None, label=None):
        extra = ["--texture-usage", usage] if usage else []
        reply = json.loads(self.run.call("incant_headless", "import", self.file, source, *extra,
                                         label=label or f"import {self.key} {source}", cwd=self.dir).stdout)
        return {"asset": reply["asset"], "changed": reply.get("changed"),
                "diagnostics": reply.get("diagnostics", []), "revision": reply.get("revision")}

    def cook(self, scene_id, name):
        cooked = self.run.root / "cooked" / f"{name}.scene"
        cooked.parent.mkdir(exist_ok=True)
        if cooked.exists():
            raise SystemExit(f"{cooked} exists; cooked scenes are never replaced")
        result = self.run.call("cook_scene", self.file, scene_id, cooked, label=f"cook {name}")
        data = cooked.read_bytes()
        return {"path": str(cooked), "sha256": c.sha(cooked), "bytes": len(data),
                "version": int.from_bytes(data[8:12], "little"), "stdout": result.stdout.strip()}

    def snapshot(self):
        cache = self.dir / ".incant/cache"
        return {"authored": {p.name: c.sha(p) for p in (self.file, self.journal) if p.exists()},
                "cache": {str(p.relative_to(self.dir)): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}


def files_for(board, manifest):
    needed = set()
    for model in board.models():
        needed.add(f"{model}.gltf")
        needed.add(f"{manifest['models'][model]['mesh']}.bin")
        needed.update(manifest["models"][model]["images"])
    needed.update(board.textures())
    return sorted(needed)


def build(root, boards=None):
    boards = boards or bd.BOARDS
    root = Path(root)
    if root.exists():
        raise SystemExit("run directory exists; choose a new one")
    root.mkdir(parents=True)
    run = Run(root)
    started = verify()
    art = root / "art"
    subprocess.run([sys.executable, "-I", "-B", str(TOOLS / "art.py"), str(art)], check=True, capture_output=True)
    manifest = c.read_json(art / "manifest.json")
    record = {"started": c.now(), "tools": started, "art_manifest_sha256": c.sha(art / "manifest.json"),
              "boards": {}}
    (root / "boards").mkdir()
    for key, make in boards.items():
        board = make()
        project = Project(run, key)
        project.dir.mkdir()
        run.call("incant_headless", "init", project.file, "--name", f"0064 board {key}", "--entities", "0",
                 label=f"init {key}")
        (project.dir / "art").mkdir()
        for name in files_for(board, manifest):
            shutil.copyfile(art / name, project.dir / "art" / name)
        imports = {}
        for model in board.models():
            imports[model] = project.import_source(f"art/{model}.gltf")
        for texture in board.textures():
            imports[texture] = project.import_source(f"art/{texture}", usage="color")
        ids = {name: entry["asset"]["id"] for name, entry in imports.items()}
        state = project.read()
        scene_id = next(iter(state["project"]["scenes"]))
        entities = bd.entity_values(board, ids)
        replies = []
        if entities:
            request = {"id": 2, "method": "command.execute", "params": {
                "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
                "expected_revision": state["revision"], "description": f"0064 board {key} fixture"}}
            replies = project.rpc([request, {"id": 3, "method": "project.save"}], f"create {key}")
        validate = run.call("incant_headless", "validate", project.file, label=f"validate {key}")
        (root / "boards" / f"{key}.json").write_text(json.dumps(bd.describe(board), indent=1) + "\n")
        (root / "boards" / f"{key}-entities.json").write_text(json.dumps(entities, indent=1) + "\n")
        entry = {"project": str(project.file), "scene_id": scene_id, "imports": imports, "entities": len(entities),
                 "transaction_id": replies[0]["result"]["transaction_id"] if replies else None,
                 "revision": replies[0]["result"]["revision"] if replies else state["revision"],
                 "validate": validate.stdout.strip(),
                 "cameras": {i["name"]: bd.ulid_hex(board.id(i["name"])) for i in board.items if "camera" in i},
                 "local_lights": {n: bd.ulid_hex(board.id(n)) for n in bd.local_light_names(board)}}
        if key == "R":
            entry["cooked"] = {"R-v1": project.cook(scene_id, "R-v1")}
            entry["before_reimport"] = project.snapshot()
            panel = project.dir / "art" / "panel.png"
            shutil.copyfile(art / "panel-v2.png", panel)
            entry["panel_v2_sha256"] = c.sha(panel)
            entry["reimport"] = project.import_source("art/quad-panel.gltf", label="reimport R quad-panel")
            if entry["reimport"]["asset"]["id"] != ids["quad-panel"] or not entry["reimport"]["changed"]:
                raise SystemExit("reimport did not update the existing panel asset")
            entry["validate_after_reimport"] = run.call("incant_headless", "validate", project.file,
                                                        label="validate R after reimport").stdout.strip()
            entry["cooked"]["R-v2"] = project.cook(scene_id, "R-v2")
        else:
            entry["cooked"] = {key: project.cook(scene_id, key)}
        entry.update(project.snapshot())
        record["boards"][key] = entry
    record["tools_end"] = verify()
    record["finished"] = c.now()
    c.write_new_json(root / "build.json", record)
    print(json.dumps({k: {"entities": v["entities"], "cooked": {n: (x["bytes"], x["version"])
                                                                for n, x in v["cooked"].items()},
                          "validate": v["validate"][:160]} for k, v in record["boards"].items()}, indent=1))


def source_free(root, key):
    root = Path(root)
    marker = root / f"source-free-{key}.json"
    if marker.exists():
        raise SystemExit("source-free already ran for this board")
    project = Project(Run(root), key)
    record_in = c.read_json(root / "build.json")["boards"][key]
    cooked = [Path(v["path"]) for v in record_in["cooked"].values()]
    cache = project.dir / ".incant/cache"
    kept_before = {**{str(p): c.sha(p) for p in cooked},
                   **{str(p): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
    sources = [project.file, project.journal] + sorted((project.dir / "art").iterdir())
    private = root / "private-fixture" / key
    removed = {}
    for path in sources:
        target = private / path.relative_to(project.dir)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        if c.sha(target) != c.sha(path):
            raise SystemExit(f"private copy differs for {path}")
        removed[str(path.relative_to(root))] = c.sha(path)
    for path in sources:
        path.unlink()
    (project.dir / "art").rmdir()
    kept_after = {**{str(p): c.sha(p) for p in cooked},
                  **{str(p): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
    remaining = sorted(str(p.relative_to(project.dir)) for p in project.dir.rglob("*") if p.is_file())
    c.write_new_json(marker, {
        "board": key, "at": c.now(), "removed": removed, "private_copy": str(private),
        "kept_unchanged": kept_before == kept_after, "kept": kept_after, "remaining_in_project": remaining,
        "no_authoring_or_art_left": all(r.startswith(".incant/") for r in remaining)})
    print(json.dumps({"removed": len(removed), "kept_unchanged": kept_before == kept_after,
                      "remaining": remaining}, indent=1))


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "build":
        build(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "build" and sys.argv[3] == "shadow":
        build(sys.argv[2], bd.SHADOW_BOARDS)
    elif len(sys.argv) == 4 and sys.argv[1] == "source-free" and sys.argv[3] in bd.BOARDS:
        source_free(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
