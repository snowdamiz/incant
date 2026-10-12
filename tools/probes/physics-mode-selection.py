#!/usr/bin/env python3
"""Exercise public physics setting edits and matching native Play processes.

Build the current deterministic incant_headless first, then pass it with --binary.
All outputs must be new. The engine cache --target-dir must be outside the project.
This CPU developer proof does not satisfy shipping, editor or device gates.
"""
import argparse
import json
from pathlib import Path
import runpy
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from native_process import run
from native_package import sha256, write_json

Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
SCRIPT = '00000000000000000000000001'
ENTITY = '00000000000000000000000002'
SOURCE = '''import type { NativeBehavior } from "@incant/runtime/native";
export default { update({ chunks, dt }) {
  for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++)
    chunk.transforms[i * chunk.transformStride] += 6 * dt;
}} satisfies NativeBehavior;
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--target-dir', type=Path, required=True)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    project = out / 'project/game.json'
    project.parent.mkdir()
    journal = project.with_suffix('.journal.jsonl')
    calls, processes = [], []

    def execute(argv, expected=0):
        argv = list(map(str, argv))
        result = run(argv, cwd=ROOT, timeout=3600)
        item = {'argv': argv, 'exit_code': result.returncode,
                'stdout': result.stdout, 'stderr': result.stderr}
        processes.append(item)
        write_json(out / 'processes.json', processes)
        assert result.returncode == expected, item
        return result

    def call(rpc, method, params=None):
        record = {'method': method, 'params': params}
        calls.append(record)
        try:
            record['result'] = rpc.call(method, params)
            return record['result']
        finally:
            write_json(out / 'rpc.json', calls)

    execute([binary, 'init', project, '--entities', 0])
    rpc = Rpc(binary, project, journal)
    try:
        before = call(rpc, 'project.read')
        scene = next(iter(before['project']['scenes']))
        commands = [
            {'op': 'write_script', 'script_id': SCRIPT,
             'expected_source': {'state': 'missing'}, 'source': SOURCE},
            {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Motion', 'path': 'motion.ts'}},
            {'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': ENTITY, 'name': 'Mover', 'parent': None, 'provenance': None,
                'components': {
                    'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                    'Script': {'source': SCRIPT, 'props': {}},
                },
            }},
        ]
        call(rpc, 'command.execute', {'commands': commands, 'expected_revision': before['revision'],
                                     'description': 'Author physics process proof'})
        call(rpc, 'project.save')
    finally:
        rpc.close()
    write_json(out / 'observe.json', {
        'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
        'checks': [{'name': 'authored displacement', 'tick': 12, 'expect': {
            'type': 'vector', 'entity': ENTITY, 'property': 'translation',
            'value': [1.2, 0, 0], 'tolerance': 1e-12,
        }}],
    })
    modes = ['deterministic', 'fast', 'deterministic']
    reports = []
    for ordinal, mode in enumerate(modes):
        rpc = Rpc(binary, project, journal)
        try:
            old = call(rpc, 'project.read')
            call(rpc, 'command.execute', {
                'commands': [{'op': 'set_physics_mode', 'mode': mode}],
                'expected_revision': old['revision'], 'description': 'Select ' + mode,
            })
            new = call(rpc, 'project.read')
            call(rpc, 'history.undo')
            assert call(rpc, 'project.read')['project'] == old['project']
            call(rpc, 'history.redo')
            assert call(rpc, 'project.read')['project'] == new['project']
            call(rpc, 'project.save')
        finally:
            rpc.close()
        payload = project.read_bytes()
        evidence = out / f'play-{ordinal}-{mode}'
        result = execute([sys.executable, ROOT / 'tools/play_native.py', project,
                          '--target-dir', args.target_dir, '--evidence', evidence,
                          '--', '--ticks', 12, '--observe', out / 'observe.json'])
        report = json.loads(result.stdout)
        receipt = json.loads((evidence / 'selection.json').read_text())
        assert report['passed'] and report['tick'] == 12 and report['script_source_parsing']
        assert receipt['passed'] and receipt['physics_mode'] == mode
        assert receipt['project_sha256'] == sha256(payload) and project.read_bytes() == payload
        reports.append(report)
        if mode == 'fast':
            wrong = execute([binary, 'native-play', project, '--ticks', 12], 1)
            assert 'physics mode fast' in wrong.stderr and 'deterministic' in wrong.stderr
            assert not wrong.stdout
    write_json(out / 'result.json', {
        'passed': True, 'public_mode_undo_redo': True, 'modes': modes, 'reports': reports,
        'limits': ['Native CPU developer process only; no editor GPU/window, shipping export, performance or phone claim.'],
    })
    print(json.dumps({'passed': True, 'output': str(out)}))


if __name__ == '__main__':
    main()
