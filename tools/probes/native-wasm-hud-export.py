#!/usr/bin/env python3
"""Public mixed HUD game, generated Rust SDK and ordinary static Wasm export.

Checks actual shaped text, routed notifications, gameplay movement and PCM after
source/cache/build/debug-map deletion. CPU only; guest checkpoint still refuses.
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
WORLD = '00000000000000000000000002'
CAMERA = '00000000000000000000000003'
UI_ROOT = '00000000000000000000000010'
BUTTON = '00000000000000000000000011'
FIELD = '00000000000000000000000012'


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def recording(rate):
    return {'format': 'incant-input', 'version': 2, 'tick_rate': rate, 'start_tick': 0, 'ticks': 12,
            'frames': [{'tick': 1, 'events': [{'type': 'key', 'code': 'Tab', 'down': True}, {'type': 'key', 'code': 'Space', 'down': True}]},
                       {'tick': 2, 'events': [{'type': 'key', 'code': 'Tab', 'down': False}, {'type': 'key', 'code': 'Space', 'down': False}], 'ui': [{'type': 'focus', 'entity_id': FIELD}]},
                       {'tick': 3, 'events': [], 'ui': [{'type': 'commit', 'entity_id': FIELD, 'text': '中文'}]},
                       {'tick': 6, 'events': [{'type': 'key', 'code': 'Enter', 'down': True}]},
                       {'tick': 7, 'events': [{'type': 'key', 'code': 'Enter', 'down': False}]}]}


def expected_logs():
    result = []
    for n in range(1, 13):
        button = 'Run' if n == 1 else 'Edited 中文' if n == 4 else 'Submitted' if n == 7 else 'Health 8'
        focus = 33 if n == 1 else 34
        message = (f'{n}|focus={focus}|text={str(n > 1).lower()}|discard=0|w32:false:-'
                   f'|w33:{str(n == 5).lower()}:{button}|w34:false:Name'
                   f'|t2:{"ffi سلام 中文" if n == 1 else "Pickup 中文"}')
        message += {1: '|e33:0:-', 3: '|e34:1:Name中文', 6: '|e34:2:Name中文'}.get(n, '')
        result.append((n, message))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--wabt', type=Path, required=True)
    parser.add_argument('--stage', choices=('author', 'export', 'all'), default='all')
    parser.add_argument('--existing-proof', type=Path, help='Verify an already exported/deleted package into a new output; no rebuild')
    args = parser.parse_args()
    out, binary, wabt = args.output.resolve(), args.binary.resolve(strict=True), args.wabt.resolve(strict=True)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    if args.stage != 'export' or args.existing_proof:
        out.mkdir(parents=True, exist_ok=False)
    sys.path.insert(0, str(ROOT / 'tools'))
    import native_process
    commands = out / 'commands'
    commands.mkdir(exist_ok=True)
    def run(name, argv, *, success=True, **kwargs):
        base = commands / name
        require(not base.with_suffix('.json').exists(), 'Command receipt exists')
        argv = list(map(str, argv))
        result = native_process.run(argv, **kwargs)
        base.with_suffix('.stdout').write_text(result.stdout, encoding='utf-8')
        base.with_suffix('.stderr').write_text(result.stderr, encoding='utf-8')
        write(base.with_suffix('.json'), {'argv': argv, 'cwd': str(kwargs.get('cwd', ROOT)), 'returncode': result.returncode,
              'stdout_sha256': digest(base.with_suffix('.stdout')), 'stderr_sha256': digest(base.with_suffix('.stderr'))})
        require(not success or result.returncode == 0, f'{name}: {result.stderr[-8000:]}')
        return result
    inputs = proof / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'
    if args.stage != 'export' and not args.existing_proof:
        run('fixture', [sys.executable, ROOT / 'tools/probes/native-hud-export.py', out, '--binary', binary, '--author'], timeout=300)
        authored = json.loads((out / 'authoring.json').read_text())
        run('guest', [sys.executable, ROOT / 'tools/rust_guest.py', ROOT / 'sdk/rust/samples/hud.rs', out / 'guest-build/hud.wasm', '--rustup', '/opt/homebrew/bin/rustup'], timeout=300)
        shutil.copy2(out / 'guest-build/hud.wasm', inputs / 'hud.wasm')
        # Public import on an exact disposable document copy has its own journal.
        # Register its validated asset in the live fixture's original bus below.
        import_project = inputs / 'module-import.json'
        shutil.copy2(project, import_project)
        module = json.loads(run('import-wasm', [binary, 'import', import_project, 'hud.wasm'], timeout=120).stdout)
        Rpc = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        calls = []
        def call(rpc, method, params=None):
            value = rpc.call(method, params)
            calls.append({'method': method, 'params': params, 'result': value})
            return value
        rpc = Rpc(binary, project, journal)
        try:
            before = call(rpc, 'project.read')
            call(rpc, 'command.execute', {'expected_revision': before['revision'], 'description': 'Select native Rust HUD behavior', 'commands': [
                {'op': 'upsert_asset', 'asset': module['asset']},
                {'op': 'remove_component', 'scene_id': authored['scene'], 'entity_id': WORLD, 'component': 'Script'},
                {'op': 'remove_script', 'script_id': '00000000000000000000000001'},
                {'op': 'set_scene_program', 'scene_id': authored['scene'], 'program': {'kind': 'wasm', 'module': module['asset']['id']}}]})
            selected = call(rpc, 'project.read')['project']
            call(rpc, 'history.undo'); require(call(rpc, 'project.read')['project'] == before['project'], 'Undo mismatch')
            call(rpc, 'history.redo'); require(call(rpc, 'project.read')['project'] == selected, 'Redo mismatch')
            require(call(rpc, 'project.save')['saved'], 'Save failed')
            accepted = call(rpc, 'project.read')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(call(rpc, 'project.read') == accepted, 'Reopen mismatch')
        finally:
            rpc.close()
        write(out / 'module-rpc.json', {'argv': [str(binary), 'rpc', str(project), '--journal', str(journal)], 'exit_codes': [0, 0], 'calls': calls})
        authored.update(project_sha256=digest(project), wasm_sha256=digest(inputs / 'hud.wasm'), guest_source_sha256=digest(ROOT / 'sdk/rust/samples/hud.rs'), module=module['asset']['id'])
        removed = {str(p.relative_to(out)): digest(p) for p in sorted((out / 'guest-build').rglob('*')) if p.is_file()}
        shutil.rmtree(out / 'guest-build')
        for name in ('move.ts', 'hud.wasm'):
            p = inputs / name
            if p.exists(): removed[str(p.relative_to(out))] = digest(p); p.unlink()
        write(out / 'deleted-guest.json', removed)
        write(out / 'module-authoring.json', authored)
        if args.stage == 'author': return
    authored = json.loads((proof / 'module-authoring.json').read_text())
    package = proof / 'package'
    if not args.existing_proof:
        require(authored['project_sha256'] == digest(project), 'Project changed')
        run('export', [sys.executable, ROOT / 'tools/export_native.py', project, package, '--camera', CAMERA, '--ui-viewport', '320x200', '--wabt', wabt], timeout=7200)
        write(out / 'deleted-authoring.json', {str(p.relative_to(inputs)): digest(p) for p in sorted(inputs.rglob('*')) if p.is_file()})
        shutil.rmtree(inputs)
        # Wasm-only exports normally have no source maps. Remove only this
        # proof's own optional symbol maps, never files outside its package.
        maps = sorted((package / 'symbols').rglob('*.map'))
        removed = []
        for path in maps:
            require(path.is_file() and not path.is_symlink() and path.resolve().is_relative_to(package.resolve()), 'Unexpected symbol map')
            removed.append({'path':str(path.relative_to(package)), 'sha256':digest(path)})
        for path in maps: path.unlink()
        write(out / 'deleted-maps.json', removed)
    exported = json.loads((package / 'export.json').read_text())
    require(exported['program']['kind'] == 'wasm' and exported['build_storage_removed_before_startup'], 'Wrong program/build boundary')
    require(not inputs.exists() and not list(package.rglob('*.map')), 'Authoring/source maps remain')
    player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    require(digest(player) == exported['player_sha256'] and exported['project_sha256'] == authored['project_sha256'], 'Export/player/project binding differs')
    data = player.parent / 'data'
    package_files = {str(p.relative_to(package)): digest(p) for p in sorted(package.rglob('*')) if p.is_file()}
    env = {k: os.environ[k] for k in ('HOME', 'SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ}
    env['PATH'] = str(player.parent)
    record = out / 'input.json'; write(record, recording(authored['tick_rate']))
    observe = out / 'observe.json'
    write(observe, {'format': 'incant-native-observation', 'version': 1, 'entities': [WORLD], 'checks': [{'name': 'movement', 'tick': 12,
        'expect': {'type': 'vector', 'entity': WORLD, 'property': 'translation', 'value': [72/authored['tick_rate'], 0, 0], 'tolerance': 1e-9}}]})
    prefix = [player, '--package', data, '--ui-viewport', '320x200', '--ticks', '12']
    runs = []
    for i in range(3):
        pcm = out / f'run-{i}.pcm'
        result = json.loads(run(f'run-{i}', prefix + ['--input-recording', record, '--observe', observe, '--output', pcm], cwd=player.parent, env=env, timeout=90).stdout)
        require(result['observations']['passed'], 'Movement failed')
        require([(e['tick'], e['message']) for e in result['logs']['entries']] == expected_logs() and result['logs']['dropped'] == 0, 'Exact declarations/routed outcomes differ')
        r = result['render_input']; u = r['ui']
        require(not r['gpu_submitted'] and r['tick'] == 12 and r['model_count'] == 1 and r['text_count'] == 1, 'Wrong composed host')
        require(r['text_sources'] == {WORLD: 'Pickup 中文'} and u['sources'] == {BUTTON: 'Health 8', FIELD: 'Name中文'}, 'Actual shaped text did not change')
        require(r['text_missing_clusters'] == 0 and r['text_font_count'] == 2 and r['text_glyph_count'] >= 8 and r['text_atlas_nonzero'] > 0 and u['glyphs'] >= 12, 'Packaged Latin/CJK fallback shaping missing')
        require(u['values'][FIELD] == 'Name中文' and u['focus'] == FIELD, 'Edit/focus differs')
        require(any(x != 0 for (x,) in struct.iter_unpack('<f', pcm.read_bytes())), 'Expected mixed audio')
        write(out / f'run-{i}.json', result); runs.append(digest(pcm))
    require(len(set(runs)) == 1, 'Fresh PCM differs')
    refusals = []
    # Late native validation and ignored SDK errors are different typed paths.
    expected = {'F1': 'Error: Session(Module(Boundary(Runtime(Scene(Invalid("HUD text requires a text widget"))))))',
                'F2': 'Error: Session(Module(Trap(1)))', 'F3': 'Error: Session(Module(Boundary(Runtime(Scene(Invalid("HUD command text or count limit"))))))',
                'F4': 'Error: Session(Module(Trap(1)))',
                'F5': 'Error: Session(Module(Boundary(Runtime(MissingEntity(StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 33]))))))'}
    for key, diagnostic in expected.items():
        rec = out / f'{key}.json'; write(rec, {'format': 'incant-input', 'version': 2, 'tick_rate': authored['tick_rate'], 'start_tick': 0, 'ticks': 12,
            'frames': [{'tick': 1, 'events': [{'type': 'key', 'code': key, 'down': True}]}]})
        pcm = out / f'bad-{key}.pcm'
        p = run(f'bad-{key}', prefix + ['--input-recording', rec, '--output', pcm], success=False, cwd=player.parent, env=env, timeout=90)
        require(p.returncode == 1 and p.stderr.strip() == diagnostic and not p.stdout and not pcm.exists(), f'Wrong {key} refusal: {p.stderr}')
        refusals.append({'key': key, 'diagnostic': diagnostic})
    pcm, save = out / 'checkpoint.pcm', out / 'module.save'
    p = run('checkpoint-refused', prefix + ['--input-recording', record, '--output', pcm, '--save-checkpoint', save], success=False, cwd=player.parent, env=env, timeout=90)
    diagnostic = 'Error: Checkpoint(Unsupported("static module state ABI"))'
    require(p.returncode == 1 and p.stderr.strip() == diagnostic and not p.stdout and not pcm.exists() and not save.exists(), f'Wrong checkpoint refusal: {p.stderr}')
    refusals.append({'kind':'module-checkpoint', 'diagnostic':diagnostic})
    require(package_files == {str(p.relative_to(package)): digest(p) for p in sorted(package.rglob('*')) if p.is_file()}, 'Package changed')
    write(out / 'result.json', {'passed': True, 'proof_root': str(proof), 'authoring': authored, 'player_sha256': digest(player), 'player_bytes': player.stat().st_size,
        'package_files': package_files, 'positive_processes': 3, 'exact_refusals': refusals, 'source_cache_build_maps_deleted': True,
        'limitations': ['No module checkpoints', 'CPU canonical shaping only, no GPU/window/appearance or performance gate']})


if __name__ == '__main__':
    main()
