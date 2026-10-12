#!/usr/bin/env python3
"""Public CPU package UI checkpoint continuation, with authored/build/map deletion.

Uses actual retained fonts/paint and the existing offline player. No GPU/window,
physical IME, audio cursor, raw solver or arbitrary VM-heap continuation claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
UI = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))
ENTITY, FIELD, SCRIPT, CAMERA = (UI[k] for k in ('ENTITY', 'FIELD', 'SCRIPT', 'CAMERA'))
write, digest, require = (UI[k] for k in ('write', 'digest', 'require'))


def source():
    return f"""import type {{ NativeBehavior }} from '@incant/runtime/native';
export default {{ persistence: {{version:1}}, update(t) {{
 let state=Number(t.state.get()??0);
 state += t.timers.events.length + t.input.ui.events.filter(e=>e.kind==='value_changed').length;
 t.state.set(state);
 if(t.clock.tick===1)t.timers.set('beat',3,3);
 for(const instance of t.instances) {{
  const count=Number(t.state.get(instance.entity)??0)+2;t.state.set(count,instance.entity);
  for(const c of t.chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]===instance.entity)c.transforms[i*c.transformStride]=count+state;
 }}
 if(t.clock.tick===14 && (t.input.keyboard.held.includes('KeyA') || t.input.keyboard.released.includes('KeyA')))
  throw Error('ordinary-load input was not neutral');
}} }} satisfies NativeBehavior;
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    output, binary = args.output.resolve(), args.binary.resolve(strict=True)
    sys.path.insert(0, str(ROOT / 'tools'))
    import export_native
    import native_process
    output.mkdir(parents=True, exist_ok=False)
    inputs = output / 'authoring'
    inputs.mkdir()
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'

    def execute(command, **kwargs):
        result = native_process.run(list(map(str, command)), **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    execute([binary, 'init', project, '--entities', '0'])
    execute([sys.executable, ROOT / 'tools/probes/native-ui-export.py', output,
             '--binary', binary, '--author'], timeout=240)
    authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        previous = rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['version']
        rpc.call('command.execute', {'expected_revision': revision, 'description': 'Author UI checkpoint game', 'commands': [
            {'op': 'remove_component', 'scene_id': authored['scene'], 'entity_id': ENTITY, 'component': 'AudioSource'},
            {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': previous, 'source': source()}]})
        accepted = rpc.call('project.read')
        rpc.call('history.undo')
        rpc.call('history.redo')
        require(rpc.call('project.read')['project'] == accepted['project'], 'Undo/Redo changed fixture')
        require(rpc.call('project.save')['saved'], 'Project save failed')
        accepted = rpc.call('project.read')
    finally:
        rpc.close()
    rpc = Rpc(binary, project, journal)
    try:
        require(rpc.call('project.read') == accepted, 'Journal reopen differs')
    finally:
        rpc.close()
    authored.update(project_sha256=digest(project), source_sha256=digest(inputs / 'move.ts'),
                    revision=accepted['revision'], checkpoint_undo_redo=True)
    write(output / 'authoring.json', authored)
    exported = export_native.export(project, output / 'package', camera=CAMERA, ui_viewport='320x200')
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export identity/storage mismatch')
    shutil.rmtree(inputs)
    removed_maps = UI['remove_debug_source_maps'](output / 'package')
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    data = player.parent / 'data'
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})

    def record(name, frames, ticks):
        path = output / (name + '-input.json')
        write(path, {'format': 'incant-input', 'version': 2, 'tick_rate': authored['tick_rate'],
                     'start_tick': 0, 'ticks': ticks, 'frames': frames})
        return path

    def key(code, down):
        return {'type': 'key', 'code': code, 'down': down}

    # Release A before the save boundary. The separate unit/process controls
    # exercise a held control at capture; this script also requires no false edge.
    frames = [
        {'tick': 1, 'events': [], 'ui': [{'type': 'focus', 'entity_id': FIELD}]},
        {'tick': 2, 'events': [], 'ui': [{'type': 'commit', 'entity_id': FIELD, 'text': 'e\u0301日本'}]},
        {'tick': 3, 'events': [key('KeyA', True)]},
        {'tick': 4, 'events': [key('KeyA', False)]},
        {'tick': 5, 'events': [key('ShiftLeft', True), key('ArrowLeft', True)]},
        {'tick': 6, 'events': [key('ShiftLeft', False), key('ArrowLeft', False)]},
    ]
    initial_record = record('initial', frames, 40)
    neutral_record = record('neutral', [], 27)

    def expected(tick):
        return tick * 2 + max(0, (tick - 1) // 3) + int(tick >= 2)

    def plan(name, ticks, wrong=False):
        path = output / (name + '-observe.json')
        write(path, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY], 'checks': [
            {'name': 'state-timer-ui-' + str(tick), 'tick': tick, 'expect': {'type': 'vector', 'entity': ENTITY,
             'property': 'translation', 'value': [expected(tick) + int(wrong), 0, 0], 'tolerance': 0}} for tick in ticks]})
        return path

    runs, refused = [], []

    def launch(name, ticks, load=None, save=None, recording=None, observe=None, success=True, pcm_absent=True):
        pcm = output / (name + '.pcm')
        command = [player, '--package', data, '--ticks', ticks, '--output', pcm, '--ui-viewport', '600x300' if load else '320x200']
        for flag, value in [('--load-checkpoint', load), ('--save-checkpoint', save), ('--input-recording', recording), ('--observe', observe)]:
            if value is not None:
                command += [flag, value]
        result = native_process.run(list(map(str, command)), cwd=player.parent, env=env, timeout=90)
        require((result.returncode == 0) == success, name + ': ' + result.stderr[-4000:])
        receipt = {'name': name, 'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        write(output / (name + '-receipt.json'), receipt)
        if not success:
            require(not pcm_absent or not pcm.exists(), name + ': PCM published')
            require(save is None or name == 'no-clobber' or not save.exists(), name + ': save published')
            refused.append(receipt)
            return
        value = json.loads(result.stdout)
        require(value['observations']['passed'], 'Observation failure')
        ui = value['render_input']['ui']
        require(ui['focus'] == FIELD and ui['values'][FIELD] == 'Namee\u0301日本', 'Restored widget data differs')
        require(value['render_input']['first_model_position'] == [expected(value['tick']), 0, 0], 'Retained pose differs')
        require(not value['render_input']['gpu_submitted'] and set(pcm.read_bytes()) == {0}, 'Unexpected GPU or audio')
        runs.append({'name': name, 'report': value, 'save_sha256': digest(save), 'pcm_bytes': pcm.stat().st_size})

    baseline, middle, resumed, repeated = [output / (name + '.save') for name in ('baseline', 'middle', 'resumed', 'repeated')]
    launch('baseline', 40, save=baseline, recording=initial_record, observe=plan('baseline', [0, 13, 40]))
    launch('middle', 13, save=middle, recording=initial_record, observe=plan('middle', [13]))
    for name, save in [('resumed', resumed), ('repeated', repeated)]:
        launch(name, 27, load=middle, save=save, recording=neutral_record, observe=plan(name, [13, 40]))
    require(baseline.read_bytes() == resumed.read_bytes() == repeated.read_bytes(), 'Final checkpoint continuation differs')
    require((output / 'baseline.pcm').read_bytes() == (output / 'middle.pcm').read_bytes() + (output / 'resumed.pcm').read_bytes(), 'PCM clock differs')
    corrupt = output / 'corrupt.save'
    content = bytearray(middle.read_bytes()); content[-1] ^= 1; corrupt.write_bytes(content)
    launch('corrupt', 1, load=corrupt, success=False)
    mismatch = output / 'wrong-program.save'
    content = bytearray(middle.read_bytes()); content[16] ^= 1
    content[100:132] = hashlib.sha256(content[:100] + content[132:]).digest(); mismatch.write_bytes(content)
    launch('wrong-program', 1, load=mismatch, success=False)
    malformed = output / 'wrong-widget.save'
    content = bytearray(middle.read_bytes())
    start = 132 + sum(int.from_bytes(content[80+4*i:84+4*i], 'little') for i in range(4))
    content[start + 80] ^= 1
    content[start+48:start+80] = hashlib.sha256(content[start:start+48] + content[start+80:]).digest()
    content[100:132] = hashlib.sha256(content[:100] + content[132:]).digest(); malformed.write_bytes(content)
    launch('wrong-widget', 1, load=malformed, success=False)
    oversized = output / 'oversized.save'
    with oversized.open('wb') as file:
        file.truncate(256 * 1024 * 1024 + 1)
    launch('oversized', 1, load=oversized, success=False)
    launch('assertion', 1, load=middle, save=output / 'assertion.save', observe=plan('assertion', [14], True), success=False)
    preedit = record('preedit', [{'tick': 1, 'events': [], 'ui': [{'type': 'preedit', 'entity_id': FIELD, 'text': '候補', 'cursor': [0, 3]}]}], 1)
    launch('preedit', 1, load=middle, save=output / 'preedit.save', recording=preedit, success=False)
    sentinel = output / 'existing.save'; sentinel.write_bytes(b'preserve existing')
    launch('no-clobber', 1, load=middle, save=sentinel, success=False, pcm_absent=False)
    require(sentinel.read_bytes() == b'preserve existing' and not list(output.glob('.incant-checkpoint-*')), 'Save publication damaged existing data')
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'export': exported, 'runs': runs, 'refusals': refused,
        'authoring_deleted': not inputs.exists(), 'debug_source_maps_removed': removed_maps, 'player_sha256': digest(player),
        'limits': ['CPU hosted ordinary-load UI only; new owner/router and neutral physical input',
                   'Active preedit refuses; pointer capture is discarded', 'Physics/audio/static/development-module saves still refuse',
                   'No GPU/window/native-IME or full-save gate claim; PCM and save are not a multi-file transaction']})
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
