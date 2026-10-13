#!/usr/bin/env python3
"""Public authored Core bindings: real native UI/audio and packaged session reset.

prepare copies an accepted scene.py build, binds its existing roles through RPC,
and executes native CPU checks. ship exports only the production attachment,
removes its own sources/maps, and runs the complete routes plus whole-session
restarts. The separate observer/control attachment is never exported. No GPU.
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
import wave

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import bindings
import binding_oracle as oracle
import hud
from art import ids


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


def same_source(actual, expected):
    # Undo is a new project revision. Only that enclosing read revision changes;
    # source bytes, digest/version, identity/path and history consistency do not.
    return {k: v for k, v in actual.items() if k != 'revision'} == {
        k: v for k, v in expected.items() if k != 'revision'}


class Processes:
    def __init__(self, out):
        self.out, self.calls = out, []

    def run(self, argv, *, cwd=ROOT, env=None, diagnostic=None):
        command = list(map(str, argv))
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=900)
        row = {'argv': command, 'cwd': str(cwd), 'environment': env,
               'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        self.calls.append(row); write(self.out / f'command-{len(self.calls):03d}.json', row)
        if diagnostic is not None:
            assert result.returncode != 0 and result.stderr == diagnostic and not result.stdout, row
            return row
        assert result.returncode == 0, row
        return json.loads(result.stdout)


class Author:
    def __init__(self, binary, project, out):
        self.factory = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        self.binary, self.project, self.out = binary, project, out
        self.journal = project.parent / 'bindings.journal.jsonl'
        self.calls = []
        self.rpc = self.factory(binary, project, self.journal)

    def call(self, method, params=None):
        result = self.rpc.call(method, params)
        self.calls.append({'method': method, 'params': params, 'result': result})
        write(self.out / 'authoring-calls.json', self.calls)
        return result

    def edit(self, description, commands):
        return self.call('command.execute', {'expected_revision': self.call('project.read')['revision'],
            'description': description, 'commands': commands})

    def save_reopen(self):
        self.call('project.save'); expected = self.call('project.read')
        self.rpc.close(); self.rpc = self.factory(self.binary, self.project, self.journal)
        assert self.call('project.read') == expected

    def close(self):
        self.rpc.close()


def negative_bindings(author, source):
    """Real public documents, each temporarily edited then exactly undone."""
    before = author.call('project.read')['project']
    script = author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data']
    health = copy.deepcopy(before['scenes'][bindings.SCENE]['entities'][bindings.HUD['health']]['components']['UiNode'])
    health['kind'] = {'role': 'container'}
    alternative = ids.make('script', 97)
    controller = copy.deepcopy(before['scenes'][bindings.SCENE]['entities'][bindings.CONTROLLER]['components']['Script'])
    controller['source'] = alternative
    cases = [
        ('wrong-role', [{'op': 'set_component', 'scene_id': bindings.SCENE,
            'entity_id': bindings.HUD['health'], 'component': 'UiNode', 'value': health}], 'Core HUD role mismatch: health'),
        ('wrong-source', [
            {'op': 'write_script', 'script_id': alternative, 'expected_source': {'state': 'missing'}, 'source': source},
            {'op': 'upsert_script', 'script': {'id': alternative, 'name': 'Wrong source control', 'path': 'wrong.ts'}},
            {'op': 'set_component', 'scene_id': bindings.SCENE, 'entity_id': bindings.CONTROLLER,
             'component': 'Script', 'value': controller}], 'Core controller source identity mismatch'),
        ('missing-table', [{'op': 'remove_string_table', 'table_id': bindings.HUD['table']}], 'Core HUD string table is missing'),
    ]
    records = []
    for name, commands, message in cases:
        author.edit('Binding refusal control ' + name, commands)
        try:
            bindings.commands(author.call('project.read')['project'], script, source)
        except ValueError as error:
            assert str(error) == message
        else:
            raise AssertionError('Binding preflight accepted ' + name)
        author.call('history.undo')
        assert author.call('project.read')['project'] == before
        assert same_source(author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data'], script)
        records.append({'case': name, 'diagnostic': message, 'public_edit_undo': True})
    return records


def native(process, binary, project, out, receipt, name, recording, *, observe_name=None, diagnostic=None):
    write(out / f'{name}-input.json', recording)
    plan = oracle.plan(receipt, observe_name or name, recording)
    write(out / f'{name}-observe.json', plan)
    output = out / f'{name}.pcm'
    report = process.run([binary, 'native-play', project, '--camera', receipt['entities']['camera'],
        '--ui-viewport', '640x480', '--ticks', recording['ticks'], '--sample-rate', oracle.RATE,
        '--input-replay', out / f'{name}-input.json', '--observe', out / f'{name}-observe.json',
        '--pcm-output', output], diagnostic=diagnostic)
    if diagnostic is not None:
        assert not output.exists(); return report
    assert report['cpu_render_input'] and not report['gpu_submitted'] and report['script_source_parsing']
    assert digest(output) == report['pcm_sha256']
    oracle.pcm(output, report); oracle.verify(report, receipt, observe_name or name)
    write(out / f'{name}.json', report)
    return report


def prepare(args):
    original = args.build.resolve(strict=True); protected = inventory(original)
    receipt = json.loads((original / 'receipt.json').read_text())
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    inputs = out / 'authoring'; shutil.copytree(original / 'project', inputs)
    project = inputs / 'core.json'; binary = args.binary.resolve(strict=True)
    source = (HERE / 'gameplay.ts').read_text()
    write(out / 'original.json', {'root': str(original), 'files': protected, 'receipt': receipt})
    author = Author(binary, project, out)
    try:
        before = author.call('project.read')['project']
        prior = author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data']
        refusals = negative_bindings(author, source)
        tx = author.edit('Connect existing Core HUD, cues and whole-session restart', bindings.commands(before, prior, source))
        after = author.call('project.read')['project']
        author.call('history.undo'); assert author.call('project.read')['project'] == before
        assert same_source(author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data'], prior)
        author.call('history.redo'); assert author.call('project.read')['project'] == after
        author.save_reopen()
        assert author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data']['source'] == source
    finally:
        author.close()
    # Prove every original component and hierarchy survives except the explicitly
    # bound Script props/provenance; no art/layout/mixer or physics mutation.
    for owner, entity in before['scenes'][bindings.SCENE]['entities'].items():
        if owner != bindings.CONTROLLER:
            assert after['scenes'][bindings.SCENE]['entities'][owner] == entity
    frozen = inventory(inputs); process = Processes(out)
    recordings = oracle.cases(); reports = {}
    for name, recording in recordings.items():
        reports[name] = native(process, binary, project, out, receipt, name, recording)
    assert not any((out / 'lose.pcm').read_bytes()[-oracle.TICK_BYTES * 60:])
    assert not any((out / 'win-tail.pcm').read_bytes()[-oracle.TICK_BYTES * 60:])
    assert (out / 'win-tail.pcm').read_bytes().startswith((out / 'win.pcm').read_bytes())

    observed = out / 'observed'; shutil.copytree(inputs, observed)
    # A separate journal on a fresh copy avoids changing the production seed.
    observed.joinpath('bindings.journal.jsonl').unlink()
    obs_out = out / 'observer'; obs_out.mkdir(); author = Author(binary, observed / 'core.json', obs_out)
    try:
        author.edit('Attach test-only readback and real cue-isolation controls', [
            {'op': 'write_script', 'script_id': oracle.PROBE_SCRIPT, 'expected_source': {'state': 'missing'}, 'source': oracle.PROBE_SOURCE},
            {'op': 'upsert_script', 'script': {'id': oracle.PROBE_SCRIPT, 'name': 'Readback proof', 'path': 'observe.ts'}},
            {'op': 'create_entity', 'scene_id': bindings.SCENE, 'entity': {'id': oracle.PROBE_ENTITY,
                'name': 'Readback proof', 'parent': None, 'provenance': None, 'components': {
                    'Script': {'source': oracle.PROBE_SCRIPT, 'props': {}}}}}])
        author.save_reopen()
    finally:
        author.close()
    # Localization gold values come from the real engine, not a Python ICU clone.
    scene = runpy.run_path(str(HERE / 'scene.py'))
    texts = {locale: hud.resolve(binary, out / ('l10n-' + locale), locale, 4, scene['run'], scene['Session'])
             for locale in hud.LOCALES}
    write(out / 'localized.json', texts)
    for name in ('win', 'lose'):
        actual = native(process, binary, observed / 'core.json', obs_out, receipt, name, recordings[name])
        oracle.verify_hud(actual, texts['en'])
        assert oracle.game_logs(actual) == reports[name]['logs']['entries']
        assert actual['observations'] == reports[name]['observations']
        assert (obs_out / f'{name}.pcm').read_bytes() == (out / f'{name}.pcm').read_bytes()
    cue_checks = {}
    for role, key in oracle.MUTE_KEYS.items():
        name = 'lose' if role == 'lost' else 'win'
        recording = copy.deepcopy(recordings[name])
        recording = oracle.timeline([*recording['frames'], {'tick': 1, 'events': [
            {'type': 'key', 'code': key, 'down': True}]}], recording['ticks'])
        control = native(process, binary, observed / 'core.json', obs_out, receipt, 'mute-' + role, recording, observe_name=name)
        assert oracle.game_logs(control) == reports[name]['logs']['entries']
        clip = {'pickup': 'cue_pickup', 'damage': 'cue_hurt', 'won': 'cue_won', 'lost': 'cue_lost'}[role]
        with wave.open(str(inputs / 'art' / (clip + '.wav'))) as sound:
            seconds = sound.getnframes() / sound.getframerate()
        cue_checks[role] = oracle.cue_difference((out / f'{name}.pcm').read_bytes(),
            (obs_out / ('mute-' + role + '.pcm')).read_bytes(), reports[name], role, seconds)
    for locale in ('ar', 'zh-Hans', 'en-XA'):
        locale_out = out / ('locale-' + locale); locale_out.mkdir()
        author = Author(binary, observed / 'core.json', locale_out)
        try:
            author.edit('Use existing Claude locale presentation ' + locale,
                hud.present(bindings.SCENE, texts[locale],
                    [scene['ASSETS'][k][0] for k in hud.DOCUMENT['fonts']], 4, locale, 'playing'))
            author.save_reopen()
            localized = native(process, binary, observed / 'core.json', locale_out, receipt, 'win', recordings['win'])
            oracle.verify_hud(localized, texts[locale])
            assert (locale_out / 'win.pcm').read_bytes() == (out / 'win.pcm').read_bytes()
        finally:
            author.call('history.undo'); author.call('project.save'); author.close()
    unsupported = oracle.restarted(recordings['win'], recordings['idle'])
    native(process, binary, project, out, receipt, 'unsupported-restart', unsupported, observe_name='idle',
        diagnostic='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n')
    assert inventory(inputs) == frozen and inventory(original) == protected
    write(out / 'prepared.json', {'passed': True, 'project_sha256': digest(project), 'source_sha256': digest(HERE / 'gameplay.ts'),
        'binary_sha256': digest(binary), 'authoring_files': frozen, 'authoring_transaction': tx,
        'public_undo_redo_reopen': True, 'binding_refusals': refusals, 'cue_checks': cue_checks,
        'receipt': receipt, 'runs': list(recordings), 'observer_source_sha256': hashlib.sha256(oracle.PROBE_SOURCE.encode()).hexdigest(),
        'helper_sources': {p: digest(HERE / p) for p in ('bindings.py', 'binding_oracle.py', 'verify_bindings.py')},
        'limits': ['No GPU/window/device execution', 'Observer and isolation controls exist only in a separate test project',
                   'Headless Play intentionally refuses restart; packaged replacement owner checked by ship']})


def ship(args):
    development = args.development.resolve(strict=True)
    prepared = json.loads((development / 'prepared.json').read_text())
    assert prepared['passed'] and prepared['source_sha256'] == digest(HERE / 'gameplay.ts')
    protected = inventory(development); receipt = prepared['receipt']
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    if not args.existing_proof:
        inputs = out / 'authoring'; shutil.copytree(development / 'authoring', inputs)
        assert inventory(inputs) == prepared['authoring_files']
        document = json.loads((inputs / 'core.json').read_text())
        assert set(document['scripts']) == {bindings.SCRIPT} and not (inputs / 'observe.ts').exists()
        sys.path.insert(0, str(ROOT / 'tools')); import export_native
        exported = export_native.export(inputs / 'core.json', out / 'package',
            camera=receipt['entities']['camera'], ui_viewport='640x480')
        assert exported['project_sha256'] == digest(inputs / 'core.json')
        write(out / 'deleted-authoring.json', inventory(inputs)); shutil.rmtree(inputs)
        remove = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(out / 'deleted-maps.json', remove(out / 'package'))
    exported = json.loads((proof / 'package/export.json').read_text())
    deleted = json.loads((proof / 'deleted-authoring.json').read_text())
    assert exported['project_sha256'] == prepared['project_sha256'] == deleted['core.json']
    assert deleted['gameplay.ts'] == prepared['source_sha256']
    assert exported['build_storage_removed_before_startup'] and not (proof / 'authoring').exists()
    assert not list((proof / 'package').rglob('*.map'))
    player = proof / 'package/player/incant_game_player'
    assert digest(player) == exported['player_sha256']
    package_files = inventory(proof / 'package'); process = Processes(out)
    env = {'PATH': str(player.parent), 'TMPDIR': str(out), 'TMP': str(out), 'TEMP': str(out)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    texts = json.loads((development / 'localized.json').read_text())['en']
    recordings = oracle.cases(); actual = {}

    def play(name, recording, plan):
        write(out / f'{name}-input.json', recording); write(out / f'{name}-observe.json', plan)
        report = process.run([player, '--package', player.parent / 'data', '--ticks', recording['ticks'],
            '--input-recording', out / f'{name}-input.json', '--observe', out / f'{name}-observe.json',
            '--ui-viewport', '640x480', '--output', out / f'{name}.pcm'], cwd=player.parent, env=env)
        assert report['observations']['passed'] and report['logs']['dropped'] == 0
        assert digest(out / f'{name}.pcm') == report['pcm_sha256']
        ui = report['render_input']['ui']; assert ui['widgets'] == 12 and ui['glyphs'] > 0 and ui['focus'] is None
        expected = oracle.labels(texts, *oracle.game_state(report)[:2])
        assert all(ui['sources'].get(owner) == text for owner, text in expected.items())
        write(out / f'{name}.json', report); actual[name] = report
        return report, oracle.pcm(out / f'{name}.pcm', report)

    for name, recording in recordings.items():
        report, data = play(name, recording, oracle.plan(receipt, name, recording))
        oracle.verify(report, receipt, name)
        baseline = json.loads((development / f'{name}.json').read_text())
        for field in ('tick', 'frames', 'logs', 'observations', 'pcm_sha256'):
            assert report[field] == baseline[field], (name, field)
        assert data == (development / f'{name}.pcm').read_bytes() and 'session' not in report
    recordings, restarts = oracle.restart_cases()
    report, data = play('lose-short', recordings['lose-short'],
                        oracle.plan(receipt, 'lose', recordings['lose-short']))
    oracle.verify(report, receipt, 'lose')
    assert data == (out / 'lose.pcm').read_bytes()[:len(data)]
    assert not any(data[-oracle.TICK_BYTES * 60:])
    assert report['logs'] == actual['lose']['logs']
    for name, first, second, pad, held in restarts:
        one, two = recordings[first], recordings[second]; split = one['ticks']
        report, data = play(name, oracle.restarted(one, two, pad=pad, held=held),
                           oracle.restart_plan(receipt, first, second, one, two, held=held))
        assert report['session'] == {'generation': 2, 'host_tick': split + two['ticks'],
            'simulation_tick': two['ticks'], 'restarts': [{'host_step': split, 'completed_generation': 1,
                'simulation_tick': split, 'output_frames': str(actual[first]['frames'])}]}
        assert report['tick'] == two['ticks']
        for field in ('logs', 'render_input', 'flipbooks', 'sources'):
            assert report[field] == actual[second][field], (name, field)
        assert report['observations']['entities'] == actual[second]['observations']['entities']
        assert len(report['observations']['checks']) == len(oracle.restart_plan(receipt, first, second, one, two, held=held)['checks'])
        assert data == (out / f'{first}.pcm').read_bytes() + (out / f'{second}.pcm').read_bytes()
    assert inventory(development) == protected and inventory(proof / 'package') == package_files
    write(out / 'result.json', {'passed': True, 'proof': str(proof), 'player_sha256': digest(player),
        'project_sha256': prepared['project_sha256'], 'source_sha256': prepared['source_sha256'],
        'package_files': package_files, 'runs': list(actual), 'actual_processes': len(process.calls),
        'production_observer_absent': True, 'source_maps_build_removed': True, 'exact_restart_pcm': True,
        'limits': ['CPU ordinary shipping package only, no native-window/device/appearance gate',
                   'Two full wins exceed the existing 3600-host-tick CLI bound; complete win/loss restarts are tested in both directions',
                   'Whole-session reset is not physics save/restore', 'Historical0060 offscreen helper identity remains unresolved']})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest='mode', required=True)
    p = modes.add_parser('prepare'); p.add_argument('build', type=Path); p.add_argument('output', type=Path)
    p.add_argument('--binary', type=Path, required=True)
    p = modes.add_parser('ship'); p.add_argument('development', type=Path); p.add_argument('output', type=Path)
    p.add_argument('--existing-proof', type=Path)
    args = parser.parse_args(); (prepare if args.mode == 'prepare' else ship)(args)


if __name__ == '__main__':
    main()
