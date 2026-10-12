#!/usr/bin/env python3
"""Public Core/Planar whole-session restart proof; numerical, without a window.

Prepare copies retained authored fixtures and changes them through public commands.
Ship performs an ordinary export, deletes only its own source/debug-map copy, and
compares fresh games with restarted games through the actual packaged player.
"""
import argparse
import copy
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
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def api(sample):
    base = runpy.run_path(str(ROOT / f'samples/{sample}/verify.py'))
    display = runpy.run_path(str(ROOT / ('samples/planar/presentation.py' if sample == 'planar'
                                       else 'samples/core/verify_presentation.py')))
    return base, display


class Processes:
    def __init__(self, out):
        self.out, self.calls = out, []

    def run(self, argv, *, cwd=ROOT, env=None, diagnostic=None):
        argv = list(map(str, argv))
        result = subprocess.run(argv, cwd=cwd, env=env, text=True, capture_output=True, timeout=300)
        record = {'argv': argv, 'cwd': str(cwd), 'exit_code': result.returncode,
                  'stdout': result.stdout, 'stderr': result.stderr}
        self.calls.append(record)
        write(self.out / f'command-{len(self.calls):03d}.json', record)
        if diagnostic is not None:
            assert result.returncode != 0 and diagnostic == result.stderr, record
            assert not result.stdout, record
            return record
        assert result.returncode == 0, record
        return json.loads(result.stdout)


def timeline(frames, ticks):
    grouped = {}
    for row in frames:
        grouped.setdefault(row['tick'], []).extend(row['events'])
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
            'ticks': ticks, 'frames': [{'tick': t, 'events': e} for t, e in sorted(grouped.items())]}


def checks(base, sample, name, ticks):
    if name == 'idle':
        return {'format': 'incant-native-observation', 'version': 1, 'entities': [base['PLAYER']],
                'checks': [{'name': 'fresh-' + p, 'tick': ticks,
                            'expect': {'type': 'exists', 'entity': p, 'value': True}} for p in base['PICKUPS']]}
    plan = base['observations'](name, ticks)
    if sample == 'core':
        for check in plan['checks']:
            if check['expect']['type'] == 'entity_count':
                check['expect']['value'] += 14  # Actual accepted presentation fixture entities.
    else:
        api(sample)[1]['extend_observations'](plan)
    return plan


def prepare(args):
    sample, out = args.sample, args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = args.authoring.resolve(strict=True)
    source_hashes = inventory(source)
    inputs = out / 'authoring'; shutil.copytree(source, inputs)
    project = inputs / f'{sample}.json'
    binary = args.binary.resolve(strict=True)
    base, display = api(sample)
    camera = base['CAMERA'] if sample == 'planar' else display['CAMERA']
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    # A new journal begins from the copied accepted document, preserving originals.
    journal = inputs / 'restart.journal.jsonl'
    rpc = Rpc(binary, project, journal); calls = []

    def call(method, params=None):
        result = rpc.call(method, params)
        calls.append({'method': method, 'params': params, 'result': result})
        write(out / 'authoring-calls.json', calls)
        return result

    authored_source = display['observed_source']((ROOT / f'samples/{sample}/gameplay.ts').read_text())
    try:
        before = call('project.read')
        old_script = call('script.read', {'script_id': base['SCRIPT']})['untrusted_source_data']
        actions = copy.deepcopy(before['project']['settings']['input_actions'])
        actions['restart'] = {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'KeyR'},
            {'type': 'gamepad_button', 'id': 0, 'button': 'start'}]}
        call('command.execute', {'expected_revision': before['revision'], 'description': 'Bind whole-session restart fixture',
            'commands': [{'op': 'write_script', 'script_id': base['SCRIPT'], 'expected_source': old_script['version'],
                          'source': authored_source}, {'op': 'set_input_actions', 'actions': actions}]})
        authored = call('project.read')
        call('history.undo'); assert call('project.read')['project'] == before['project']
        assert call('script.read', {'script_id': base['SCRIPT']})['untrusted_source_data']['source'] == old_script['source']
        call('history.redo'); assert call('project.read')['project'] == authored['project']
        call('project.save'); authored = call('project.read')
    finally:
        rpc.close()
    rpc = Rpc(binary, project, journal)
    try:
        assert call('project.read') == authored
        assert call('script.read', {'script_id': base['SCRIPT']})['untrusted_source_data']['source'] == authored_source
    finally:
        rpc.close()
    frozen = inventory(inputs)
    process = Processes(out); reports = {}
    names = base.get('NAMES', ('win', 'lose', 'gamepad', 'jump', 'skip-pickup', 'wall', 'repeat-win'))
    for name in (*names, 'idle'):
        replay = timeline([], 30) if name == 'idle' else base['replay']('win' if name == 'repeat-win' else name)
        plan = checks(base, sample, name, replay['ticks'])
        write(out / f'{name}-input.json', replay); write(out / f'{name}-observe.json', plan)
        report = process.run([binary, 'native-play', project, '--camera', camera, '--ticks', replay['ticks'],
            '--ui-viewport', '640x480', '--input-replay', out / f'{name}-input.json', '--observe', out / f'{name}-observe.json'])
        assert report['observations']['passed'] and not report['gpu_submitted']
        write(out / f'{name}.json', report); reports[name] = report
    base['verify_reports']({k: v for k, v in reports.items() if k != 'idle'})
    # Headless Play does not silently accept requests without a replacement owner.
    unsupported = copy.deepcopy(base['replay']('win'))
    unsupported['frames'].append({'tick': 300, 'events': [{'type': 'key', 'code': 'KeyR', 'down': True}]})
    write(out / 'unsupported-input.json', unsupported)
    process.run([binary, 'native-play', project, '--camera', camera, '--ticks', 300, '--ui-viewport', '640x480',
                 '--input-replay', out / 'unsupported-input.json'], diagnostic='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n')
    assert inventory(inputs) == frozen and inventory(source) == source_hashes
    write(out / 'prepared.json', {'sample': sample, 'camera': camera, 'runs': list(reports),
        'binary_sha256': digest(binary), 'original_authoring': str(source), 'original_hashes': source_hashes,
        'authoring_files': frozen, 'source_sha256': digest(ROOT / f'samples/{sample}/gameplay.ts'),
        'authored_source_sha256': hashlib.sha256(authored_source.encode()).hexdigest(),
        'helper_sha256': digest(Path(__file__)), 'public_undo_redo_reopen': True})


def ship(args):
    development = args.development.resolve(strict=True)
    prepared = json.loads((development / 'prepared.json').read_text())
    sample = prepared['sample']; base, _ = api(sample)
    assert prepared['source_sha256'] == digest(ROOT / f'samples/{sample}/gameplay.ts')
    before = inventory(development)
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    if not args.existing_proof:
        inputs = out / 'authoring'; shutil.copytree(development / 'authoring', inputs)
        assert inventory(inputs) == prepared['authoring_files']
        sys.path.insert(0, str(ROOT / 'tools')); import export_native
        exported = export_native.export(inputs / f'{sample}.json', out / 'package',
                                         camera=prepared['camera'], ui_viewport='640x480')
        assert exported['project_sha256'] == digest(inputs / f'{sample}.json')
        write(out / 'deleted-authoring.json', inventory(inputs)); shutil.rmtree(inputs)
        remove = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(out / 'deleted-maps.json', remove(out / 'package'))
    exported = json.loads((proof / 'package/export.json').read_text())
    deleted = json.loads((proof / 'deleted-authoring.json').read_text())
    assert exported['build_storage_removed_before_startup'] and exported['project_sha256'] == deleted[f'{sample}.json']
    assert deleted['gameplay.ts'] == prepared['authored_source_sha256']
    assert not (proof / 'authoring').exists() and not list((proof / 'package').rglob('*.map'))
    player = proof / 'package/player/incant_game_player'
    assert digest(player) == exported['player_sha256']
    package_hashes = inventory(proof / 'package')
    process = Processes(out); actual = {}
    environment = {'PATH': str(player.parent), 'TMPDIR': str(out), 'TMP': str(out), 'TEMP': str(out)}
    environment.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})

    def play(name, replay, plan):
        write(out / f'{name}-input.json', replay); write(out / f'{name}-observe.json', plan)
        pcm = out / f'{name}.pcm'
        report = process.run([player, '--package', player.parent / 'data', '--ticks', replay['ticks'],
            '--input-recording', out / f'{name}-input.json', '--observe', out / f'{name}-observe.json',
            '--ui-viewport', '640x480', '--output', pcm], cwd=player.parent, env=environment)
        assert report['observations']['passed'] and digest(pcm) == report['pcm_sha256']
        assert report['render_input']['ui']['glyphs'] > 0 and not report['logs']['dropped']
        write(out / f'{name}.json', report); actual[name] = report
        return report, pcm.read_bytes()

    for name in prepared['runs']:
        report, _ = play(name, json.loads((development / f'{name}-input.json').read_text()),
                         json.loads((development / f'{name}-observe.json').read_text()))
        expected = json.loads((development / f'{name}.json').read_text())
        for field in ('tick', 'frames', 'logs', 'observations', 'pcm_sha256'):
            assert report[field] == expected[field], (name, field)
        assert 'session' not in report, 'Non-restart report compatibility changed'

    for name, first, second, pad, held in [
        ('win-restart-win', 'win', 'win', False, False),
        ('gamepad-restart-win', 'gamepad', 'gamepad', True, False),
        ('lose-restart-win', 'lose', 'win', False, False),
        ('held-input-restart-idle', 'win', 'idle', False, True),
    ]:
        one = json.loads((development / f'{first}-input.json').read_text())
        two = json.loads((development / f'{second}-input.json').read_text())
        split = one['ticks']; frames = copy.deepcopy(one['frames'])
        button = {'type': 'gamepad_button', 'id': 0, 'button': 'start', 'value': 1} if pad else {'type': 'key', 'code': 'KeyR', 'down': True}
        frames.append({'tick': split, 'events': [button]})
        if held:
            frames.append({'tick': split - 1, 'events': [{'type': 'key', 'code': 'KeyD', 'down': True},
                                                      {'type': 'key', 'code': 'Space', 'down': True}]})
        else:
            release = {**button, **({'value': 0} if pad else {'down': False})}
            frames.append({'tick': split + 1, 'events': [release]})
        for row in two['frames']:
            frames.append({'tick': split + row['tick'], 'events': [e for e in row['events'] if e['type'] != 'gamepad_connected']})
        plan = copy.deepcopy(json.loads((development / f'{first}-observe.json').read_text()))
        final = copy.deepcopy(json.loads((development / f'{second}-observe.json').read_text()))
        plan['entities'] = final['entities']
        for check in plan['checks']: check['name'] = 'before-' + check['name']
        for check in final['checks']:
            check['name'] = 'after-' + check['name']; check['tick'] += split
        plan['checks'] += final['checks']
        # Identical simulation ticks in distinct sessions must each be checked once.
        for t in (1, split + 1):
            plan['checks'].append({'name': f'fresh-pickup-host-{t}', 'tick': t,
                'expect': {'type': 'exists', 'entity': base['PICKUPS'][0], 'value': True}})
        if held:
            for action in ('move', 'restart'):
                plan['checks'].append({'name': 'held-isolated-' + action, 'tick': split + 1,
                    'expect': {'type': 'action', 'name': action, 'active': False, 'pressed': False}})
        report, pcm = play(name, timeline(frames, split + two['ticks']), plan)
        expected = actual[second]
        assert report['tick'] == expected['tick'] and report['session'] == {
            'generation': 2, 'host_tick': split + two['ticks'], 'simulation_tick': two['ticks'],
            'restarts': [{'host_step': split, 'completed_generation': 1, 'simulation_tick': split,
                          'output_frames': str(actual[first]['frames'])}]}, name
        for field in ('logs', 'render_input', 'flipbooks', 'sources'):
            assert report[field] == expected[field], (name, field)
        assert report['observations']['entities'] == expected['observations']['entities'], name
        assert pcm == (out / f'{first}.pcm').read_bytes() + (out / f'{second}.pcm').read_bytes(), name
        assert report['frames'] == actual[first]['frames'] + expected['frames']
        assert len(report['observations']['checks']) == len(plan['checks'])
    assert inventory(development) == before and inventory(proof / 'package') == package_hashes
    write(out / 'result.json', {'passed': True, 'sample': sample, 'proof': str(proof),
        'helper_sha256': digest(Path(__file__)), 'player_sha256': digest(player),
        'source_sha256': prepared['source_sha256'], 'project_sha256': exported['project_sha256'],
        'package_files': package_hashes, 'runs': list(actual), 'commands': len(process.calls),
        'exact_fresh_session_pcm': True, 'same_simulation_tick_distinct_host_checks': True,
        'limits': ['CPU ordinary package only; no native-window/GPU/device evidence',
                   'Fresh startup reset, not physics save/restore', 'Wasm and headless Play request owners remain unsupported']})


def main():
    parser = argparse.ArgumentParser(description=__doc__); modes = parser.add_subparsers(dest='mode', required=True)
    p = modes.add_parser('prepare'); p.add_argument('sample', choices=['core', 'planar']); p.add_argument('authoring', type=Path)
    p.add_argument('output', type=Path); p.add_argument('--binary', type=Path, required=True)
    p = modes.add_parser('ship'); p.add_argument('development', type=Path); p.add_argument('output', type=Path)
    p.add_argument('--existing-proof', type=Path)
    args = parser.parse_args(); (prepare if args.mode == 'prepare' else ship)(args)


if __name__ == '__main__':
    main()
