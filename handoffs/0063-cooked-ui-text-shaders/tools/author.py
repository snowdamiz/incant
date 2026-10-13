#!/usr/bin/env python3
"""CPU-only fixture authoring for handoff 0063 through the public command bus.

Projects are built only with the frozen incant_headless `init`, `import`, `rpc`
(`project.read`, one atomic `command.execute`, `history.undo`, `history.redo`,
`project.save`; then a journal reopen) and `validate` commands; scenes are
cooked with the frozen cook_scene example. This script never writes a project
or journal file itself and never starts a renderer. Every engine call is
appended to <run>/engine-calls.jsonl with the executable SHA-256 checked before
and after the call.

  python3 -I -B author.py build <new-run-dir>
  python3 -I -B author.py add-case <run-dir> <case>      # supplementary request (EXTRA_CASES)
  python3 -I -B author.py source-free <run-dir> <board>   # copy, then delete sources
"""

import json
import shutil
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import boards as bd  # noqa: E402
import common as c  # noqa: E402

FONT_DIR = c.ROOT / "crates/incant_text/tests/fonts"
SELECT = [["ShiftLeft", True]] + [["ArrowLeft", True], ["ArrowLeft", False]] * len("Lovelace") + \
    [["ShiftLeft", False]]
# case -> (board, camera key, request without camera)
CASES = {
    "U-1x": ("U", "camera", {"format": "rgba8unorm-srgb", "size": [960, 540], "ui_viewport": [960, 540],
                             "steps": [{"capture": "idle"}, {"focus": bd.U_IDS["name"]},
                                       {"keys": SELECT, "capture": "select"}]}),
    "U-15x": ("U", "camera", {"format": "rgba8unorm-srgb", "size": [960, 540], "ui_viewport": [640, 360],
                              "steps": [{"focus": bd.U_IDS["cjk"]},
                                        {"preedit": ["語", [3, 3]], "capture": "preedit"}]}),
    "U-2x": ("U", "camera", {"format": "rgba8unorm-srgb", "size": [960, 540], "ui_viewport": [480, 270],
                             "steps": [{"keys": [["Tab", True]]},
                                       {"keys": [["Tab", False]], "capture": "focus"}]}),
    "U-1x-unorm": ("U", "camera", {"format": "rgba8unorm", "size": [960, 540], "ui_viewport": [960, 540],
                                   "steps": [{"capture": "idle"}]}),
    "W-ortho": ("W", "cam_ortho", {"format": "rgba8unorm-srgb", "size": [960, 540], "steps": [{"capture": "t1"}]}),
    "W-persp": ("W", "cam_persp", {"format": "rgba8unorm-srgb", "size": [960, 540],
                                   "steps": [{"capture": "t1"}] + [{}] * 59 + [{"capture": "t61"}]}),
}
# Added to run r3 after U-15x failed identically in both variants: at a 640x360 logical
# viewport the CJK field lies below the visible area, so the host rejects focusing it.
EXTRA_CASES = {
    "U-15x-name": ("U", "camera", {"format": "rgba8unorm-srgb", "size": [960, 540], "ui_viewport": [640, 360],
                                   "steps": [{"focus": bd.U_IDS["name"]},
                                             {"preedit": ["語", [3, 3]], "capture": "preedit"}]}),
}


def tools():
    frozen = c.read_json(c.EVIDENCE / "build/tools.json")["frozen"]
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
        before = verify()
        argv = [str(tools()[tool][0]), *[str(a) for a in args]]
        started = c.now()
        result = subprocess.run(argv, capture_output=True, text=True, input=stdin, cwd=cwd or self.root,
                                timeout=120, env=c.clean_env())
        after = verify()
        with self.calls.open("a") as log:
            log.write(json.dumps({"tool": tool, "argv": argv[1:], "exit": result.returncode, "label": label,
                                  "started": started, "ended": c.now(), "sha256_before": before[tool],
                                  "sha256_after": after[tool], "stdout_sha256": c.sha_bytes(result.stdout.encode()),
                                  "stderr": result.stderr[-2000:]}, ensure_ascii=False) + "\n")
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
                               stdin="".join(json.dumps(r, ensure_ascii=False) + "\n" for r in requests))
        replies = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        if len(replies) != len(requests) or any("error" in r for r in replies):
            raise SystemExit(f"RPC failure: {replies}")
        return replies


def build(root):
    c.check_lease()
    root = Path(root)
    if root.exists():
        raise SystemExit("run directory exists; choose a new one")
    root.mkdir(parents=True)
    run = Run(root)
    record = {"started": c.now(), "tools": verify(), "boards": {}}
    for key, (key_ids, make) in bd.BOARDS.items():
        project = Project(run, key)
        project.dir.mkdir()
        run.call("incant_headless", "init", project.file, "--name", f"0063 board {key}", "--entities", "0",
                 label=f"init {key}")
        art = project.dir / "art"
        art.mkdir()
        fonts, meshes, imports = [], {}, {}
        for name in bd.FONTS:
            shutil.copyfile(FONT_DIR / name, art / name)
            reply = json.loads(run.call("incant_headless", "import", project.file, f"art/{name}",
                                        label=f"import {key} {name}", cwd=project.dir).stdout)
            fonts.append(reply["asset"]["id"])
            imports[name] = reply["asset"]["id"]
        for name, color in bd.MESHES.items():
            (art / f"{name}.gltf").write_text(json.dumps(bd.cube(color)), encoding="utf-8")
            reply = json.loads(run.call("incant_headless", "import", project.file, f"art/{name}.gltf",
                                        label=f"import {key} {name}", cwd=project.dir).stdout)
            meshes[name] = reply["asset"]["id"]
            imports[f"{name}.gltf"] = reply["asset"]["id"]
        state = project.rpc([{"id": 1, "method": "project.read"}], "project.read")[0]["result"]
        scene_id = next(iter(state["project"]["scenes"]))

        def entity(name, label, components, parent=None):
            return {"op": "create_entity", "scene_id": scene_id, "entity": {
                "id": key_ids[name], "name": label, "parent": parent and key_ids[parent], "provenance": None,
                "components": components}}

        commands = make(fonts, meshes, entity)
        request = {"id": 2, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": state["revision"],
            "description": f"Author 0063 board {key}"}}
        replies = project.rpc([request, {"id": 3, "method": "project.read"}, {"id": 4, "method": "history.undo"},
                               {"id": 5, "method": "history.redo"}, {"id": 6, "method": "project.read"},
                               {"id": 7, "method": "project.save"}], f"author {key}")
        executed, authored, redone = replies[0]["result"], replies[1]["result"], replies[4]["result"]
        if executed["revision"] != state["revision"] + 1 or authored["project"] != redone["project"]:
            raise SystemExit("authoring was not one atomic, reversible transaction")
        reopened = project.rpc([{"id": 8, "method": "project.read"}], f"reopen {key}")[0]["result"]
        if reopened["project"] != redone["project"]:
            raise SystemExit("journal reopen changed the project")
        validate = run.call("incant_headless", "validate", project.file, label=f"validate {key}")
        cooked = root / "cooked" / f"{key}.scene"
        cooked.parent.mkdir(exist_ok=True)
        cook = run.call("cook_scene", project.file, scene_id, cooked, label=f"cook {key}")
        data = cooked.read_bytes()
        (root / "boards").mkdir(exist_ok=True)
        (root / "boards" / f"{key}-commands.json").write_text(
            json.dumps(commands, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
        cache = project.dir / ".incant/cache"
        record["boards"][key] = {
            "project": str(project.file), "scene_id": scene_id, "imports": imports, "commands": len(commands),
            "transaction_id": executed.get("transaction_id"), "revision": executed["revision"],
            "request_sha256": c.sha_bytes(json.dumps(request, sort_keys=True, ensure_ascii=False).encode()),
            "undo_redo_restored": True, "journal_reopened": True,
            "validate": validate.stdout.strip(), "cook_stdout": cook.stdout.strip(),
            "cooked": {"path": str(cooked), "sha256": c.sha(cooked), "bytes": len(data)},
            "authored": {p.name: c.sha(p) for p in (project.file, project.journal)},
            "art": {p.name: c.sha(p) for p in sorted(art.iterdir())},
            "cache": {str(p.relative_to(project.dir)): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()},
            "ids": key_ids,
        }
    requests = root / "requests"
    requests.mkdir()
    record["cases"] = {}
    for case, (board, camera, body) in CASES.items():
        payload = {"camera": ulid_hex(bd.BOARDS[board][0][camera]), **body}
        path = requests / f"{case}.json"
        path.write_text(json.dumps(payload, ensure_ascii=False) + "\n", encoding="utf-8")
        record["cases"][case] = {"board": board, "camera": camera, "request_sha256": c.sha(path),
                                 "captures": [s["capture"] for s in body["steps"] if "capture" in s]}
    (root / "empty").mkdir()
    record["tools_end"] = verify()
    record["finished"] = c.now()
    c.write_new_json(root / "build.json", record)
    print(json.dumps({k: {"commands": v["commands"], "cooked_bytes": v["cooked"]["bytes"],
                          "validate": v["validate"][:300]} for k, v in record["boards"].items()}, indent=1,
                     ensure_ascii=False))


def add_case(root, case):
    """Write one supplementary request for an existing run; never rewrites build.json."""
    root = Path(root)
    board, camera, body = EXTRA_CASES[case]
    path = root / "requests" / f"{case}.json"
    with open(path, "x", encoding="utf-8") as out:
        out.write(json.dumps({"camera": ulid_hex(bd.BOARDS[board][0][camera]), **body}, ensure_ascii=False) + "\n")
    c.write_new_json(root / "cases-extra" / f"{case}.json", {
        "board": board, "camera": camera, "request_sha256": c.sha(path),
        "captures": [s["capture"] for s in body["steps"] if "capture" in s], "added": c.now(),
        "reason": "U-15x focused a field outside the 640x360 logical viewport; the host refused it in both variants"})
    print(path)


def ulid_hex(text):
    digits = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
    value = 0
    for ch in text:
        value = value * 32 + digits.index(ch)
    return f"{value:032x}"


def source_free(root, key):
    c.check_lease()
    root = Path(root)
    marker = root / f"source-free-{key}.json"
    if marker.exists():
        raise SystemExit("source-free already ran for this board")
    project = Project(Run(root), key)
    cooked = root / "cooked" / f"{key}.scene"
    cache = project.dir / ".incant/cache"
    kept_before = {str(cooked): c.sha(cooked), **{str(p): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
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
    kept_after = {str(cooked): c.sha(cooked), **{str(p): c.sha(p) for p in sorted(cache.rglob("*")) if p.is_file()}}
    remaining = sorted(str(p.relative_to(project.dir)) for p in project.dir.rglob("*") if p.is_file())
    record = {"board": key, "at": c.now(), "removed": removed, "private_copy": str(private),
              "kept_unchanged": kept_before == kept_after, "kept": kept_after, "remaining_in_project": remaining,
              "no_authoring_or_art_left": all(r.startswith(".incant/") for r in remaining)}
    c.write_new_json(marker, record)
    print(json.dumps({"removed": len(removed), "kept_unchanged": record["kept_unchanged"], "remaining": remaining},
                     indent=1))


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "build":
        build(sys.argv[2])
    elif len(sys.argv) == 4 and sys.argv[1] == "add-case" and sys.argv[3] in EXTRA_CASES:
        add_case(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 4 and sys.argv[1] == "source-free" and sys.argv[3] in bd.BOARDS:
        source_free(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
