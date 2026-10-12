#!/usr/bin/env python3
"""Exercise the trusted mode-selecting worker owner with real authored behavior."""
import argparse
import json
from pathlib import Path
import runpy
import shutil
import sys
import threading

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from native_package import require, sha256, write_json
from play_native import play
from play_worker import NativeRefusal, start
Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
ENTITY = '00000000000000000000000002'


def inventory(root):
    return {str(p.relative_to(root)): sha256(p.read_bytes()) for p in root.rglob('*') if p.is_file()}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--authoring', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--target-dir', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    args = p.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    original = args.authoring.resolve(strict=True); original_files = inventory(original)
    project = out / 'authoring/game.json'; shutil.copytree(original, project.parent)
    calls, states = [], []

    def mode(value):
        rpc = Rpc(args.binary.resolve(strict=True), project, project.parent / 'owner.journal.jsonl')
        try:
            before = rpc.call('project.read')
            result = rpc.call('command.execute', {'expected_revision': before['revision'],
                'description': 'Select retained worker mode', 'commands': [{'op': 'set_physics_mode', 'mode': value}]})
            rpc.call('project.save')
            calls.append({'mode': value, 'result': result, 'project_sha256': sha256(project.read_bytes())})
            write_json(out / 'authoring.json', calls)
        finally:
            rpc.close()

    write_json(out / 'observe.json', {'format': 'incant-native-observation', 'version': 1,
                                     'entities': [ENTITY], 'checks': []})
    mode('deterministic')
    with start(project, args.target_dir, evidence=out / 'deterministic') as worker:
        # An authorized authoring edit after ready affects only the next process.
        mode('fast')
        for tick in range(1, 13):
            state = worker.step({'observe': {'entities': [ENTITY], 'include_logs': True}})
            require(state['tick'] == tick, 'Wrong deterministic tick')
        require(worker.ready['physics_mode'] == 'deterministic', 'Live kernel changed with authoring')
        states.append(state)
        try:
            worker.step({'observe': {'entities': ['bad-id']}})
        except NativeRefusal:
            pass
        else:
            raise ValueError('Bad native selection was accepted')
        require(worker.observe()['tick'] == 12, 'Refusal advanced the retained world')
    baseline = play(project, args.target_dir, ['--ticks', '12', '--observe', str(out / 'observe.json')],
                    evidence=out / 'one-shot-fast')
    with start(project, args.target_dir, evidence=out / 'fast') as worker:
        require(worker.ready['physics_mode'] == 'fast', 'Next worker did not select edited mode')
        shutil.rmtree(project.parent)
        for tick in range(1, 13):
            state = worker.step({'observe': {'entities': [ENTITY], 'include_logs': True}})
            require(state['tick'] == tick, 'Wrong fast tick')
        states.append(state)
    for state in states:
        entity = state['entities'][ENTITY]; expected = baseline['observations']['entities'][ENTITY]
        require(entity['transform'] == {k: expected[k] for k in ('translation', 'rotation', 'scale')}
                and entity['world_transform'] == expected['world_transform']
                and state['frames'] == baseline['frames'] and state['logs'] == baseline['logs'],
                'Owned worker differs from ordinary native Play')
    shutil.copytree(original, project.parent)
    mode('fast')
    try:
        with start(project, args.target_dir, evidence=out / 'caller-failure') as worker:
            worker.step()
            raise RuntimeError('intentional caller cancellation')
    except RuntimeError as error:
        require(str(error) == 'intentional caller cancellation', 'Unexpected owner error')
    shutil.rmtree(project.parent)
    selections = {}
    for name in ('deterministic', 'fast', 'one-shot-fast', 'caller-failure'):
        receipt = json.loads((out / name / 'selection.json').read_text())
        require(receipt['passed'] == (name != 'caller-failure'), 'Incorrect completion receipt')
        command = receipt.get('worker_command', receipt.get('play_command'))
        require(not Path(command[0]).exists(), 'Private executable was left behind')
        selections[name] = receipt
    require(not any(t.name == 'native-worker-exchange' for t in threading.enumerate()), 'Worker thread survived')
    require(inventory(original) == original_files, 'Original evidence changed')
    write_json(out / 'result.json', {'passed': True, 'states': states, 'baseline': baseline,
        'selections': selections, 'original_unchanged': True,
        'limits': ['Developer CPU worker only; editor/GPU, Windows execution and phone performance are separate.',
                   'Movement fixture has no active solver bodies; no solver parity/speed claim.']})
    print(json.dumps({'passed': True, 'output': str(out)}))


if __name__ == '__main__':
    main()
