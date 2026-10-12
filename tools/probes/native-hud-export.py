#!/usr/bin/env python3
"""Public authored HUD controls, retained shaping and ordinary packaged playback.

CPU observations only. Source/maps/build inputs are removed before fresh player
processes. This does not exercise native windows, GPU drawing or Wasm HUD APIs.
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
TABLE = '00000000000000000000000050'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source():
    return f"""import type {{ NativeBehavior }} from '@incant/runtime/native';
export default {{ persistence: {{ version: 1 }}, update({{ chunks, dt, commands, clock, hud, localization, input, log }}) {{
  const n=clock.tick, button=hud.widgets.find(w=>w.id==='{BUTTON}')!;
  const field=hud.widgets.find(w=>w.id==='{FIELD}')!, text=hud.texts.find(w=>w.id==='{ENTITY}')!;
  const prior=n===1?'Run':(n<=7?'Health ':'Santé ')+(21-n);
  if(button.text!==prior || button.disabled!==(n===6 || n===7)) throw Error('precommand HUD');
  if(field.text!=='Name' || text.text!==(n===1?'ffi سلام 中文':'Pickup '+(n-1))) throw Error('precommand text');
  for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='{ENTITY}')c.transforms[i*c.transformStride]+=6*dt;
  const next=localization.localize({{table_id:'{TABLE}',key:'health',arguments:{{n:20-n}}}}).text;
  commands.setUiText('{BUTTON}',next);
  commands.setText('{ENTITY}','Pickup '+n);
  if(n===5)commands.setUiDisabled('{BUTTON}',true);
  if(n===7)commands.setUiDisabled('{BUTTON}',false);
  if(n===6)commands.setLocale({{locale:'fr'}});
  if(n===12)commands.setUiText('{FIELD}','Done');
  if(button.text!==prior || text.text!==(n===1?'ffi سلام 中文':'Pickup '+(n-1)))throw Error('early HUD publication');
  if(input.keyboard.held.includes('F1'))commands.setUiText('{UI_ROOT}','invalid');
  if(input.keyboard.held.includes('F2'))throw Error('HUD deliberate trap');
  if(input.keyboard.held.includes('F3')){{for(let i=0;i<257;i++)try{{commands.setUiDisabled('{BUTTON}',false)}}catch{{}}}}
  log(n+'|'+prior+'|'+text.text+'|'+button.disabled);
}} }} satisfies NativeBehavior;
"""


def recording(rate, ticks=12):
    return {'format': 'incant-input', 'version': 2, 'tick_rate': rate, 'start_tick': 0, 'ticks': ticks,
            'frames': [{'tick': 2, 'events': [], 'ui': [{'type': 'focus', 'entity_id': FIELD}]},
                       {'tick': 3, 'events': [], 'ui': [{'type': 'commit', 'entity_id': FIELD, 'text': '中文'}]}]}


def expected_logs(start=1, end=12):
    return [(n, f"{n}|{'Run' if n == 1 else ('Health ' if n <= 7 else 'Santé ') + str(21-n)}|"
             f"{'ffi سلام 中文' if n == 1 else 'Pickup ' + str(n-1)}|{str(n in (6,7)).lower()}")
            for n in range(start, end + 1)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--author', action='store_true')
    parser.add_argument('--export-existing', action='store_true')
    parser.add_argument('--existing-proof', type=Path, help='Reverify its frozen package in a new output directory; no build')
    args = parser.parse_args()
    require(sum((args.author, args.export_existing, args.existing_proof is not None)) <= 1, 'Choose one phase')
    output = args.output.resolve()
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else output
    if args.existing_proof:
        output.mkdir(parents=True, exist_ok=False)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    commands = output / 'commands'
    commands.mkdir(parents=True, exist_ok=True)
    receipts = []

    def execute(name, argv, *, success=True, **kwargs):
        argv = list(map(str, argv))
        base = commands / name
        require(not base.with_suffix('.json').exists(), 'Command receipt already exists')
        result = native_process.run(argv, **kwargs)
        base.with_suffix('.stdout').write_text(result.stdout, encoding='utf-8')
        base.with_suffix('.stderr').write_text(result.stderr, encoding='utf-8')
        receipt = {'argv': argv, 'cwd': str(kwargs.get('cwd', ROOT)), 'returncode': result.returncode,
                   'stdout_sha256': digest(base.with_suffix('.stdout')), 'stderr_sha256': digest(base.with_suffix('.stderr'))}
        write(base.with_suffix('.json'), receipt)
        receipts.append(str(base.relative_to(output)) + '.json')
        if success:
            require(result.returncode == 0, f'{name}: {result.stderr[-8000:]}')
        return result

    inputs = proof / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'
    if args.author or not (args.export_existing or args.existing_proof):
        require(args.binary is not None, 'Authoring requires --binary')
        binary = args.binary.resolve(strict=True)
        inputs.mkdir(exist_ok=False)
        execute('init', [binary, 'init', project, '--entities', '0'], timeout=60)
        execute('mixed-fixture', [sys.executable, ROOT / 'tools/probes/native-ui-export.py', output,
                                  '--binary', binary, '--author'], timeout=300)
        authored = json.loads((output / 'authoring.json').read_text(encoding='utf-8'))
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        calls = []
        def call(rpc, method, params=None):
            result = rpc.call(method, params)
            calls.append({'method': method, 'params': params, 'result': result})
            return result
        rpc = Rpc(binary, project, journal)
        try:
            revision = call(rpc, 'project.read')['revision']
            previous = call(rpc, 'script.read', {'script_id': SCRIPT})['untrusted_source_data']['version']
            result = call(rpc, 'command.execute', {'expected_revision': revision,
                'description': 'Author runtime HUD controls', 'commands': [
                    {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': previous, 'source': source()},
                    {'op': 'upsert_string_table', 'table': {'id': TABLE, 'name': 'HUD', 'source_locale': 'en',
                     'messages': {'health': {'en': 'Health {n}', 'fr': 'Santé {n}'}}}},
                    {'op': 'set_locale', 'settings': {'locale': 'en'}}]})
            require(result['revision'] == revision + 1, 'Nonatomic HUD authoring')
            accepted = call(rpc, 'project.read')
            call(rpc, 'history.undo')
            call(rpc, 'history.redo')
            require(call(rpc, 'project.read')['project'] == accepted['project'], 'HUD Undo/Redo mismatch')
            require(call(rpc, 'project.save')['saved'], 'HUD project save failed')
            accepted = call(rpc, 'project.read')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call(rpc, 'project.read') == accepted, 'HUD journal reopen mismatch')
            require(call(rpc, 'script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == source(), 'HUD source mismatch')
        finally:
            rpc.close()
        write(output / 'authoring-rpc.json', {'argv': [str(binary), 'rpc', str(project), '--journal', str(journal)],
            'two_processes_returncode': [0, 0], 'calls': calls})
        require(digest(inputs / 'move.ts') == hashlib.sha256(source().encode()).hexdigest(), 'Final source mismatch')
        authored.update(project_sha256=digest(project), source_sha256=digest(inputs / 'move.ts'),
                        revision=accepted['revision'], headless_sha256=digest(binary), hud_undo_redo_reopen=True)
        write(output / 'authoring.json', authored)
        if args.author:
            return

    authored = json.loads((proof / 'authoring.json').read_text(encoding='utf-8'))
    package = proof / 'package'
    if not args.existing_proof:
        execute('export', [sys.executable, ROOT / 'tools/export_native.py', project, package,
                          '--camera', CAMERA, '--ui-viewport', '320x200'], timeout=7200)
        exported = json.loads((package / 'export.json').read_text(encoding='utf-8'))
        summary = json.loads((commands / 'export.stdout').read_text(encoding='utf-8').splitlines()[-1])
        require(summary['player_sha256'] == exported['player_sha256'], 'Export command/receipt mismatch')
        require(exported['project_sha256'] == authored['project_sha256'] and exported['build_storage_removed_before_startup'], 'Export boundary mismatch')
        write(output / 'deleted-authoring.json', {str(p.relative_to(inputs)): digest(p) for p in sorted(inputs.rglob('*')) if p.is_file()})
        shutil.rmtree(inputs)  # Only this proof's newly authored private project/cache.
        remove_maps = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(output / 'deleted-maps.json', remove_maps(package))
    require(not inputs.exists() and not list(package.rglob('*.map')), 'Source deletion boundary incomplete')
    player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    exported = json.loads((package / 'export.json').read_text(encoding='utf-8'))
    require(exported['project_sha256'] == authored['project_sha256'] and
            exported['build_storage_removed_before_startup'] and
            exported['player_sha256'] == digest(player), 'Frozen public package identity differs')
    data = player.parent / 'data'
    package_hashes = {str(p.relative_to(package)): digest(p) for p in sorted(package.rglob('*')) if p.is_file()}
    # No HOME override. The copied executable has no compiler/tool PATH or project.
    env = {'PATH': str(player.parent), 'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('HOME', 'SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    record = output / 'input.json'
    write(record, recording(authored['tick_rate']))
    plan = output / 'observe.json'
    write(plan, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY], 'checks': [
        {'name': 'mixed-native-motion', 'tick': 12, 'expect': {'type': 'vector', 'entity': ENTITY,
         'property': 'translation', 'value': [72 / authored['tick_rate'], 0, 0], 'tolerance': 1e-9}}]})
    prefix = [player, '--package', data, '--ui-viewport', '320x200']

    def launch(name, extra, ticks=12, start=1):
        pcm, save = output / f'{name}.pcm', output / f'{name}.save'
        result = execute(name, prefix + ['--ticks', str(ticks), '--output', pcm, '--save-checkpoint', save] + extra,
                         cwd=player.parent, env=env, timeout=90)
        value = json.loads(result.stdout)
        require([(e['tick'], e['message']) for e in value['logs']['entries']] == expected_logs(start, start + ticks - 1), 'Pre-command readback logs differ')
        render, n = value['render_input'], start + ticks - 1
        ui = render['ui']
        require(not render['gpu_submitted'] and render['tick'] == n and ui['widgets'] == 3, 'Wrong composed host')
        require(render['text_sources'] == {ENTITY: 'Pickup ' + str(n)}, 'World shaped text not updated')
        require(ui['sources'] == {BUTTON: ('Health ' if n <= 6 else 'Santé ') + str(20-n),
                                 FIELD: 'Done' if n == 12 else 'Name中文'}, 'UI shaped text not updated')
        require(ui['values'][FIELD] == ('Done' if n == 12 else 'Name中文') and ui['focus'] == FIELD, 'Committed edit/focus mismatch')
        require(render['model_count'] == 1 and render['text_count'] == 1 and render['text_missing_clusters'] == 0, 'Mixed consumer missing')
        samples = struct.unpack('<' + 'f' * (pcm.stat().st_size // 4), pcm.read_bytes())
        require(any(s != 0 for s in samples), 'Mixed PCM silent')
        write(output / f'{name}.json', value)
        return value, pcm, save

    runs = [launch(f'run-{i}', ['--input-recording', record, '--observe', plan]) for i in range(3)]
    require(all(r[0]['observations']['passed'] for r in runs), 'Native motion observation failed')
    require(len({digest(r[1]) for r in runs}) == len({digest(r[2]) for r in runs}) == 1, 'Fresh PCM/checkpoint differs')
    short = output / 'input-first.json'
    write(short, recording(authored['tick_rate'], 6))
    first = launch('first', ['--input-recording', short], ticks=6)
    second = launch('second', ['--load-checkpoint', first[2], '--observe', plan], ticks=6, start=7)
    require(first[1].read_bytes() + second[1].read_bytes() == runs[0][1].read_bytes(), 'Resumed PCM differs')
    require(second[2].read_bytes() == runs[0][2].read_bytes(), 'Resumed committed HUD/session differs')
    require(second[0]['observations']['passed'], 'Resumed observation failed')

    refusals = []
    def refuse(name, extra, diagnostic):
        pcm, save = output / f'{name}.pcm', output / f'{name}.save'
        result = execute(name, prefix + ['--ticks', '12', '--output', pcm, '--save-checkpoint', save] + extra,
                         success=False, cwd=player.parent, env=env, timeout=90)
        require(result.returncode == 1 and diagnostic == result.stderr.strip() and not result.stdout,
                f'{name} wrong refusal: {result.returncode} {result.stderr} {result.stdout[:200]}')
        require(not pcm.exists() and not save.exists(), f'{name} published outputs')
        refusals.append({'name': name, 'diagnostic': diagnostic})
    for name, key, diagnostic in [
            ('bad-role', 'F1', 'Error: Session(Script(Structural(Scene(Invalid("HUD text requires a text widget")))))'),
            ('trap', 'F2', 'Error: Session(Script(Javascript { phase: "tick", message: "Error: HUD deliberate trap" }))'),
            ('quota', 'F3', 'Error: Session(Script(Structural(Scene(Invalid("HUD command text or count limit")))))')]:
        path = output / f'{name}-input.json'
        write(path, {'format': 'incant-input', 'version': 2, 'tick_rate': authored['tick_rate'],
                     'start_tick': 0, 'ticks': 12, 'frames': [{'tick': 1, 'events': [{'type': 'key', 'code': key, 'down': True}]}]})
        refuse(name, ['--input-recording', path], diagnostic)
    require(package_hashes == {str(p.relative_to(package)): digest(p) for p in sorted(package.rglob('*')) if p.is_file()}, 'Player mutated package')
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'commands': receipts,
        'original_public_proof': str(proof), 'export_receipt_sha256': digest(package / 'export.json'),
        'package_files': package_hashes, 'player_bytes': player.stat().st_size,
        'player_sha256': digest(player), 'fresh_runs': 3, 'continuation_processes': 2,
        'continuation_pcm_and_final_save_equal': True, 'exact_refusals': refusals,
        'authoring_maps_build_deleted': True, 'limitations': ['CPU canonical shaped text; no GPU/window/appearance',
        'Wasm HUD commands/snapshots not implemented', 'No device or performance gate',
        'PCM and save publication are separate files, not a multifile transaction']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
