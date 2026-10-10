#!/usr/bin/env python3
"""Replay keyboard/mouse/gamepad/touch through a real TS character; no rendering."""
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
    run("init", project, "--entities", 0, "--name", "Input-driven character")
    scene = next(iter(json.loads(project.read_text())["scenes"]))
    player = f"{12:026d}"
    commands = []
    for eid, name, at, shape in [
        (f"{10:026d}", "Floor", [0, -.5, 0], {"type": "box", "half_extents": [10, .5, 10]}),
        (f"{11:026d}", "Wall", [2.5, 1, 0], {"type": "box", "half_extents": [.2, 1, 2]}),
        (player, "Player", [-2, .83, 0], {"type": "capsule", "half_height": .5, "radius": .3}),
    ]:
        components = {
            "Transform": {"translation": at, "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]},
            "Collider": {"shape": shape, "density": 1000, "friction": .6, "restitution": 0,
                         "sensor": False, "memberships": 4294967295, "filter": 4294967295},
        }
        if eid == player:
            components["RigidBody"] = {"motion": "kinematic", "gravity_scale": 0, "linear_damping": 0,
                                        "angular_damping": 0, "can_sleep": False, "ccd": False}
            components["Velocity"] = {"linear": [0, 0, 0]}
        commands.append({"op": "create_entity", "scene_id": scene, "entity": {
            "id": eid, "name": name, "parent": None, "provenance": None, "components": components}})
    requests = [{"id": 1, "method": "command.execute", "params": {"commands": commands,
                 "expected_revision": 0, "description": "Create input-driven character course"}},
                {"id": 2, "method": "project.save"}]
    replies = subprocess.check_output([str(binary), "rpc", str(project), "--journal", str(journal)],
                                     input="".join(json.dumps(r) + "\n" for r in requests), text=True, cwd=ROOT)
    assert all("error" not in json.loads(line) for line in replies.splitlines()), replies
    source = out / "behavior.ts"
    source.write_text("import type { ScriptApi } from " + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n" +
        "const scene = " + json.dumps(scene) + ";\nconst player = " + json.dumps(player) + ";\n" + """
    type State={ticks:number,vy:number,grounded:boolean,maxY:number,jumps:number,presses:number,
                taps:number,clicks:number,wheel:number,yaw:number,isolated:boolean};
    export default defineBehavior<State>({initialState:{ticks:0,vy:0,grounded:false,maxY:0,jumps:0,
        presses:0,taps:0,clicks:0,wheel:0,yaw:0,isolated:true},
      update(api:ScriptApi,dt,state){
        state.ticks++;const input=api.input();
        if(input.keyboard.pressed.includes('KeyW'))state.presses++;
        if(input.mouse.buttons.pressed.includes('left'))state.clicks++;
        state.taps+=input.gestures.filter(g=>g.type==='tap').length;
        state.wheel+=input.mouse.wheel_lines[1];state.yaw+=input.mouse.motion[0]*0.001;
        const entity=api.query('RigidBody').find(e=>e.id===player);
        if(!entity)throw Error('player missing');
        const transform=entity.components.Transform as {translation:number[]};
        state.maxY=Math.max(state.maxY,transform.translation[1]);
        if(input.keyboard.pressed.includes('Space') && state.grounded){state.vy=5;state.jumps++;}
        state.vy-=9.81*dt;
        const speed=(input.keyboard.held.includes('KeyW')?2:0)+(input.gamepads['0']?.left_stick[0]??0);
        const movement=api.computeCharacterMotion({scene_id:scene,entity_id:player,
          translation:[Math.cos(state.yaw)*speed*dt,state.vy*dt,Math.sin(state.yaw)*speed*dt],
          options:{offset:0.02,slide:true,max_slope_climb_angle:Math.PI/4,min_slope_slide_angle:Math.PI/4,
                   snap_to_ground:0.3,autostep:null}});
        state.grounded=movement.grounded;if(movement.grounded&&state.vy<0)state.vy=0;
        api.command({op:'set_component',scene_id:scene,entity_id:player,component:'Velocity',
                     value:{linear:movement.translation.map(v=>v/dt)}});
        input.keyboard.held.push('KeyQ');if(api.input().keyboard.held.includes('KeyQ'))state.isolated=false;
        if(state.ticks%30===0)api.log('tick '+state.ticks+' grounded '+state.grounded);
      }
    });\n""")
    subprocess.run(["node", str(ROOT / "node_modules/typescript/bin/tsc"), "--strict", "--noEmit", "--target", "ES2022",
                    "--module", "ESNext", "--moduleResolution", "bundler", str(source)], check=True, cwd=ROOT)
    compiled = out / "behavior.js"
    subprocess.run(["node", str(ROOT / "tools/build_script.mjs"), str(source), str(compiled)], check=True, cwd=ROOT)
    key = lambda code, down: {"type": "key", "code": code, "down": down}
    clip = {"format": "incant-input", "version": 1, "tick_rate": 60, "start_tick": 0, "ticks": 180, "frames": [
        {"tick": 1, "events": [key("KeyW", True), {"type": "gamepad_connected", "id": 0},
            {"type": "gamepad_axis", "id": 0, "axis": "left_x", "value": .5}]},
        {"tick": 5, "events": [{"type": "mouse_button", "button": "left", "down": True}]},
        {"tick": 8, "events": [{"type": "mouse_button", "button": "left", "down": False}]},
        {"tick": 15, "events": [{"type": "wheel", "delta": [0, 2], "unit": "lines"},
                                   {"type": "pointer_motion", "delta": [20, 0]}]},
        {"tick": 50, "events": [key("KeyW", True)]},
        {"tick": 60, "events": [key("Space", True)]},
        {"tick": 61, "events": [key("Space", False)]},
        {"tick": 90, "events": [{"type": "touch", "id": 1, "phase": "down", "position": [10, 20]}]},
        {"tick": 92, "events": [{"type": "touch", "id": 1, "phase": "up", "position": [11, 20]}]},
        {"tick": 140, "events": [key("KeyW", False)]},
        {"tick": 150, "events": [{"type": "focus", "focused": False}]},
    ]}
    recording = out / "input.json"
    recording.write_text(json.dumps(clip, indent=2) + "\n")
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal, recording]}
    common = ["play", project, "--compiled-script", compiled, "--input-replay", recording]
    checks = [
        {"name": "initial position", "tick": 0, "path": f"/state/entities/{player}/translation/0", "expect": {"type": "approx", "value": -2, "tolerance": 1e-8}},
        {"name": "jump occurred", "tick": 75, "path": "/script_state/jumps", "expect": {"type": "equals", "value": 1}},
        {"name": "airborne checkpoint", "tick": 75, "path": f"/state/entities/{player}/translation/1", "expect": {"type": "range", "min": 1.5, "max": 3}},
        {"name": "focus lost", "tick": 150, "path": "/input/focused", "expect": {"type": "equals", "value": False}},
        {"name": "controls released", "tick": 150, "path": "/input/keyboard/held", "expect": {"type": "equals", "value": []}},
        {"name": "wall contact", "tick": 180, "path": f"/state/entities/{player}/translation/0", "expect": {"type": "range", "min": 1.9, "max": 2.01}},
        {"name": "landed", "tick": 180, "path": "/script_state/grounded", "expect": {"type": "equals", "value": True}},
    ]
    def assertions(name, selected):
        path = out / name
        path.write_text(json.dumps({"format": "incant-play-assertions", "version": 1, "checks": selected}, indent=2) + "\n")
        return path
    early = assertions("early-assertions.json", [c for c in checks if c["tick"] <= 75])
    late = assertions("late-assertions.json", [c for c in checks if c["tick"] >= 75])
    whole = assertions("all-assertions.json", checks)
    first = run(*common, "--ticks", 75, "--assertions", early, "--save-output", out / "checkpoint.json")
    resumed = run(*common, "--ticks", 105, "--assertions", late, "--load-save", out / "checkpoint.json")
    full = run(*common, "--ticks", 180, "--assertions", whole)
    repeat = run(*common, "--ticks", 180, "--assertions", whole)
    assert all(r["passed"] and all(c["passed"] for c in r["assertions"]) for r in [first, resumed, full, repeat])
    wrong = assertions("wrong-assertions.json", checks + [
        {"name": "deliberately wrong", "tick": 180, "path": "/script_state/grounded", "expect": {"type": "equals", "value": False}}])
    failed = subprocess.run([str(binary), *map(str, common), "--ticks", "180", "--assertions", str(wrong),
                             "--save-output", str(out / "must-not-publish.json")], capture_output=True, text=True, cwd=ROOT)
    diagnostic = json.loads(failed.stdout)
    assert failed.returncode != 0 and diagnostic["completed"] and not diagnostic["passed"]
    assert diagnostic["state"] == full["state"] and diagnostic["assertions"][-1]["passed"] is False
    assert not (out / "must-not-publish.json").exists()
    (out / "failed-assertion-report.json").write_text(json.dumps(diagnostic, indent=2) + "\n")
    for field in ["state", "script_state", "input"]:
        assert full[field] == repeat[field] == resumed[field], field
    state = full["script_state"]
    assert state["jumps"] == state["presses"] == state["taps"] == state["clicks"] == 1, state
    assert state["wheel"] == 2 and state["yaw"] == .02 and state["isolated"] and state["grounded"], state
    assert state["maxY"] > 1.8, state
    position = full["state"]["entities"][player]["translation"]
    assert 1.9 < position[0] < 2.01 and .8 < position[1] < .85, position
    assert resumed["logs"] == [entry for entry in full["logs"] if entry["tick"] > 75]
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal, recording]}
    assert all(report["frames"] == [] and report["adapter"] is None for report in [first, resumed, full, repeat])
    result = {"passed": True, "strict_typescript": True, "rendering": False, "recorded_inputs": ["keyboard", "mouse", "gamepad", "touch"],
              "actual_character_jump_and_wall_contact": True, "save_resume_mid_jump_exact": True, "repeat_exact": True,
              "final_position": position, "script_state": state, "author_files_unchanged": before,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "live_device_adapters": False, "scheduled_gameplay_assertions": True,
              "failed_assertion_exits_nonzero_with_report_and_no_save": True}
    (out / "runs.json").write_text(json.dumps({"first": first, "resumed": resumed, "full": full, "repeat": repeat}, indent=2) + "\n")
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
