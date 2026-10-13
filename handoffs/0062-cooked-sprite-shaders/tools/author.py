#!/usr/bin/env python3
"""CPU-only fixture authoring for handoff 0062 through the public command bus.

Projects are built only with the frozen incant_headless `init`, `import`,
`rpc` (`project.read`, `command.execute`, `project.save`) and `validate`
commands; scenes are cooked with the frozen cook_scene example. This script
never writes a project or journal file itself and never starts a renderer.
Every engine call is appended to <run>/engine-calls.jsonl with the executable
SHA-256 checked before and after the call.

  python3 -I -B author.py build <new-run-dir>
  python3 -I -B author.py source-free <run-dir> <board>   # copy, then delete sources
"""

import datetime
import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TOOLS = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS))
import scene as sc  # noqa: E402

EVIDENCE = Path("/Users/andreyyurlov/.codex/incant-evidence/0062")
BUILD = EVIDENCE / "build"
SOURCES = {"atlas": "sprites.atlas.json", "sheet": "sheet.png", "strip-h": "strip-h.png",
           "strip-v": "strip-v.png", "occluder": "occluder.gltf"}
WIDTH, HEIGHT = 960, 540
CASES = {  # case -> (board, camera, ticks)
    "A-pp": ("A", "camera-pp", [0]), "A-ortho": ("A", "camera-ortho", [0]),
    "A-persp": ("A", "camera-persp", [0]), "W-pp": ("W", "camera-pp", [0]),
    "W-ortho": ("W", "camera-ortho", [0]), "B-pp": ("B", "camera-pp", [0, sc.FINAL_TICK]),
    "B-ortho": ("B", "camera-ortho", [0, sc.FINAL_TICK]),
}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def tools():
    found = {}
    for record in ("tools-cook", "tools-headless"):
        found.update(json.loads((BUILD / f"{record}.json").read_text())["frozen"])
    return {name: (Path(v["path"]), v["sha256"]) for name, v in found.items()}


def verify():
    actual = {name: sha(path) for name, (path, _) in tools().items()}
    for name, (_, expected) in tools().items():
        if actual[name] != expected:
            raise SystemExit(f"{name} hash {actual[name]} != frozen {expected}")
    return actual


class Run:
    def __init__(self, root):
        self.root = Path(root)
        self.calls = self.root / "engine-calls.jsonl"

    def call(self, tool, *args, stdin=None, label=None, cwd=None):
        before = verify()
        argv = [str(tools()[tool][0]), *[str(a) for a in args]]
        started = now()
        result = subprocess.run(argv, capture_output=True, text=True, input=stdin, cwd=cwd or self.root,
                                timeout=120)
        after = verify()
        with self.calls.open("a") as log:
            log.write(json.dumps({"tool": tool, "argv": argv[1:], "exit": result.returncode, "label": label,
                                  "started": started, "ended": now(), "sha256_before": before[tool],
                                  "sha256_after": after[tool],
                                  "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
                                  "stderr": result.stderr[-2000:]}) + "\n")
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


def build(root):
    root = Path(root)
    if root.exists():
        raise SystemExit("run directory exists; choose a new one")
    root.mkdir(parents=True)
    run = Run(root)
    started = verify()
    subprocess.run([sys.executable, "-I", "-B", str(TOOLS / "art.py"), str(root / "art")], check=True,
                   capture_output=True)
    manifest = json.loads((root / "art/manifest.json").read_text())
    record = {"started": now(), "tools": started, "art_manifest_sha256": sha(root / "art/manifest.json"),
              "boards": {}}
    for key, make in sc.BOARDS.items():
        board = make()
        project = Project(run, key)
        project.dir.mkdir()
        run.call("incant_headless", "init", project.file, "--name", f"0062 board {key}", "--entities", "0",
                 label=f"init {key}")
        art = project.dir / "art"
        art.mkdir()
        for path in sorted((root / "art").iterdir()):
            if path.name != "manifest.json":
                shutil.copyfile(path, art / path.name)
        needed = sc.textures_used(board) + (["occluder"] if any(i["kind"] == "mesh" for i in board.items) else [])
        imports = {}
        for name in needed:
            usage = [] if name == "occluder" else ["--texture-usage", "color"]
            reply = json.loads(run.call("incant_headless", "import", project.file, f"art/{SOURCES[name]}", *usage,
                                        label=f"import {key} {name}", cwd=project.dir).stdout)
            imports[name] = {"asset": reply["asset"], "changed": reply.get("changed"),
                             "diagnostics": reply.get("diagnostics", []), "revision": reply.get("revision")}
        ids = {name: entry["asset"]["id"] for name, entry in imports.items()}
        state = project.read()
        scene_id = next(iter(state["project"]["scenes"]))
        entities = sc.entity_values(board, ids, manifest)
        request = {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities],
            "expected_revision": state["revision"], "description": f"0062 board {key} fixture"}}
        replies = project.rpc([request, {"id": 3, "method": "project.save"}], f"create {key}")
        validate = run.call("incant_headless", "validate", project.file, label=f"validate {key}")
        cooked = root / "cooked" / f"{key}.scene"
        cooked.parent.mkdir(exist_ok=True)
        cook = run.call("cook_scene", project.file, scene_id, cooked, label=f"cook {key}")
        data = cooked.read_bytes()
        (root / "boards").mkdir(exist_ok=True)
        (root / "boards" / f"{key}.json").write_text(json.dumps(sc.describe(board), indent=1) + "\n")
        (root / "boards" / f"{key}-entities.json").write_text(json.dumps(entities, indent=1) + "\n")
        cache = project.dir / ".incant/cache"
        record["boards"][key] = {
            "project": str(project.file), "scene_id": scene_id, "imports": imports, "entities": len(entities),
            "transaction_id": replies[0]["result"]["transaction_id"], "revision": replies[0]["result"]["revision"],
            "request_sha256": hashlib.sha256(json.dumps(request, sort_keys=True).encode()).hexdigest(),
            "validate": validate.stdout.strip(), "cook_stdout": cook.stdout.strip(),
            "cooked": {"path": str(cooked), "sha256": sha(cooked), "bytes": len(data),
                       "version": int.from_bytes(data[8:12], "little")},
            "authored": {p.name: sha(p) for p in (project.file, project.journal)},
            "cache": {str(p.relative_to(project.dir)): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()},
            "cameras": {i["name"]: sc.ulid_hex(board.id(i["name"])) for i in board.items if i["kind"] == "camera"},
        }
    requests = root / "requests"
    requests.mkdir()
    for case, (key, camera, ticks) in CASES.items():
        (requests / f"{case}.json").write_text(json.dumps({"width": WIDTH, "height": HEIGHT, "ticks": ticks}) + "\n")
    (root / "empty-models").mkdir()
    record["cases"] = {case: {"board": key, "camera": camera, "ticks": ticks,
                              "camera_hex": record["boards"][key]["cameras"][camera],
                              "request_sha256": sha(requests / f"{case}.json")}
                       for case, (key, camera, ticks) in CASES.items()}
    record["tools_end"] = verify()
    record["finished"] = now()
    with open(root / "build.json", "x") as out:
        json.dump(record, out, indent=1)
        out.write("\n")
    print(json.dumps({k: {"entities": v["entities"], "cooked_version": v["cooked"]["version"],
                          "cooked_bytes": v["cooked"]["bytes"], "validate": v["validate"][:200]}
                      for k, v in record["boards"].items()}, indent=1))


def source_free(root, key):
    root = Path(root)
    marker = root / f"source-free-{key}.json"
    if marker.exists():
        raise SystemExit("source-free already ran for this board")
    project = Project(Run(root), key)
    cooked = root / "cooked" / f"{key}.scene"
    cache = project.dir / ".incant/cache"
    kept_before = {str(cooked): sha(cooked),
                   **{str(p): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
    sources = [project.file, project.journal] + sorted((project.dir / "art").iterdir())
    private = root / "private-fixture" / key
    removed = {}
    for path in sources:
        target = private / path.relative_to(project.dir)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
        if sha(target) != sha(path):
            raise SystemExit(f"private copy differs for {path}")
        removed[str(path.relative_to(root))] = sha(path)
    for path in sources:
        path.unlink()
    (project.dir / "art").rmdir()
    kept_after = {str(cooked): sha(cooked),
                  **{str(p): sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
    remaining = sorted(str(p.relative_to(project.dir)) for p in project.dir.rglob("*") if p.is_file())
    record = {"board": key, "at": now(), "removed": removed, "private_copy": str(private),
              "kept_unchanged": kept_before == kept_after, "kept": kept_after, "remaining_in_project": remaining,
              "no_authoring_or_art_left": all(r.startswith(".incant/cache/") for r in remaining)}
    with open(marker, "x") as out:
        json.dump(record, out, indent=1)
        out.write("\n")
    print(json.dumps({"removed": len(removed), "kept_unchanged": record["kept_unchanged"],
                      "remaining": remaining}, indent=1))


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "build":
        build(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "source-free" and sys.argv[3] in sc.BOARDS:
        source_free(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
