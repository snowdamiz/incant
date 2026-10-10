#!/usr/bin/env python3
"""Exercise strict TypeScript timers across real headless process restarts. No GPU."""
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

    project, journal = out / "game.incant.json", out / "history.jsonl"
    run("init", project, "--name", "Saved timer callbacks", "--entities", 0)
    sid = next(iter(json.loads(project.read_text())["scenes"]))
    entity, spawned = (f"{i:026d}" for i in [10, 11])
    transform = {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}
    requests = [
        {"id": 1, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": sid, "entity": {
                "id": entity, "name": "Timer target", "parent": None, "provenance": None,
                "components": {"Transform": transform}}}],
            "expected_revision": 0, "description": "Create timed gameplay fixture"}},
        {"id": 2, "method": "project.save"},
    ]
    replies = subprocess.check_output([str(binary), "rpc", str(project), "--journal", str(journal)],
                                     input="".join(json.dumps(r) + "\n" for r in requests), text=True, cwd=ROOT)
    replies = [json.loads(line) for line in replies.splitlines()]
    assert len(replies) == 2 and all("error" not in reply for reply in replies), replies
    source = out / "behavior.ts"
    source.write_text("import type { ScriptApi, Transform } from " +
                      json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n" +
                      "const scene = " + json.dumps(sid) + ";\nconst target = " + json.dumps(entity) +
                      ";\nconst spawned = " + json.dumps(spawned) + ";\n" + """
export default defineBehavior<{ticks:number,events:string[]}>({
  initialState:{ticks:0,events:[]},
  update(api:ScriptApi,_dt,state){
    state.ticks++;
    if(api.clock().tick===1){
      api.setTimer({id:'pulse',delay_ticks:2,interval_ticks:4,payload:{step:2}});
      api.setTimer({id:'spawn',delay_ticks:12});
      api.setTimer({id:'z-cancelled',delay_ticks:12});
    }
  },
  onTimer(api,event,state){
    const tick=api.clock().tick;
    if(event.scheduled_tick!==tick)throw Error('late callback');
    state.events.push(event.id+':'+tick);
    api.log(event.id+':'+tick);
    if(event.id==='pulse'){
      const e=api.query('Transform').find(e=>e.id===target)!;
      const t=e.components.Transform as Transform;
      const payload=event.payload as {step:number};
      t.translation[0]+=payload.step;
      api.command({op:'set_component',scene_id:scene,entity_id:target,component:'Transform',value:t});
      if(tick===19)api.cancelTimer('pulse');
    }else if(event.id==='spawn'){
      api.cancelTimer('z-cancelled');
      api.command({op:'create_entity',scene_id:scene,entity:{id:spawned,name:'Delayed spawn',parent:null,
        provenance:null,components:{Transform:{translation:[1,2,3],rotation:[0,0,0,1],scale:[1,1,1]}}}});
    }else throw Error('cancelled callback ran');
  }
});
""")
    subprocess.run(["node", str(ROOT / "node_modules/typescript/bin/tsc"), "--strict", "--noEmit", "--target", "ES2022",
                    "--module", "ESNext", "--moduleResolution", "bundler", str(source)], check=True, cwd=ROOT)
    compiled = out / "behavior.js"
    subprocess.run(["node", str(ROOT / "tools/build_script.mjs"), str(source), str(compiled)], check=True, cwd=ROOT)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    first = run("play", project, "--ticks", 10, "--compiled-script", compiled, "--save-output", out / "one.save.json")
    checkpoint = json.loads((out / "one.save.json").read_text())
    assert checkpoint["version"] == 2 and checkpoint["schedule"]["clock"]["tick"] == 10
    assert checkpoint["schedule"]["timers"]["pulse"]["due_tick"] == 11
    assert checkpoint["schedule"]["timers"]["spawn"]["due_tick"] == 13
    resumed = run("play", project, "--ticks", 20, "--compiled-script", compiled, "--load-save", out / "one.save.json",
                  "--save-output", out / "two.save.json")
    uninterrupted = run("play", project, "--ticks", 30, "--compiled-script", compiled)
    reopened = run("play", project, "--ticks", 0, "--compiled-script", compiled, "--load-save", out / "two.save.json")
    assert resumed["state"] == uninterrupted["state"] == reopened["state"]
    assert resumed["script_state"] == uninterrupted["script_state"] == reopened["script_state"]
    assert resumed["script_state"] == {"ticks": 30, "events": ["pulse:3", "pulse:7", "pulse:11", "spawn:13", "pulse:15", "pulse:19"]}
    assert spawned in resumed["state"]["entities"]
    assert resumed["logs"] == [log for log in uninterrupted["logs"] if log["tick"] > 10]
    assert json.loads((out / "two.save.json").read_text())["schedule"]["timers"] == {}
    assert all(r["frames"] == [] and r["adapter"] is None for r in [first, resumed, uninterrupted, reopened])
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    result = {"passed": True, "strict_typescript": True, "rendering": False, "steps": [10, 20],
              "pending_deadlines_restored": True, "callbacks_mutate_through_commands": True,
              "same_tick_cancellation": True, "full_state_equal": True,
              "resumed_logs_equal_to_uninterrupted_suffix": True, "second_save_reopened": True,
              "author_files_unchanged": before, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    (out / "runs.json").write_text(json.dumps({"first": first, "resumed": resumed, "uninterrupted": uninterrupted,
                                              "reopened": reopened}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
