#!/usr/bin/env python3
"""Public mixed static/streamed audio + UI checkpoint continuation.

Uses actual retained fonts/paint and the existing offline player. No GPU/window,
physical IME, device-audio, raw solver or arbitrary VM-heap continuation claim.
"""
import argparse
import hashlib
import json
import os
import re
import math
import struct
import wave
from pathlib import Path
import runpy
import shutil
import sys

ROOT = Path(__file__).resolve().parents[2]
UI = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))
ENTITY, FIELD, SCRIPT, CAMERA = (UI[k] for k in ('ENTITY', 'FIELD', 'SCRIPT', 'CAMERA'))
PARENT_BUS = '00000000000000000000000006'
STREAM, BUS = '00000000000000000000000004', '00000000000000000000000005'
write, digest, require = (UI[k] for k in ('write', 'digest', 'require'))


def source(transport=False):
    playback = f"""
 if(t.clock.tick===5){{t.commands.stopAudio('{ENTITY}');t.commands.stopAudio('{STREAM}');}}
 if(t.clock.tick===6){{t.commands.seekAudio('{ENTITY}',.01);t.commands.seekAudio('{STREAM}',.02);}}
 if(t.clock.tick===9){{t.commands.restartAudio('{ENTITY}');t.commands.restartAudio('{STREAM}');}}
 if(t.clock.tick===12)t.commands.seekAudio('{STREAM}',.01);
 if([15,25,33].includes(t.clock.tick))t.commands.restartAudio('{ENTITY}');
""" if transport else ''
    return f"""import type {{ NativeBehavior }} from '@incant/runtime/native';
export default {{ persistence: {{version:1}}, update(t) {{
 {playback}
 let state=Number(t.state.get()??0);
 state += t.timers.events.length + t.input.ui.events.filter(e=>e.kind==='value_changed').length;
 t.state.set(state);
 if(t.clock.tick===1)t.timers.set('beat',3,3);
 if(t.clock.tick===14)t.commands.setAudioPlaying('{ENTITY}',false);
 if(t.clock.tick===18)t.commands.setAudioPlaying('{ENTITY}',true);
 if(t.clock.tick===21){{t.commands.setAudioGain('{STREAM}',-8);t.commands.setAudioBusGain('{BUS}',-6);}}
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
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--existing-package', type=Path, help='Reverify a frozen public proof package; no build or authoring')
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    parser.add_argument('--transport', action='store_true', help='Exercise explicit stop/restart/seek and transport-bound checkpoints')
    args = parser.parse_args()
    output = args.output.resolve()
    sys.path.insert(0, str(ROOT / 'tools'))
    import export_native
    import native_process
    output.mkdir(parents=True, exist_ok=False)
    if args.existing_package:
        require(args.binary is None and not args.author, 'Existing-package mode cannot author')
        package = args.existing_package.resolve(strict=True)
        original = package.parent
        authored = json.loads((original / 'authoring.json').read_text(encoding='utf-8'))
        exported = json.loads((package / 'export.json').read_text(encoding='utf-8'))
        prior = json.loads((original / 'result.json').read_text(encoding='utf-8'))
        verify(output, package, authored, exported, original / 'authoring',
               prior['debug_source_maps_removed'], {'mode': 'existing-package',
               'preliminary_result_sha256': digest(original / 'result.json'),
               'helper_sha256': digest(Path(__file__))})
        return
    require(args.binary is not None, 'Fresh export requires --binary')
    binary = args.binary.resolve(strict=True)
    inputs = output / 'authoring'
    inputs.mkdir()
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'

    commands = []
    def execute(command, **kwargs):
        argv = list(map(str, command))
        result = native_process.run(argv, **kwargs)
        commands.append({'argv': argv, 'returncode': result.returncode,
                         'stdout': result.stdout, 'stderr': result.stderr})
        write(output / 'authoring-commands.json', commands)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    execute([binary, 'init', project, '--entities', '0'])
    clips = []
    for name, rate in [('static', 22050), ('stream', 32000)]:
        path = inputs / (name + '.wav')
        with wave.open(str(path), 'wb') as audio:
            audio.setnchannels(2); audio.setsampwidth(2); audio.setframerate(rate)
            audio.writeframes(b''.join(struct.pack('<hh', int(7000 * math.sin(i * .13)),
                int(6000 * math.cos(i * .17))) for i in range(2309)))
        clips.append(json.loads(execute([binary, 'import', project, path.name]).stdout)['asset']['id'])
    execute([sys.executable, ROOT / 'tools/probes/native-ui-export.py', output,
             '--binary', binary, '--author'], timeout=240)
    authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
    Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
    BaseRpc = Rpc
    class Rpc(BaseRpc):
        def __init__(self, binary, project, journal):
            super().__init__(binary, project, journal)
            self.trace = {'argv': self.process.args, 'calls': []}
            commands.append(self.trace)
        def call(self, method, params=None):
            result = super().call(method, params)
            self.trace['calls'].append({'method': method, 'params': params, 'result': result})
            write(output / 'authoring-commands.json', commands)
            return result
        def close(self):
            super().close()
            self.trace['returncode'] = self.process.returncode
            write(output / 'authoring-commands.json', commands)
    def audio_source(index):
        return {'clip': clips[index], 'bus': BUS, 'gain_db': -4, 'pan': 0 if index else .2,
                'rate': .87 if index else 1.13, 'start_seconds': .00037,
                'playing': True, 'looping': not args.transport or bool(index), 'streaming': bool(index),
                'spatial': {'min_distance': 1, 'max_distance': 30} if index else None}

    rpc = Rpc(binary, project, journal)
    try:
        revision = rpc.call('project.read')['revision']
        previous = rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['version']
        rpc.call('command.execute', {'expected_revision': revision, 'description': 'Author mixed audio checkpoint game', 'commands': [
            {'op': 'set_component', 'scene_id': authored['scene'], 'entity_id': ENTITY,
             'component': 'AudioSource', 'value': audio_source(0)},
            {'op': 'create_entity', 'scene_id': authored['scene'], 'entity': {'id': BUS, 'name': 'Mix',
             'parent': None, 'provenance': None, 'components': {'AudioBus': {'parent': PARENT_BUS, 'gain_db': -3}}}},
            {'op': 'create_entity', 'scene_id': authored['scene'], 'entity': {'id': PARENT_BUS, 'name': 'Parent bus',
             'parent': None, 'provenance': None, 'components': {'AudioBus': {'parent': None, 'gain_db': -1}}}},
            {'op': 'set_component', 'scene_id': authored['scene'], 'entity_id': CAMERA,
             'component': 'AudioListener', 'value': {}},
            {'op': 'create_entity', 'scene_id': authored['scene'], 'entity': {'id': STREAM, 'name': 'Stream',
             'parent': None, 'provenance': None, 'components': {'AudioSource': audio_source(1),
             'Transform': {'translation': [-1,0,0], 'rotation': [0,0,0,1], 'scale': [1,1,1]}}}},
            {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': previous, 'source': source(args.transport)}]})
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
                    revision=accepted['revision'], checkpoint_undo_redo=True, audio_clips=clips,
                    audio_source_rates=[22050, 32000], audio_source_frames=2309,
                    explicit_transport=args.transport)
    write(output / 'authoring.json', authored)
    if args.author:
        print(json.dumps({'passed': True, 'authoring': str(output / 'authoring.json')}))
        return
    exported = export_native.export(project, output / 'package', camera=CAMERA, ui_viewport='320x200')
    write(output / 'export-call.json', {'api': 'tools.export_native.export',
        'project': str(project), 'output': str(output / 'package'), 'camera': CAMERA,
        'ui_viewport': '320x200', 'completed': True, 'receipt': exported})
    require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export identity/storage mismatch')
    shutil.rmtree(inputs)
    removed_maps = UI['remove_debug_source_maps'](output / 'package')
    verify(output, output / 'package', authored, exported, inputs, removed_maps,
           {'mode': 'fresh-export', 'helper_sha256': digest(Path(__file__))})


def verify(output, package, authored, exported, original_inputs, removed_maps, provenance):
    import native_process
    player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    data = player.parent / 'data'
    audio_data = []
    for path in sorted((data / 'audio').glob('*.json')):
        metadata = json.loads(path.read_text(encoding='utf-8'))
        payload = data / 'audio' / (metadata['content_sha256'] + '.ipcm')
        require(metadata['frames'] == 2309 and metadata['channels'] == 2
                and digest(payload) == metadata['content_sha256'], 'Packaged audio differs')
        audio_data.append({'metadata': metadata, 'manifest_sha256': digest(path),
                           'payload_sha256': digest(payload)})
    require(sorted(v['metadata']['sample_rate'] for v in audio_data) == [22050, 32000],
            'Source sample rates were not preserved')
    require(not original_inputs.exists() and not list(package.rglob('*.js.map')),
            'Authored inputs or source maps remain')
    package_files = {str(path.relative_to(package)): {'sha256': digest(path), 'bytes': path.stat().st_size}
                     for path in sorted((package / 'player').rglob('*')) if path.is_file()}
    require(digest(player) == exported['player_sha256'], 'Player identity differs from fresh export')
    env = {'PATH': str(player.parent), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
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
    diagnostics = {
        'corrupt': 'Error: Checkpoint(Integrity)',
        'wrong-program': 'Error: Checkpoint(ProgramMismatch)',
        'wrong-widget': 'Error: Checkpoint(Ui(Invalid("widget order or duplicate")))',
        'wrong-audio-catalog': 'Error: Audio(Invalid("audio package catalog"))',
        'wrong-audio-clock': 'Error: Checkpoint(Audio(Invalid("audio/world sample clock")))',
        'wrong-audio-transport': 'Error: Checkpoint(Audio(Invalid("audio source state")))',
        'oversized': 'Error: "runtime input size limit"',
        'assertion': 'Error: "checkpoint withheld: native observation assertions failed"',
        'preedit': 'Error: Checkpoint(Ui(CompositionActive))',
    }

    def launch(name, ticks, load=None, save=None, recording=None, observe=None, success=True, pcm_absent=True):
        pcm = output / (name + '.pcm')
        command = [player, '--package', data, '--ticks', ticks, '--output', pcm, '--ui-viewport', '600x300' if load else '320x200']
        for flag, value in [('--load-checkpoint', load), ('--save-checkpoint', save), ('--input-recording', recording), ('--observe', observe)]:
            if value is not None:
                command += [flag, value]
        result = native_process.run(list(map(str, command)), cwd=player.parent, env=env, timeout=90)
        require((result.returncode == 0) == success, name + ': ' + result.stderr[-4000:])
        receipt = {'name': name, 'argv': list(map(str, command)), 'cwd': str(player.parent),
                   'environment_keys': sorted(env), 'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        write(output / (name + '-receipt.json'), receipt)
        if not success:
            error = result.stderr.strip()
            exact = error == diagnostics[name] if name in diagnostics else bool(re.fullmatch(
                r'Error: Os \{ code: \d+, kind: AlreadyExists, message: "[^"\n]*" \}', error))
            require(result.returncode == 1 and exact, name + ': wrong refusal diagnostic: ' + error)
            receipt['matched_diagnostic'] = error
            write(output / (name + '-receipt.json'), receipt)
            require(not pcm_absent or not pcm.exists(), name + ': PCM published')
            require(save is None or name == 'no-clobber' or not save.exists(), name + ': save published')
            refused.append(receipt)
            return
        value = json.loads(result.stdout)
        require(value['observations']['passed'], 'Observation failure')
        ui = value['render_input']['ui']
        require(ui['focus'] == FIELD and ui['values'][FIELD] == 'Namee\u0301日本', 'Restored widget data differs')
        require(value['render_input']['first_model_position'] == [expected(value['tick']), 0, 0], 'Retained pose differs')
        samples = struct.unpack('<' + 'f' * (pcm.stat().st_size // 4), pcm.read_bytes())
        require(not value['render_input']['gpu_submitted'] and any(abs(v) > .001 for v in samples), 'Missing real PCM or unexpected GPU')
        require(all(math.isfinite(v) for v in samples), 'Nonfinite PCM')
        runs.append({'name': name, 'report': value, 'save_sha256': digest(save), 'pcm_bytes': pcm.stat().st_size, 'pcm_sha256': digest(pcm)})

    baseline, middle, resumed, repeated = [output / (name + '.save') for name in ('baseline', 'middle', 'resumed', 'repeated')]
    launch('baseline', 40, save=baseline, recording=initial_record, observe=plan('baseline', [0, 13, 40]))
    launch('middle', 13, save=middle, recording=initial_record, observe=plan('middle', [13]))
    for name, save in [('resumed', resumed), ('repeated', repeated)]:
        launch(name, 27, load=middle, save=save, recording=neutral_record, observe=plan(name, [13, 40]))
    require(baseline.read_bytes() == resumed.read_bytes() == repeated.read_bytes(), 'Final checkpoint continuation differs')
    require((output / 'baseline.pcm').read_bytes() == (output / 'middle.pcm').read_bytes() + (output / 'resumed.pcm').read_bytes(), 'PCM clock differs')
    if authored.get('explicit_transport', False):
        pcm = (output / 'baseline.pcm').read_bytes()
        frames_per_tick = runs[0]['report']['frames'] // 40
        # Both voices stop at tick5. Seeking at tick6 must not resume them;
        # the old voice has a full tick to retire before this silent interval.
        silent = pcm[5 * frames_per_tick * 8:8 * frames_per_tick * 8]
        require(silent and not any(silent), 'Paused seek resumed playback')
        restarted = pcm[9 * frames_per_tick * 8:11 * frames_per_tick * 8]
        require(any(abs(v) > .001 for (v,) in struct.iter_unpack('<f', restarted)),
                'Explicit restart did not produce PCM')
        content = bytearray(middle.read_bytes())
        start = 136 + sum(int.from_bytes(content[80+4*i:84+4*i], 'little') for i in range(5))
        require(int.from_bytes(content[start+8:start+12], 'little') == 2, 'Transport checkpoint is not version2')
        catalog_count = int.from_bytes(content[start+156:start+158], 'little')
        epoch = start + 165 + catalog_count * 92 + 16
        content[epoch] ^= 1
        content[start+16:start+48] = hashlib.sha256(content[start:start+16] + content[start+48:]).digest()
        content[104:136] = hashlib.sha256(content[:104] + content[136:]).digest()
        wrong = output / 'wrong-audio-transport.save'; wrong.write_bytes(content)
        launch('wrong-audio-transport', 1, load=wrong, success=False)
    corrupt = output / 'corrupt.save'
    content = bytearray(middle.read_bytes()); content[-1] ^= 1; corrupt.write_bytes(content)
    launch('corrupt', 1, load=corrupt, success=False)
    mismatch = output / 'wrong-program.save'
    content = bytearray(middle.read_bytes()); content[16] ^= 1
    content[104:136] = hashlib.sha256(content[:104] + content[136:]).digest(); mismatch.write_bytes(content)
    launch('wrong-program', 1, load=mismatch, success=False)
    malformed = output / 'wrong-widget.save'
    content = bytearray(middle.read_bytes())
    start = 136 + sum(int.from_bytes(content[80+4*i:84+4*i], 'little') for i in range(4))
    content[start + 80] ^= 1
    end = start + int.from_bytes(content[96:100], 'little')
    content[start+48:start+80] = hashlib.sha256(content[start:start+48] + content[start+80:end]).digest()
    content[104:136] = hashlib.sha256(content[:104] + content[136:]).digest(); malformed.write_bytes(content)
    launch('wrong-widget', 1, load=malformed, success=False)
    # Recomputed checksums are not authority: a different immutable audio
    # catalog or impossible session sample clock must still fail before output.
    for name, offset in [('wrong-audio-catalog', 192), ('wrong-audio-clock', 108)]:
        bad = output / (name + '.save')
        content = bytearray(middle.read_bytes())
        start = 136 + sum(int.from_bytes(content[80+4*i:84+4*i], 'little') for i in range(5))
        content[start + offset] ^= 1
        content[start+16:start+48] = hashlib.sha256(content[start:start+16] + content[start+48:]).digest()
        content[104:136] = hashlib.sha256(content[:104] + content[136:]).digest(); bad.write_bytes(content)
        launch(name, 1, load=bad, success=False)
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
    require(all(digest(package / name) == value['sha256'] for name, value in package_files.items()),
            'Player or package data mutated during controls')
    write(output / 'result.json', {'passed': True, 'provenance': provenance, 'package_files': package_files,
        'authoring': authored, 'export': exported, 'packaged_audio': audio_data, 'runs': runs, 'refusals': refused,
        'authoring_deleted': not original_inputs.exists(), 'debug_source_maps_removed': removed_maps, 'player_sha256': digest(player),
        'limits': ['CPU hosted ordinary-load mixed static/streamed audio and UI; fresh owners and neutral input',
                   'Active preedit refuses; pointer capture is discarded', 'Physics/static/development-module and device-native audio saves still refuse',
                   'No GPU/window/native-IME or full-save gate claim; PCM and save are not a multi-file transaction']})
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
