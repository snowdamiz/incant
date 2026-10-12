#!/usr/bin/env python3
"""Actual source-deleted Core package proof for bounded streaming PCM.

Consumes a copied, hash-bound authored fixture and immutable prior package.
Neither source art nor the controller is changed. No window or GPU is launched.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(65536), b''):
            value.update(chunk)
    return value.hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture', type=Path, required=True)
    parser.add_argument('--old-package', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--existing-proof', type=Path)
    args = parser.parse_args()
    fixture = args.fixture.resolve(strict=True)
    old = args.old_package.resolve(strict=True)
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    admission = json.loads((fixture / 'receipt.json').read_text())
    assert inventory(fixture / 'project') == admission['project_files']
    assert digest(fixture / 'win-input.json') == admission['route_sha256']
    source = json.loads((fixture / 'project/core.json').read_text())
    scenes = list(source['scenes'].values()); assert len(scenes) == 1
    controllers = [e['components']['Script']['props'] for e in scenes[0]['entities'].values()
                   if 'Script' in e['components']]
    assert len(controllers) == 1
    props = controllers[0]
    camera, player_id, pickups = props['camera'], props['player'], props['pickups']
    assert len(pickups) == 4
    old_manifest = json.loads((old / 'package.json').read_text())
    assert old_manifest['project_sha256'] == digest(fixture / 'project/core.json')
    original = inventory(fixture)
    old_hashes = inventory(old)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    if not args.existing_proof:
        authoring = out / 'authoring'; shutil.copytree(fixture / 'project', authoring)
        sys.path.insert(0, str(ROOT / 'tools')); import export_native
        exported = export_native.export(authoring / 'core.json', out / 'package',
                                         camera=camera, ui_viewport='640x480')
        assert exported['project_sha256'] == digest(authoring / 'core.json')
        write(out / 'deleted-authoring.json', inventory(authoring)); shutil.rmtree(authoring)
        remove = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(out / 'deleted-maps.json', remove(out / 'package'))
    exported = json.loads((proof / 'package/export.json').read_text())
    assert exported['project_sha256'] == digest(fixture / 'project/core.json')
    assert exported['build_storage_removed_before_startup']
    assert json.loads((proof / 'deleted-authoring.json').read_text()) == admission['project_files']
    assert not (proof / 'authoring').exists() and not list((proof / 'package').rglob('*.map'))
    player = proof / 'package/player/incant_game_player'
    assert digest(player) == exported['player_sha256']
    package_before = inventory(proof / 'package')
    commands = []

    def run(name, executable, data, recording, plan, *, diagnostic=None, output=None, failed_report=False):
        input_path = out / f'{name}-input.json'; write(input_path, recording)
        observe_path = out / f'{name}-observe.json'; write(observe_path, plan)
        pcm = output or out / f'{name}.pcm'
        argv = list(map(str, [executable, '--package', data, '--ticks', recording['ticks'],
            '--input-recording', input_path, '--observe', observe_path,
            '--ui-viewport', '640x480', '--output', pcm]))
        env = dict(os.environ); env['PATH'] = str(executable.parent)
        result = subprocess.run(argv, cwd=executable.parent, env=env, capture_output=True,
                                text=True, timeout=600)
        call = {'argv': argv, 'cwd': str(executable.parent), 'exit_code': result.returncode,
                'stdout': result.stdout, 'stderr': result.stderr}
        write(out / f'command-{len(commands):03d}.json', call); commands.append(call)
        if diagnostic is not None:
            assert result.returncode != 0 and result.stderr == diagnostic, call
            if not failed_report:
                assert not result.stdout, call
                return None
        else:
            assert result.returncode == 0 and not result.stderr, call
        report = json.loads(result.stdout); write(out / f'{name}.json', report)
        assert report['observations']['passed'] is (not failed_report)
        assert report['frames'] * 8 == pcm.stat().st_size and report['pcm_sha256'] == digest(pcm)
        assert not list(out.glob('.incant-pcm-*')), 'staging leak'
        return report

    replay = json.loads((fixture / 'win-input.json').read_text()); assert replay['ticks'] == 2156
    plan = {'format': 'incant-native-observation', 'version': 1,
            'entities': [player_id], 'checks': [
                {'name': 'player survives', 'tick': 2156,
                 'expect': {'type': 'exists', 'entity': player_id, 'value': True}},
                *[{'name': 'collected ' + p, 'tick': 2156,
                   'expect': {'type': 'exists', 'entity': p, 'value': False}} for p in pickups]]}
    # The prior ordinary package refuses this same duration before any output.
    run('old-bound', old / 'player/incant_game_player', old / 'player/data', replay, plan,
        diagnostic='Error: output frame bound\n')
    assert not (out / 'old-bound.pcm').exists()
    full = run('full', player, player.parent / 'data', replay, plan)
    assert full['frames'] == 1_724_800 and full['tick'] == 2156
    messages = full['logs']['entries']; assert not full['logs']['dropped']
    assert [e['tick'] for e in messages if e['message'].startswith('core.pickup')] == [127, 508, 1266, 1842]
    assert [e['tick'] for e in messages if e['message'].startswith('core.won')] == [2066]
    assert not [e for e in messages if e['message'].startswith('core.lost')]
    assert full['render_input']['ui']['glyphs'] > 0
    assert full == run('repeat', player, player.parent / 'data', replay, plan)
    assert digest(out / 'full.pcm') == digest(out / 'repeat.pcm')
    # Compare the same sample-rate prefix to the prior whole-buffer implementation.
    short = {**replay, 'ticks': 1000, 'frames': [f for f in replay['frames'] if f['tick'] <= 1000]}
    short_plan = {'format': 'incant-native-observation', 'version': 1,
                  'entities': [player_id], 'checks': []}
    before = run('old-prefix', old / 'player/incant_game_player', old / 'player/data', short, short_plan)
    after = run('prefix', player, player.parent / 'data', short, short_plan)
    for key in ('tick', 'frames', 'logs', 'observations', 'pcm_sha256'):
        assert before[key] == after[key], key
    # Completed failed assertions keep diagnostic PCM without a checkpoint request.
    bad = json.loads(json.dumps(plan)); bad['checks'][0]['expect']['value'] = False
    failed = run('failed-observation', player, player.parent / 'data', replay, bad,
                 diagnostic='Error: native observation assertions failed (see completed report)\n',
                 failed_report=True)
    assert failed['pcm_sha256'] == full['pcm_sha256'] and failed['tick'] == full['tick']
    sentinel = out / 'existing.pcm'; sentinel.write_bytes(b'existing accepted output')
    run('no-clobber', player, player.parent / 'data', replay, plan, output=sentinel,
        diagnostic='Error: File exists (os error 17)\n')
    assert sentinel.read_bytes() == b'existing accepted output'
    missing = out / 'absent/audio.pcm'
    run('output-io', player, player.parent / 'data', replay, plan, output=missing,
        diagnostic='Error: No such file or directory (os error 2)\n')
    assert not missing.exists() and not list(out.glob('.incant-pcm-*'))
    assert inventory(fixture) == original and inventory(old) == old_hashes
    assert inventory(proof / 'package') == package_before
    write(out / 'result.json', {'passed': True, 'player_sha256': digest(player),
        'fixture': original, 'old_package': old_hashes, 'package_files': package_before,
        'helper_sha256': digest(Path(__file__)), 'commands': len(commands),
        'frames': full['frames'], 'pcm_sha256': full['pcm_sha256'],
        'won_tick': 2066, 'exact_prior_prefix': True, 'diagnostic_pcm_preserved': True,
        'limits': ['CPU ordinary package only; no native/GPU/device/performance claim',
                   'PCM and checkpoint publication remain separate transactions']})


if __name__ == '__main__':
    main()
