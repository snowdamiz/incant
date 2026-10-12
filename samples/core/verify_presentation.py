#!/usr/bin/env python3
"""Actual native Core gameplay with numerical HUD/audio fixtures; no visual claim."""
import argparse
import sys
import hashlib
import json
from pathlib import Path
import runpy
import shutil
import struct
import subprocess
import wave

ROOT = Path(__file__).resolve().parents[2]
BASE = runpy.run_path(str(ROOT / 'samples/core/verify.py'))
ID = BASE['ID']
HUD = dict(zip(('table', 'health', 'collected', 'objective', 'won', 'lost', 'result', 'inactive'), map(ID, range(100, 108))))
AUDIO = dict(zip(('music', 'pickup', 'damage', 'won', 'lost'), map(ID, range(110, 115))))
UI_ROOT, CAMERA = ID(108), ID(109)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')


def observed_source(source):
    # Observe committed host declarations without replacing any gameplay/host method.
    return source.replace('export default {', 'const behavior = {', 1) + '''
let observed = '';
export default { ...behavior, update(context: NativeContext) {
  const actual = context.hud.widgets.filter(w => w.text !== null)
    .map(w => [w.id, w.text]);
  const encoded = JSON.stringify(actual);
  if (encoded !== observed) { context.log('fixture.hud ' + encoded); observed = encoded; }
  behavior.update(context);
  if (context.clock.tick === 50) context.commands.setLocale({ locale: 'fr' });
} } satisfies NativeBehavior;
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--existing-proof', type=Path, help='Reuse the exact exported package in a new result directory')
    parser.add_argument('--base-fixture', type=Path)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    inputs = proof / 'authoring'
    binary = args.binary.resolve(strict=True) if args.binary else None
    project, journal = inputs / 'core.json', inputs / 'core.journal.jsonl'
    original = (ROOT / 'samples/core/gameplay.ts').read_text()
    processes = []

    def execute(argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, text=True, capture_output=True, timeout=300)
        write(out / f'process-{len(processes):02d}.json', {'argv': list(map(str, argv)), 'exit': result.returncode,
              'stdout': result.stdout, 'stderr': result.stderr})
        processes.append(result.returncode)
        if result.returncode: raise RuntimeError(result.stderr[-8000:])
        return json.loads(result.stdout)

    if not args.existing_proof:
        assert binary is not None and args.base_fixture is not None, 'Fresh authoring requires binary and base fixture'
        inputs.mkdir()
        for name in ('core.json', 'gameplay.ts'):
            shutil.copyfile(args.base_fixture / 'authoring' / name, inputs / name)
        font_name = 'NotoSans-Regular.ttf'
        shutil.copyfile(ROOT / 'crates/incant_text/tests/fonts' / font_name, inputs / font_name)
        font = execute([binary, 'import', project, font_name])['asset']['id']
        clips = {}
        for i, role in enumerate(AUDIO):
            name = role + '.wav'
            with wave.open(str(inputs / name), 'wb') as audio:
                audio.setnchannels(1); audio.setsampwidth(2); audio.setframerate(48000)
                audio.writeframes(b''.join(struct.pack('<h', (500 + i * 300) * (1 if n % (60 + 20*i) < 30 + 10*i else -1)) for n in range(48000)))
            clips[role] = execute([binary, 'import', project, name])['asset']['id']
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        rpc = Rpc(binary, project, journal); calls = []

        def call(method, params=None):
            result = rpc.call(method, params); calls.append({'method': method, 'params': params, 'result': result})
            return result

        original = (ROOT / 'samples/core/gameplay.ts').read_text()
        try:
            before = call('project.read'); scene = next(iter(before['project']['scenes']))
            props = before['project']['scenes'][scene]['entities'][BASE['CONTROLLER']]['components']['Script']['props']
            props = {**props, 'hud': HUD, 'audio': AUDIO}
            script = call('script.read', {'script_id': BASE['SCRIPT']})['untrusted_source_data']
            commands = [
                {'op': 'write_script', 'script_id': BASE['SCRIPT'], 'expected_source': script['version'], 'source': observed_source(original)},
                {'op': 'set_component', 'scene_id': scene, 'entity_id': BASE['CONTROLLER'], 'component': 'Script',
                 'value': {'source': BASE['SCRIPT'], 'props': props}},
                {'op': 'upsert_string_table', 'table': {'id': HUD['table'], 'name': 'Fixture HUD', 'source_locale': 'en', 'messages': {
                    'hud.health': {'en': 'Health {health}', 'fr': 'Santé {health}'},
                    'hud.samples': {'en': 'Samples {collected}/{total}', 'fr': 'Échantillons {collected}/{total}'},
                    'hud.objective.collect': {'en': 'Collect samples', 'fr': 'Collecter'},
                    'hud.objective.exit': {'en': 'Reach exit', 'fr': 'Sortir'},
                    'hud.won': {'en': 'Won', 'fr': 'Gagné'}, 'hud.lost': {'en': 'Lost', 'fr': 'Perdu'}}}},
                {'op': 'set_locale', 'settings': {'locale': 'en'}}]
            style = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['style']
            def entity(owner, name, components, parent=None):
                return {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': owner, 'name': name, 'parent': parent,
                        'provenance': None, 'components': components}}
            commands.append(entity(UI_ROOT, 'Numerical UI root', {'UiNode': {'kind': {'role': 'root'}, 'style': style(True), 'order': 0, 'disabled': False}}))
            for role in ('health', 'collected', 'objective', 'won', 'lost', 'result', 'inactive'):
                kind = {'role': 'container'} if role in ('result', 'inactive') else {'role': 'label', 'text': {
                    'content': 'Pending', 'fonts': [font], 'font_size': 20, 'line_height': 24, 'wrap': 'word_or_glyph'}}
                parent = HUD['inactive'] if role in ('won', 'lost') else UI_ROOT
                commands.append(entity(HUD[role], role, {'UiNode': {'kind': kind, 'style': style(), 'order': 0, 'disabled': False}}, parent))
            commands.append(entity(CAMERA, 'Fixture camera', {'Transform': BASE['transform']([0, 5, 15]),
                'Camera': {'fov_degrees': 60, 'near': .1, 'far': 100, 'projection': {'kind': 'perspective'}}}))
            for role, owner in AUDIO.items():
                commands.append(entity(owner, role, {'Transform': BASE['transform']([0, 0, 0]), 'AudioSource': {
                    'clip': clips[role], 'gain_db': 0, 'pan': 0, 'rate': 1, 'start_seconds': 0,
                    'playing': False, 'looping': role == 'music', 'streaming': role == 'music'}}))
            call('command.execute', {'expected_revision': before['revision'], 'description': 'Author Core presentation correctness fixture', 'commands': commands})
            authored = call('project.read'); call('history.undo'); assert call('project.read')['project'] == before['project']
            call('history.redo'); assert call('project.read')['project'] == authored['project']
            call('project.save'); authored = call('project.read')
        finally:
            rpc.close(); write(out / 'authoring-calls.json', calls)
        rpc = Rpc(binary, project, journal)
        try: assert call('project.read') == authored
        finally: rpc.close()
        frozen = {p.name: digest(p) for p in inputs.iterdir() if p.is_file()}
        sys.path.insert(0, str(ROOT / 'tools'))
        import export_native
        exported = export_native.export(project, out / 'package', camera=CAMERA, ui_viewport='640x480')
        assert exported['build_storage_removed_before_startup']
        assert {p.name: digest(p) for p in inputs.iterdir() if p.is_file()} == frozen
        write(out / 'deleted-authoring.json', {str(p.relative_to(inputs)): digest(p) for p in inputs.rglob('*') if p.is_file()})
        shutil.rmtree(inputs)  # This invocation's disposable fixture only.
        remove_maps = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(out / 'deleted-maps.json', remove_maps(out / 'package'))
    exported = json.loads((proof / 'package/export.json').read_text())
    deleted = json.loads((proof / 'deleted-authoring.json').read_text())
    assert exported['project_sha256'] == deleted['core.json'] and exported['build_storage_removed_before_startup']
    assert deleted['gameplay.ts'] == hashlib.sha256(observed_source(original).encode()).hexdigest()
    assert not inputs.exists() and not list((proof / 'package').rglob('*.map'))
    player = proof / 'package/player/incant_game_player'
    assert digest(player) == exported['player_sha256']
    package_hashes = {str(p.relative_to(proof / 'package')): digest(p) for p in (proof / 'package').rglob('*') if p.is_file()}
    write(out / 'package-before.json', package_hashes)
    results = {}
    for name in ('win', 'lose', 'repeat-win'):
        timeline = BASE['replay']('win' if name == 'repeat-win' else name)
        path = out / f'{name}-input.json'; write(path, timeline)
        observe = BASE['observations'](name, timeline['ticks'])
        observe['checks'][0]['expect']['value'] += 14
        for role in ('won', 'lost'):
            observe['entities'].append(HUD[role])
        plan = out / f'{name}-observe.json'; write(plan, observe)
        report = execute([player, '--package', player.parent / 'data', '--ticks', timeline['ticks'], '--input-recording', path,
                          '--observe', plan, '--ui-viewport', '640x480', '--output', out / f'{name}.pcm'])
        write(out / f'{name}.json', report); results[name] = report
        assert report['observations']['passed'] and not report['logs']['dropped']
        entries = report['logs']['entries']
        hud = [(r['tick'], dict(json.loads(r['message'].removeprefix('fixture.hud ')))) for r in entries if r['message'].startswith('fixture.hud ')]
        assert hud[0][0] == 1 and hud[0][1][HUD['health']] == 'Pending'
        assert hud[1][0] == 2 and hud[1][1][HUD['health']] == 'Health 3'
        assert any(t == 52 and v[HUD['health']].startswith('Santé') for t, v in hud), hud
        for entry in entries:
            message = entry['message']
            if message.startswith('core.pickup'):
                count = int(message.rsplit(' ', 1)[1].split('/')[0])
                assert any(t == entry['tick'] + 1 and str(count) + '/2' in v[HUD['collected']] for t, v in hud)
            if message.startswith('core.damage'):
                health = message.rsplit('=', 1)[1]
                assert any(t == entry['tick'] + 1 and v[HUD['health']] == 'Santé ' + health for t, v in hud)
        won = name != 'lose'; selected = 'won' if won else 'lost'
        assert hud[-1][1][HUD['health']] == ('Santé 2' if won else 'Santé 0')
        assert hud[-1][1][HUD['objective']] == ('Sortir' if won else 'Collecter')
        for role in ('won', 'lost'):
            assert report['observations']['entities'][HUD[role]]['parent'] == HUD['result' if role == selected else 'inactive']
        pcm = (out / f'{name}.pcm').read_bytes(); assert any(pcm[:6400]) and not any(pcm[-6400*60:])
        assert digest(out / f'{name}.pcm') == report['pcm_sha256']
    assert (out / 'win.pcm').read_bytes() == (out / 'repeat-win.pcm').read_bytes()
    assert results['win']['logs'] == results['repeat-win']['logs']
    assert package_hashes == {str(p.relative_to(proof / 'package')): digest(p) for p in (proof / 'package').rglob('*') if p.is_file()}
    write(out / 'result.json', {'passed': True, 'gameplay_sha256': hashlib.sha256(original.encode()).hexdigest(),
          'binary_sha256': digest(binary) if binary else None, 'proof_root': str(proof), 'authoring_undo_redo_reopen': True, 'source_removed_before_runs': True, 'player_sha256': digest(player),
          'runs': list(results), 'real_native_hud_updates': True, 'locale_change_tick': 50,
          'exact_repeat_pcm': True, 'terminal_music_stops': True,
          'limits': ['Numerical UI fixture, no visual approval', 'Observed test wrapper; production gameplay preserved',
                     'Ordinary CPU exported numerical fixture; authored visual level and physics saves remain separate']})
    print(json.dumps({'passed': True, 'runs': len(results), 'output': str(out)}))


if __name__ == '__main__':
    main()
