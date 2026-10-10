#!/usr/bin/env python3
"""Cook through the authoring boundary, then run with all authoring files removed."""
import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / f'target/release/examples/cook_scene{suffix}')
    parser.add_argument('--runtime-binary', type=Path, default=ROOT / f'target/release/examples/run_scene{suffix}')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    headless = args.headless_binary.resolve()
    cook = args.cook_binary.resolve()
    runtime = args.runtime_binary.resolve()
    project, journal = out / 'game.incant.json', out / 'history.jsonl'
    subprocess.run([str(headless), 'init', str(project), '--name', 'Cooked world', '--entities', '0'],
                   check=True, capture_output=True, text=True)
    scene = next(iter(json.loads(project.read_text())['scenes']))
    mover, child = '00000000000000000000000001', '00000000000000000000000002'
    transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
    entities = [
        {'id': mover, 'name': 'Mover', 'parent': None, 'provenance': None,
         'components': {'Transform': transform, 'Velocity': {'linear': [3, 0, 0]}}},
        {'id': child, 'name': 'Child', 'parent': mover, 'provenance': None,
         'components': {'Transform': {**transform, 'translation': [1, 2, 3]}}},
    ]
    requests = [
        {'id': 1, 'method': 'command.execute', 'params': {'expected_revision': 0,
         'description': 'Create runtime cook fixture', 'commands': [
             {'op': 'create_entity', 'scene_id': scene, 'entity': entity} for entity in entities]}},
        {'id': 2, 'method': 'project.save'},
    ]
    reply = subprocess.check_output([str(headless), 'rpc', str(project), '--journal', str(journal)],
                                    input=''.join(json.dumps(r) + '\n' for r in requests), text=True)
    replies = [json.loads(line) for line in reply.splitlines()]
    assert len(replies) == 2 and all('error' not in r for r in replies), replies
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (project, journal)}
    scene_file = out / 'play.scene'
    subprocess.run([str(cook), str(project), scene, str(scene_file)], check=True, capture_output=True)
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (project, journal)}
    original = scene_file.read_bytes()
    # These files were created exclusively by this probe. The runtime executable
    # must start from the cooked image after both source and history are gone.
    project.unlink()
    journal.unlink()

    def run(ticks):
        return subprocess.check_output([str(runtime), str(scene_file), str(ticks)], cwd=out, text=True)

    first, repeated, reset = run(60), run(60), run(0)
    assert first == repeated
    assert first.splitlines()[0] == 'tick=60 entities=2'
    positions = [ast.literal_eval(re.search(r'(\[[^\[\]]*\])$', line)[1]) for line in first.splitlines()[1:]]
    assert abs(positions[0][0] - 3) < 1e-12 and positions[0][1:] == [0, 0], positions
    assert positions[1] == [1, 2, 3]
    assert reset.splitlines()[0] == 'tick=0 entities=2' and reset.splitlines()[1].endswith('[0.0, 0.0, 0.0]')
    assert scene_file.read_bytes() == original
    damaged = out / 'damaged.scene'
    corruption = bytearray(original)
    corruption[-1] ^= 1
    damaged.write_bytes(corruption)
    rejection = subprocess.run([str(runtime), str(damaged), '1'], cwd=out, text=True, capture_output=True)
    assert rejection.returncode != 0 and not rejection.stdout
    result = {'passed': True, 'authoring_via_commands': True, 'cook_preserves_authoring': before,
              'authoring_files_removed_before_runtime': True, 'process_restarts_equal': True,
              'reset_discards_simulation': True, 'corruption_rejected': True,
              'scene_bytes': len(original), 'scene_sha256': hashlib.sha256(original).hexdigest(),
              'binaries': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (headless, cook, runtime)},
              'scope': 'Transform/Velocity native-world foundation; no script, render, physics or device performance gate'}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    (out / 'run.txt').write_text(first)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
