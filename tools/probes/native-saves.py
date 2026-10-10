#!/usr/bin/env python3
"""Verify native world saves in fresh processes after deleting authoring inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    for name, default in (
        ('headless', f'target/release/incant_headless{suffix}'),
        ('cook', f'target/release/examples/cook_scene{suffix}'),
        ('runtime', f'target/release/examples/save_world{suffix}'),
    ):
        parser.add_argument(f'--{name}-binary', type=Path, default=ROOT / default)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    player_dir = out / 'player'
    player_dir.mkdir()
    headless, cook, runtime = (getattr(args, f'{n}_binary').resolve() for n in ('headless', 'cook', 'runtime'))
    player = player_dir / runtime.name
    shutil.copy2(runtime, player)
    binaries = {path.name: sha(path) for path in (headless, cook, runtime)}
    with tempfile.TemporaryDirectory(prefix='authoring-', dir=out) as temporary:
        inputs = Path(temporary)
        project, journal = inputs / 'world.incant.json', inputs / 'history.jsonl'
        subprocess.run([headless, 'init', project, '--name', 'Native saves', '--entities', '0'],
                       check=True, capture_output=True)
        scene_id = next(iter(json.loads(project.read_text())['scenes']))
        transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
        entities = [
            {'id': '00000000000000000000000001', 'parent': None,
             'components': {'Transform': transform, 'Velocity': {'linear': [3, -2, 1]}}},
            {'id': '00000000000000000000000002', 'parent': '00000000000000000000000001',
             'components': {'Transform': {**transform, 'translation': [1, 2, 3]}}},
        ]
        requests = [
            {'id': 1, 'method': 'command.execute', 'params': {'expected_revision': 0,
             'description': 'Native world save fixture', 'commands': [
                 {'op': 'create_entity', 'scene_id': scene_id,
                  'entity': {**entity, 'name': f'Entity {i}', 'provenance': None}}
                 for i, entity in enumerate(entities)]}},
            {'id': 2, 'method': 'project.save'},
        ]
        reply = subprocess.check_output([headless, 'rpc', project, '--journal', journal],
                                        input=''.join(json.dumps(r) + '\n' for r in requests), text=True)
        replies = [json.loads(line) for line in reply.splitlines()]
        assert len(replies) == 2 and all('error' not in r for r in replies), replies
        before = {path.name: sha(path) for path in (project, journal)}
        scene = player_dir / 'world.scene'
        subprocess.run([cook, project, scene_id, scene], check=True, capture_output=True)
        assert before == {path.name: sha(path) for path in (project, journal)}
    assert not inputs.exists()
    observations = []

    def run(kind, source, ticks, name):
        destination = player_dir / name
        reply = subprocess.check_output([player, kind, source, str(ticks), destination],
                                        cwd=player_dir, text=True)
        observations.append({'operation': kind, 'additional_ticks': ticks, 'output': reply,
                             'save_sha256': sha(destination)})
        return destination

    original = scene.read_bytes()
    baseline = run('scene', scene, 60, 'baseline.save')
    partial = run('scene', scene, 25, 'partial.save')
    assert scene.read_bytes() == original
    scene.unlink()
    partial_bytes = partial.read_bytes()
    resumed = run('save', partial, 35, 'resumed.save')
    repeated = run('save', partial, 35, 'repeated.save')
    unchanged = run('save', partial, 0, 'unchanged.save')
    assert baseline.read_bytes() == resumed.read_bytes() == repeated.read_bytes()
    assert partial.read_bytes() == unchanged.read_bytes() == partial_bytes
    assert observations[2]['output'].startswith('tick=60 rate=60 entities=2 ')
    damaged = player_dir / 'damaged.save'
    invalid = bytearray(partial_bytes)
    invalid[-1] ^= 1
    damaged.write_bytes(invalid)
    refused_output = player_dir / 'must-not-exist.save'
    rejected = subprocess.run([player, 'save', damaged, '1', refused_output],
                              cwd=player_dir, capture_output=True)
    assert rejected.returncode != 0 and not rejected.stdout and not refused_output.exists()
    protected = subprocess.run([player, 'save', partial, '1', partial],
                               cwd=player_dir, capture_output=True)
    assert protected.returncode != 0 and partial.read_bytes() == partial_bytes
    result = {'passed': True, 'binaries': binaries, 'authoring_hashes_preserved': before,
              'authoring_removed_before_execution': True, 'scene_removed_before_resume': True,
              'fresh_process_continuation_equals_uninterrupted': True,
              'repeated_and_zero_tick_restores_exact': True, 'corruption_rejected': True,
              'existing_save_preserved': True, 'observations': observations,
              'scope': 'Native Transform/Velocity/hierarchy/clock only; no VM, timers, inputs, complete session, durability or device gate'}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': True, 'evidence': str(out / 'result.json')}))


if __name__ == '__main__':
    main()
