#!/usr/bin/env python3
"""Import WAV/Vorbis, author audio through RPC, and mix a strict TS game to WAV."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import struct
import subprocess
import wave

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

    with wave.open(str(out / "tone.wav"), "wb") as file:
        file.setparams((1, 2, 48000, 24000, "NONE", "not compressed"))
        file.writeframes(b"".join(struct.pack("<h", round(8000 * math.sin(2 * math.pi * 440 * i / 48000))) for i in range(24000)))
    shutil.copyfile(ROOT / "crates/incant_assets/tests/fixtures/tone-440hz.ogg", out / "music.ogg")
    project = out / "game.incant.json"
    journal = out / "game.incant.journal.jsonl"
    run("init", project, "--entities", 0, "--name", "Scripted audio")
    wav = run("import", project, "tone.wav")
    ogg = run("import", project, "music.ogg")
    scene = next(iter(json.loads(project.read_text())["scenes"]))
    ids = [f"{n:026d}" for n in range(10, 14)]
    commands = []
    for eid, name, components in [
        (ids[0], "SFX", {"AudioBus": {"parent": None, "gain_db": 0}}),
        (ids[1], "Music", {"AudioBus": {"parent": None, "gain_db": 0}}),
        (ids[2], "Tone", {"AudioSource": {"clip": wav["asset"]["id"], "bus": ids[0], "gain_db": 0,
            "pan": -1, "rate": 1, "start_seconds": 0, "playing": True, "looping": True, "streaming": True, "spatial": None}}),
        (ids[3], "Music", {"AudioSource": {"clip": ogg["asset"]["id"], "bus": ids[1], "gain_db": 0,
            "pan": 1, "rate": 1, "start_seconds": 0, "playing": True, "looping": True, "streaming": True, "spatial": None}}),
    ]:
        commands.append({"op": "create_entity", "scene_id": scene, "entity": {
            "id": eid, "name": name, "parent": None, "provenance": None, "components": components}})
    requests = [{"id": 1, "method": "command.execute", "params": {"commands": commands,
        "expected_revision": ogg["revision"], "description": "Create audio scene"}}, {"id": 2, "method": "project.save"}]
    replies = subprocess.check_output([str(binary), "rpc", str(project), "--journal", str(journal)],
        input="".join(json.dumps(r) + "\n" for r in requests), text=True, cwd=ROOT)
    assert all("error" not in json.loads(line) for line in replies.splitlines()), replies
    source = out / "behavior.ts"
    source.write_text("import type { ScriptApi, AudioSource, AudioBus } from " + json.dumps((ROOT / "sdk/ts/src/index").as_posix()) + ";\n" + """
    export default defineBehavior<{ticks:number}>({initialState:{ticks:0},update(api:ScriptApi,dt,s){
        s.ticks++;
        if(s.ticks===21||s.ticks===31)for(const e of api.query('AudioSource'))if(e.name==='Tone') {
            const audio=e.components.AudioSource as AudioSource;
            api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'AudioSource',
                         value:{...audio,playing:s.ticks===31}});
        }
        if(s.ticks===41)for(const e of api.query('AudioBus'))if(e.name==='Music') {
            const bus=e.components.AudioBus as AudioBus;
            api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'AudioBus',
                         value:{...bus,gain_db:-6}});
        }
    }});
    """)
    subprocess.run(["node", str(ROOT / "node_modules/typescript/bin/tsc"), "--strict", "--noEmit", "--target", "ES2022",
                    "--moduleResolution", "bundler", "--module", "ESNext", str(source)], check=True, cwd=ROOT)
    compiled = out / "behavior.js"
    subprocess.run(["node", str(ROOT / "tools/build_script.mjs"), str(source), str(compiled)], check=True, cwd=ROOT)
    for filename in ["tone.wav", "music.ogg"]:
        (out / filename).unlink()
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    common = ["play", project, "--ticks", 60, "--compiled-script", compiled]
    full = run(*common, "--audio-output", out / "mix.wav")
    repeat = run(*common, "--audio-output", out / "repeat.wav")
    assert full["script_commands"] == 3 and full["script_state"]["ticks"] == 60
    assert full["audio"]["pcm_sha256"] == repeat["audio"]["pcm_sha256"]
    assert full["audio"]["frames"] == 48000 and full["audio"]["samples_at_full_scale"] == 0
    assert full["adapter"] is None and not full["frames"]
    data = (out / "mix.wav").read_bytes()
    assert data[:4] == b"RIFF" and data[48:52] == b"data" and len(data) == 56 + 48000 * 8
    pcm = list(struct.iter_unpack("<ff", data[56:]))
    def rms(start, end, channel):
        return math.sqrt(sum(frame[channel] ** 2 for frame in pcm[start:end]) / (end-start))
    left = rms(1000, 13000, 0)
    silent = rms(17000, 23000, 0)
    resumed = rms(25000, 31000, 0)
    right = rms(1000, 13000, 1)
    quiet_right = rms(34000, 46000, 1)
    assert abs(left - 8000 / 32768) < .002 and silent == 0 and abs(resumed - left) < .002
    assert .35 < right < .38 and abs(quiet_right / right - 10 ** (-6/20)) < .005
    # Audio failures cannot overwrite prior outputs or publish successful reports.
    rejected = subprocess.run([str(binary), *map(str, common), "--audio-output", str(out / "mix.wav")],
                               capture_output=True, text=True, cwd=ROOT)
    assert rejected.returncode != 0 and not rejected.stdout
    assert (out / "mix.wav").read_bytes() == data
    after = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    assert before == after
    result = {"passed": True, "public_import_and_rpc": True, "strict_typescript": True,
        "source_free": True, "repeated_pcm_exact": True, "frames": 48000, "sample_rate": 48000,
        "left_rms": left, "paused_left_rms": silent, "resumed_left_rms": resumed,
        "right_rms": right, "attenuated_right_rms": quiet_right, "bus_gain_ratio": quiet_right / right,
        "existing_output_untouched": True, "author_files_unchanged": before,
        "pcm_sha256": full["audio"]["pcm_sha256"], "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "audio_device_opened": False, "pixel_capture": False}
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
