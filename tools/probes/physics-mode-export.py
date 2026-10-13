#!/usr/bin/env python3
"""Verify both authored physics modes in fresh ordinary shipping exports.

Consumes a successful physics-mode-selection.py output. Each private project
copy is edited through the public command bus, exported, then removed before
running the copied player. This is local CPU evidence, not a performance gate.
"""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import export_native
from native_package import sha256, write_json
from native_process import run

Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']


def inventory(root):
    return {str(p.relative_to(root)): sha256(p.read_bytes())
            for p in sorted(root.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('development', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    development = args.development.resolve(strict=True)
    baseline = json.loads((development / 'result.json').read_text())
    assert baseline['passed'] and baseline['public_mode_undo_redo']
    assert baseline['modes'] == ['deterministic', 'fast', 'deterministic']
    before = inventory(development)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    results = {}
    for index, mode in enumerate(('deterministic', 'fast')):
        case = output / mode
        case.mkdir()
        authoring = case / 'authoring'
        shutil.copytree(development / 'project', authoring)
        project = authoring / 'game.json'
        rpc = Rpc(args.binary.resolve(strict=True), project, authoring / 'export.journal.jsonl')
        try:
            previous = rpc.call('project.read')
            rpc.call('command.execute', {
                'commands': [{'op': 'set_physics_mode', 'mode': mode}],
                'expected_revision': previous['revision'],
                'description': 'Select shipping physics mode',
            })
            rpc.call('project.save')
        finally:
            rpc.close()
        authored = inventory(authoring)
        write_json(case / 'authoring-before.json', authored)
        exported = export_native.export(project, case / 'package')
        assert exported['physics_mode'] == mode and exported['physics_variant'] == mode
        assert exported['physics']['mode'] == mode
        assert exported['project_sha256'] == authored['game.json']
        assert exported['build_storage_removed_before_startup']
        assert exported['startup_source_parsing'] is False
        assert inventory(authoring) == authored
        removed = {}
        for path in (case / 'package/symbols').rglob('*'):
            if path.is_file() and (path.name.endswith('.map') or path.name == 'script-symbols.json'):
                removed[str(path.relative_to(case))] = sha256(path.read_bytes())
                path.unlink()
        shutil.rmtree(authoring)
        player_root = case / 'package/player'
        player = player_root / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
        package = inventory(player_root)
        expected = baseline['reports'][index]
        observe = case / 'observe.json'
        shutil.copyfile(development / 'observe.json', observe)
        environment = {'PATH': str(player_root), 'TMPDIR': str(case),
                       'TMP': str(case), 'TEMP': str(case)}
        environment.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR')
                            if key in os.environ})
        argv = [str(player), '--package', str(player_root / 'data'), '--ticks', '12',
                '--observe', str(observe), '--output', str(case / 'output.pcm')]
        process = run(argv, cwd=player_root, env=environment, timeout=120)
        write_json(case / 'process.json', {'argv': argv, 'exit_code': process.returncode,
                                          'stdout': process.stdout, 'stderr': process.stderr})
        assert process.returncode == 0, process.stderr
        actual = json.loads(process.stdout)
        assert actual['tick'] == 12 and actual['observations']['passed']
        for field in ('frames', 'pcm_sha256', 'logs', 'observations'):
            assert actual[field] == expected[field], (mode, field)
        assert sha256((case / 'output.pcm').read_bytes()) == actual['pcm_sha256']
        assert inventory(player_root) == package and not authoring.exists()
        results[mode] = {'passed': True, 'export': exported, 'report': actual,
                         'package_files': package, 'removed_maps': removed,
                         'authoring_deleted': True}
        write_json(case / 'result.json', results[mode])
        print(mode + ' shipping proof passed', flush=True)
    assert inventory(development) == before
    write_json(output / 'result.json', {
        'passed': True, 'modes': results, 'development_unchanged': True,
        'limits': ['Local CPU execution; no editor/window, physics save, speedup or device gate.'],
    })


if __name__ == '__main__':
    main()
