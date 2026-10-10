#!/usr/bin/env python3
"""Author, undo and replay named input actions through public commands. No GPU."""
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
    out, binary = args.output.resolve(), args.binary.resolve()
    out.mkdir(parents=True, exist_ok=False)

    def run(*args):
        return json.loads(subprocess.check_output([str(binary), *map(str, args)], text=True, cwd=ROOT))

    project, journal = out / "game.incant.json", out / "history.jsonl"
    run("init", project, "--name", "Rebindable actions", "--entities", 0)
    sid = next(iter(json.loads(project.read_text())["scenes"]))
    entity = f"{10:026d}"
    actions = {
        "jump": {"kind": "button", "bindings": [{"type": "key", "code": "Space"},
            {"type": "mouse_button", "button": "left"}, {"type": "gamepad_button", "id": 3, "button": "south"}]},
        "move": {"kind": "axis2", "dead_zone": 0.2, "bindings": [
            {"type": "key", "code": "KeyW", "scale": [0, 1]},
            {"type": "key", "code": "KeyD", "scale": [1, 0]},
            {"type": "gamepad_stick", "id": 3, "stick": "left", "invert_y": True}]},
    }
    rebound = {**actions, "jump": {"kind": "button", "bindings": [
        {"type": "key", "code": "KeyJ"}, {"type": "gesture", "gesture": "tap"}]}}
    commands = [{"op": "set_input_actions", "actions": actions}, {"op": "create_entity", "scene_id": sid, "entity": {
        "id": entity, "name": "Player", "parent": None, "provenance": None, "components": {
            "Transform": {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
            "Velocity": {"linear": [0, 0, 0]}}}}]
    requests = [
        {"id": 1, "method": "command.execute", "params": {"commands": commands, "expected_revision": 0, "description": "Bind player input"}},
        {"id": 2, "method": "history.undo"}, {"id": 3, "method": "history.redo"},
        {"id": 4, "method": "command.execute", "params": {"commands": [
            {"op": "set_memory", "section": "prefix", "text": "must not commit"},
            {"op": "set_input_actions", "actions": {"jump": {"kind": "button", "threshold": 0, "bindings": []}}}],
            "expected_revision": 3, "description": "Reject invalid bindings"}},
        {"id": 5, "method": "project.read"}, {"id": 6, "method": "project.save"},
    ]
    replies = subprocess.check_output([str(binary), "rpc", str(project), "--journal", str(journal)],
                                     input="".join(json.dumps(r) + "\n" for r in requests), text=True, cwd=ROOT)
    replies = [json.loads(line) for line in replies.splitlines()]
    assert len(replies) == 6 and all("error" not in replies[i] for i in [0, 1, 2, 4, 5]), replies
    assert "error" in replies[3], replies
    assert "input_actions" not in replies[1]["result"]["project"]["settings"]
    assert replies[0]["result"]["project"] == replies[2]["result"]["project"] == replies[4]["result"]["project"]
    assert replies[4]["result"]["revision"] == 3

    source = out / "behavior.ts"
    source.write_text("import type { InputActions } from " + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n" +
        "const rebound: InputActions = " + json.dumps(rebound) + ";\n" + """
export default defineBehavior<{ticks:number,presses:number[],releases:number[],isolated:boolean}>({
  initialState:{ticks:0,presses:[],releases:[],isolated:true},
  update(api,_dt,state){
    const tick=api.clock().tick, input=api.input();state.ticks++;
    if(input.actions.jump.pressed)state.presses.push(tick);
    if(input.actions.jump.released)state.releases.push(tick);
    const move=input.actions.move.value;
    if(Math.hypot(...move)>1.00000001)throw Error('unnormalized movement');
    for(const e of api.query('Transform'))api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,
      component:'Velocity',value:{linear:[move[0]*3,0,move[1]*3]}});
    if(tick===6)api.command({op:'set_input_actions',actions:rebound});
    const previous=input.actions.jump.active;input.actions.jump.active=!previous;
    if(api.input().actions.jump.active!==previous)state.isolated=false;
    api.log(JSON.stringify([tick,api.input().actions]));
  }
});
""")
    subprocess.run(["node", str(ROOT / "node_modules/typescript/bin/tsc"), "--strict", "--noEmit", "--target", "ES2022",
                    "--module", "ESNext", "--moduleResolution", "bundler", str(source)], check=True, cwd=ROOT)
    compiled = out / "behavior.js"
    subprocess.run(["node", str(ROOT / "tools/build_script.mjs"), str(source), str(compiled)], check=True, cwd=ROOT)
    def key(code, down): return {"type": "key", "code": code, "down": down}
    def button(value): return {"type": "gamepad_button", "id": 3, "button": "south", "value": value}
    def axis(value): return {"type": "gamepad_axis", "id": 3, "axis": "left_x", "value": value}
    events = {
        1: [key("KeyW", True), key("Space", True)],
        2: [{"type": "gamepad_connected", "id": 3}, button(1)], 3: [key("Space", False)], 4: [button(0)],
        5: [key("KeyJ", True)], 6: [axis(0.6), key("KeyD", True), key("KeyW", False)],
        8: [key("KeyJ", False)], 9: [key("KeyJ", True), key("KeyJ", False)], 10: [axis(0.1)],
        11: [key("KeyD", False), key("KeyW", True), axis(0)], 12: [{"type": "focus", "focused": False}],
        13: [{"type": "focus", "focused": True}, axis(0.6)], 14: [{"type": "gamepad_disconnected", "id": 3}],
        15: [{"type": "touch", "id": 1, "phase": phase, "position": [10, 10]} for phase in ["down", "up"]],
        16: [{"type": "gamepad_connected", "id": 3}, button(1)], 17: [key("KeyJ", True)], 19: [key("KeyJ", False)],
    }
    recording = out / "input.json"
    recording.write_text(json.dumps({"format": "incant-input", "version": 1, "tick_rate": 60, "start_tick": 0,
        "ticks": 20, "frames": [{"tick": tick, "events": packet} for tick, packet in events.items()]}) + "\n")
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal, recording]}
    common = ["play", project, "--compiled-script", compiled, "--input-replay", recording]
    first = run(*common, "--ticks", 7, "--save-output", out / "one.save.json")
    resumed = run(*common, "--ticks", 13, "--load-save", out / "one.save.json", "--save-output", out / "two.save.json")
    whole = run(*common, "--ticks", 20)
    reopened = run(*common, "--ticks", 0, "--load-save", out / "two.save.json")
    assert resumed["state"] == whole["state"] == reopened["state"]
    assert resumed["script_state"] == whole["script_state"] == reopened["script_state"]
    assert resumed["script_state"] == {"ticks": 20, "presses": [1, 9, 15, 17], "releases": [4, 8, 9, 16, 19], "isolated": True}
    assert resumed["logs"] == whole["logs"][7:]
    position = resumed["state"]["entities"][entity]["translation"]
    assert all(abs(a - b) < 1e-12 for a, b in zip(position, [0.275, 0, 0.3])), position
    assert json.loads((out / "one.save.json").read_text())["project"]["settings"]["input_actions"]["jump"]["bindings"][0]["code"] == "KeyJ"
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal, recording]}
    assert all(r["frames"] == [] and r["adapter"] is None for r in [first, resumed, whole, reopened])
    result = {"passed": True, "strict_typescript": True, "rendering": False, "steps": [7, 13],
        "atomic_command_undo_redo": True, "runtime_rebind_saved": True, "full_state_equal": True,
        "continued_logs_equal": True, "subtick_taps": True, "alternative_bindings_do_not_repeat_presses": True,
        "touch_gesture_binding": True, "author_files_unchanged": before,
        "final_position": position,
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    (out / "runs.json").write_text(json.dumps({"first": first, "resumed": resumed, "whole": whole, "reopened": reopened}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
