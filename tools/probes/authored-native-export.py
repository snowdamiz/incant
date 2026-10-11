#!/usr/bin/env python3
"""Exercise public source transactions, checkpointing and a source-free export."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    root, output = Path(__file__).resolve().parents[2], args.output.resolve()
    sys.path.insert(0, str(root / 'tools'))
    import native_process
    import export_native
    module = runpy.run_path(str(root / 'tools/probes/script-source-transactions.py'))
    require, source, Rpc = module['require'], module['source'], module['Rpc']
    entity, script = module['ENTITY'], module['SCRIPT']
    binary = args.binary.resolve(strict=True)
    inputs = output / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'
    def execute(argv, **kwargs):
        result = native_process.run(argv, **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result
    def write(path, value):
        path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')
    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()
    if args.author:
        document = json.loads(project.read_text(encoding='utf-8'))
        scene = next(iter(document['scenes']))
        rate = document['settings']['tick_rate']
        rpc = Rpc(binary, project, journal)
        try:
            created = rpc.call('command.execute', {'expected_revision': 0,
                'description': 'Author export acceptance scene', 'commands': [
                    {'op': 'write_script', 'script_id': script,
                     'expected_source': {'state': 'missing'}, 'source': source(6)},
                    {'op': 'upsert_script', 'script': {'id': script, 'name': 'Move', 'path': 'move.ts'}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {
                        'id': entity, 'name': 'Mover', 'parent': None, 'provenance': None,
                        'components': {'Transform': {'translation': [0, 0, 0],
                         'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                         'Script': {'source': script, 'props': {}}}}}]})
            require(created['revision'] == 1, 'Creation was not atomic')
            authored = rpc.call('project.read')
            result = rpc.call('play.run', {'seconds': 20 / rate})
            require(result['passed'] and result['state']['tick'] == 20, 'Native Play failed')
            value = rpc.call('play.state_query', {'path': f'/entities/{entity}/transform/translation'})
            expected = 6 * 20 / rate
            require(abs(value['untrusted_runtime_data'][0] - expected) < 1e-9, 'Wrong Play result')
            require(rpc.call('project.read') == authored, 'Play mutated authoring')
            require(rpc.call('project.save')['saved'], 'Checkpoint was not saved')
            require(json.loads(project.read_text(encoding='utf-8')) == authored['project'], 'Checkpoint differs')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(rpc.call('project.read') == authored, 'Reopened checkpoint/journal differs')
            require(rpc.call('script.read', {'script_id': script})['untrusted_source_data']['source']
                    == source(6), 'Reopened script differs')
        finally:
            rpc.close()
        write(output / 'authoring-result.json', {'native_play_x': value['untrusted_runtime_data'][0],
            'expected_x': expected, 'checkpoint_and_journal_reopened': True,
            'binary_sha256': digest(binary), 'project_sha256': digest(project),
            'script_sha256': digest(inputs / 'move.ts'), 'project_content_only_through_commands': True})
        return
    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([str(binary), 'init', str(project), '--entities', '0'])
    execute([sys.executable, str(Path(__file__).resolve()), str(output), '--binary', str(binary), '--author'], timeout=180)
    authored = json.loads((output / 'authoring-result.json').read_text(encoding='utf-8'))
    report = export_native.export(project, output / 'package')
    require(report['project_sha256'] == authored['project_sha256'], 'Export used a different checkpoint')
    shutil.rmtree(inputs)  # Only this probe's newly created private fixture.
    require(not inputs.exists(), 'Authoring fixture survived')
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    plan = output / 'observe.json'
    write(plan, {'format': 'incant-native-observation', 'version': 1, 'entities': [entity],
        'checks': [{'name': 'same-as-native-play', 'tick': 20, 'expect': {
            'type': 'vector', 'entity': entity, 'property': 'translation',
            'value': [authored['expected_x'], 0, 0], 'tolerance': 1e-9}}]})
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    runs = []
    for index in range(3):
        result = execute([str(player), '--package', str(player.parent / 'data'), '--ticks', '20',
            '--output', str(output / f'run-{index}.pcm'), '--observe', str(plan)],
            cwd=player.parent, env=env, timeout=60)
        value = json.loads(result.stdout)
        require(value['observations']['passed'], 'Exported motion differs from Native Play')
        write(output / f'run-{index}.json', value)
        runs.append(value['observations']['entities'][entity]['translation'][0])
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'fresh_exported_runs': runs,
        'ticks_each': 20, 'authoring_deleted': True, 'build_storage_removed': report['build_storage_removed_before_startup'],
        'player_sha256': report['player_sha256'], 'target': report['target'],
        'limitations': ['CPU functional proof', 'No physical phone, GPU, performance or phase gate']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
