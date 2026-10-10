#!/usr/bin/env python3
"""Build through public commands, then save/resume a strict TypeScript game. No GPU."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/incant_headless")
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()

    def run(*args):
        return json.loads(subprocess.check_output([str(binary), *map(str, args)], text=True, cwd=ROOT))

    project = out / "game.incant.json"
    journal = out / "history.jsonl"
    run("init", project, "--name", "Save continuity", "--entities", 0)
    sid = next(iter(json.loads(project.read_text())["scenes"]))
    player, pickup, spawned = (f"{i:026d}" for i in [10, 11, 12])
    commands = [{"op": "create_entity", "scene_id": sid, "entity": {
        "id": eid, "name": name, "parent": None, "provenance": None, "components": {
            "Transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
            "Velocity": {"linear": [2 if eid == player else 0, 0, 0]}}}}
        for eid, name in [(player, "Player"), (pickup, "Pickup")]]
    requests = [
        {"id": 1, "method": "command.execute", "params": {
            "commands": commands, "expected_revision": 0, "description": "Create save-continuity game"}},
        {"id": 2, "method": "project.save"},
    ]
    replies = subprocess.check_output([str(binary), "rpc", str(project), "--journal", str(journal)],
                                     input="".join(json.dumps(r) + "\n" for r in requests), text=True, cwd=ROOT)
    replies = [json.loads(line) for line in replies.splitlines()]
    assert len(replies) == 2 and all("error" not in r for r in replies), replies
    source = out / "behavior.ts"
    source.write_text("import type { ScriptApi } from " + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n" +
        "const scene = " + json.dumps(sid) + ";\nconst pickup = " + json.dumps(pickup) + ";\nconst spawned = " +
        json.dumps(spawned) + ";\n" + """export default defineBehavior<{ticks:number,inventory:string[]}>({
        initialState:{ticks:0,inventory:[]}, update(api:ScriptApi,_dt,state){
            state.ticks++;
            if(state.ticks===12){
                state.inventory.push(pickup);
                api.command({op:'delete_entity',scene_id:scene,entity_id:pickup});
                api.command({op:'create_entity',scene_id:scene,entity:{id:spawned,name:'Spawned',parent:null,
                    provenance:null,components:{Transform:{translation:[0,1,0],rotation:[0,0,0,1],scale:[1,1,1]},
                    Velocity:{linear:[0,0,0.5]}}}});
            }
            api.log('tick '+state.ticks);
        }
    });\n""")
    subprocess.run(["node", str(ROOT / "node_modules/typescript/bin/tsc"), "--strict", "--noEmit", "--target", "ES2022",
                    "--module", "ESNext", "--moduleResolution", "bundler", str(source)], check=True, cwd=ROOT)
    compiled = out / "behavior.js"
    subprocess.run(["node", str(ROOT / "tools/build_script.mjs"), str(source), str(compiled)], check=True, cwd=ROOT)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    first = run("play", project, "--ticks", 19, "--compiled-script", compiled, "--save-output", out / "one.save.json")
    resumed = run("play", project, "--ticks", 41, "--compiled-script", compiled, "--load-save", out / "one.save.json",
                  "--save-output", out / "two.save.json", "--log-output", out / "continued.jsonl")
    uninterrupted = run("play", project, "--ticks", 60, "--compiled-script", compiled)
    reopened = run("play", project, "--ticks", 0, "--compiled-script", compiled, "--load-save", out / "two.save.json")
    assert resumed["state"] == uninterrupted["state"] == reopened["state"]
    assert resumed["script_state"] == uninterrupted["script_state"] == reopened["script_state"]
    assert resumed["script_state"] == {"ticks": 60, "inventory": [pickup]}
    assert pickup not in resumed["state"]["entities"] and spawned in resumed["state"]["entities"]
    assert resumed["logs"] == uninterrupted["logs"][19:]
    assert resumed["start_tick"] == 19 and resumed["ticks"] == 41
    assert all(run["frames"] == [] and run["adapter"] is None for run in [first, resumed, uninterrupted, reopened])
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    result = {"passed": True, "strict_typescript": True, "rendering": False, "steps": [19, 41],
              "full_state_equal": True, "nested_state_and_runtime_spawn_delete_preserved": True,
              "resumed_logs_equal_to_uninterrupted_suffix": True, "second_save_reopened": True,
              "author_files_unchanged": before, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    (out / "runs.json").write_text(json.dumps({"first": first, "resumed": resumed, "uninterrupted": uninterrupted,
                                              "reopened": reopened}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
