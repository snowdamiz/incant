#!/usr/bin/env python3
"""Actual public mixed-game export with CPU UI routing and retained atlas data.

Uses the ordinary packaged player, actual fonts, native motion and PCM. No GPU,
OS IME, visual approval, device measurement or complete game-UI claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
ENTITY = '00000000000000000000000002'
SCRIPT = '00000000000000000000000001'
CAMERA = '00000000000000000000000003'
UI_ROOT = '00000000000000000000000010'
BUTTON = '00000000000000000000000011'
FIELD = '00000000000000000000000012'


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def source():
    return f"""import type {{ NativeBehavior }} from '@incant/runtime/native';
export default {{ update({{ chunks, dt, input, log }}) {{
  let readonly = false;
  try {{ (input.ui.events as unknown as unknown[]).push(null); }} catch (_) {{ readonly = true; }}
  if (!readonly) throw Error('mutable UI notification sequence');
  for (const c of chunks) for (let i=0; i<c.ids.length; i++)
    if (c.ids[i] === '{ENTITY}') c.transforms[i*c.transformStride] += 6*dt;
  if (input.ui.focused !== null && (input.keyboard.held.includes('Enter') ||
      input.keyboard.held.includes('Tab') || input.keyboard.held.includes('Space')))
    throw Error('UI control leaked into gameplay');
  for (const event of input.ui.events)
    log(event.kind + ('value' in event ? ':' + event.value : ''));
}} }} satisfies NativeBehavior;
"""


def style(root=False):
    return {'size': ['auto', 'auto'] if root else [{'pixels': 160}, {'pixels': 32}],
            'min_size': [0, 0], 'max_size': [None, None], 'padding': [0, 0, 0, 0],
            'gap': [0, 0], 'direction': 'column', 'wrap': False, 'align': 'stretch',
            'justify': 'start', 'grow': 0, 'shrink': 0 if not root else 1, 'basis': 'auto'}


def recording():
    def key(code, down):
        return {'type': 'key', 'code': code, 'down': down}
    def pad(button, value):
        return {'type': 'gamepad_button', 'id': 1, 'button': button, 'value': value}
    return {'format': 'incant-input', 'version': 2, 'tick_rate': 60, 'start_tick': 0, 'ticks': 12,
        'frames': [
            {'tick': 1, 'events': [key('Tab', True), key('Enter', True)]},
            {'tick': 2, 'events': [key('Tab', False), key('Enter', False)],
             'ui': [{'type': 'focus', 'entity_id': FIELD}]},
            {'tick': 3, 'events': [], 'ui': [{'type': 'preedit', 'entity_id': FIELD, 'text': '中', 'cursor': [0, 3]}]},
            {'tick': 4, 'events': [], 'ui': [{'type': 'commit', 'entity_id': FIELD, 'text': '中文'}]},
            {'tick': 5, 'events': [key('Enter', True)]},
            {'tick': 6, 'events': [{'type': 'focus', 'focused': False}]},
            {'tick': 7, 'events': [{'type': 'focus', 'focused': True}, {'type': 'gamepad_connected', 'id': 1},
                                    pad('dpad_down', 1), pad('south', 1)]},
            {'tick': 8, 'events': [pad('dpad_down', 0), pad('south', 0)]},
            {'tick': 9, 'events': [], 'viewport': [640, 300], 'ui': [{'type': 'pointer', 'position': [10, 10], 'pressed': True}]},
            {'tick': 10, 'events': [], 'ui': [{'type': 'pointer', 'position': [10, 10], 'pressed': False}]},
            {'tick': 12, 'events': [], 'ui': [{'type': 'focus', 'entity_id': FIELD}, {'type': 'activate'}]},
        ]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    import export_native
    inputs = output / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'

    def execute(command, **kwargs):
        result = native_process.run(list(map(str, command)), **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    if args.author:
        execute([sys.executable, ROOT / 'tools/probes/native-text-export.py', output, '--binary', binary, '--author'], timeout=180)
        authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        rpc = Rpc(binary, project, journal)
        try:
            revision = rpc.call('project.read')['revision']
            previous = rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['version']
            commands = [{'op': 'write_script', 'script_id': SCRIPT, 'expected_source': previous, 'source': source()}]
            for entity, role, content in [(UI_ROOT, 'root', None), (BUTTON, 'button', 'Run'), (FIELD, 'text_input', 'Name')]:
                kind = {'role': role}
                if content is not None:
                    kind['text'] = {'content': content, 'fonts': authored['fonts'], 'font_size': 20,
                                    'line_height': 24, 'wrap': 'word_or_glyph'}
                commands.append({'op': 'create_entity', 'scene_id': authored['scene'], 'entity': {
                    'id': entity, 'name': role, 'parent': None if role == 'root' else UI_ROOT,
                    'provenance': None, 'components': {'UiNode': {'kind': kind, 'style': style(role == 'root'),
                        'order': 0, 'disabled': False}}}})
            result = rpc.call('command.execute', {'expected_revision': revision, 'description': 'Author connected native UI', 'commands': commands})
            require(result['revision'] == revision + 1, 'UI authoring was not atomic')
            accepted = rpc.call('project.read')
            rpc.call('history.undo')
            rpc.call('history.redo')
            require(rpc.call('project.read')['project'] == accepted['project'], 'UI Undo/Redo changed content')
            require(rpc.call('project.save')['saved'], 'UI project save failed')
            accepted = rpc.call('project.read')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(rpc.call('project.read') == accepted, 'UI journal reopen differs')
        finally:
            rpc.close()
        authored.update(project_sha256=digest(project), original_fixture_source_sha256=digest(inputs / 'move.ts'),
                        source_sha256=hashlib.sha256(source().encode('utf-8')).hexdigest(),
                        revision=accepted['revision'], ui_undo_redo=True)
        write(output / 'authoring.json', authored)
        return

    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, Path(__file__).resolve(), output, '--binary', binary, '--author'], timeout=240)
    authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
    exported = export_native.export(project, output / 'package', camera=CAMERA, ui_viewport='320x200')
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export identity/storage mismatch')
    shutil.rmtree(inputs)  # Only this newly created disposable fixture.
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    data = player.parent / 'data'
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    timeline = recording()
    timeline['tick_rate'] = authored['tick_rate']
    record = output / 'input.json'
    write(record, timeline)
    plan = output / 'observe.json'
    write(plan, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY], 'checks': [
        {'name': 'mixed-native-motion', 'tick': 12, 'expect': {'type': 'vector', 'entity': ENTITY,
         'property': 'translation', 'value': [72 / authored['tick_rate'], 0, 0], 'tolerance': 1e-9}}]})
    prefix = [player, '--package', data, '--ticks', '12']
    runs = []
    for index in range(3):
        pcm = output / f'run-{index}.pcm'
        value = json.loads(execute(prefix + ['--output', pcm, '--ui-viewport', '320x200', '--input-recording', record, '--observe', plan], cwd=player.parent, env=env, timeout=90).stdout)
        render = value['render_input']
        ui = render['ui']
        require(value['observations']['passed'] and render['tick'] == 12 and not render['gpu_submitted'], 'Wrong native CPU tick')
        require(ui['widgets'] == 3 and ui['viewport'] == [640, 300] and ui['focus'] == FIELD and ui['values'][FIELD] == 'Name中文', 'UI routing/resize/value mismatch')
        require(ui['glyphs'] >= 7 and render['text_count'] == 1 and render['model_count'] == 1, 'Missing mixed retained consumer')
        expected = [(1, 'activated'), (4, 'value_changed:Name中文'), (5, 'submitted:Name中文'), (7, 'activated'), (10, 'activated'), (12, 'submitted:Name中文')]
        require([(entry['tick'], entry['message']) for entry in value['logs']['entries']] == expected, 'Wrong accepted UI notifications')
        samples = struct.unpack('<' + 'f' * (pcm.stat().st_size // 4), pcm.read_bytes())
        require(any(v != 0 for v in samples), 'Mixed audio is silent')
        write(output / f'run-{index}.json', value)
        runs.append({'report': value, 'pcm_sha256': digest(pcm)})
    require(len({r['pcm_sha256'] for r in runs}) == 1 and len({r['report']['render_input']['ui']['atlas_sha256'] for r in runs}) == 1, 'Fresh process state differs')
    refusals = []
    def refuse(name, extra):
        pcm = output / f'refused-{name}.pcm'
        result = native_process.run(list(map(str, prefix + ['--output', pcm] + extra)), cwd=player.parent, env=env, timeout=90)
        require(result.returncode != 0 and not pcm.exists(), f'{name} was accepted/published')
        write(output / f'refused-{name}.json', {'returncode': result.returncode, 'stderr': result.stderr})
        refusals.append(name)
    refuse('unconfigured-consumer', ['--input-recording', record])
    invalid = output / 'invalid-input.json'
    malformed = recording()
    malformed['frames'][2]['ui'][0]['cursor'] = [1, 2]
    write(invalid, malformed)
    refuse('malformed-preedit', ['--ui-viewport', '320x200', '--input-recording', invalid])
    save = output / 'unsupported.save'
    refuse('unsupported-checkpoint', ['--ui-viewport', '320x200', '--save-checkpoint', save])
    require(not save.exists(), 'Unsupported UI checkpoint published')
    blob = next((data / 'fonts').glob('*.ifont'))
    original = blob.read_bytes()
    try:
        for name in ('missing-font', 'tampered-font'):
            if name == 'missing-font':
                blob.unlink()
            else:
                blob.write_bytes(original[:-1] + bytes([original[-1] ^ 1]))
            refuse(name, ['--ui-viewport', '320x200'])
            blob.write_bytes(original)
    finally:
        blob.write_bytes(original)
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'runs': runs, 'refusals': refusals,
        'authoring_deleted': not inputs.exists(), 'build_storage_removed': True, 'player_sha256': exported['player_sha256'],
        'limitations': ['CPU canonical UI only; GPU consumer explicitly unsupported', 'No OS input/IME, visual, phone or performance gate', 'UI checkpoint state remains unsupported']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
