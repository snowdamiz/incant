#!/usr/bin/env python3
"""Run build-produced bytecode, replay, native physics and audio after source removal.

Desktop host==target shipping proof; optional CPU-only retained render input.
No GPU submission, device output, session save,
portable export, phone measurement or performance gate is claimed.
"""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from shipping import cargo_output, codegen
from shipping_symbols import backend, package, require, run


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def binary_id(value):
    n = 0
    for char in value:
        n = n * 32 + '0123456789ABCDEFGHJKMNPQRSTVWXYZ'.index(char)
    return n.to_bytes(16, 'big')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--render-input', action='store_true', help='Enable the existing player CPU render-input feature and real cooked model/camera fixture')
    suffix = '.exe' if os.name == 'nt' else ''
    for name, default in (
        ('headless', f'incant_headless{suffix}'), ('cook', f'examples/cook_scene{suffix}'),
        ('audio', f'examples/native_scene_audio{suffix}'), ('fixture', f'examples/no_renderer_fixtures{suffix}')):
        parser.add_argument(f'--{name}-binary', type=Path, default=ROOT / 'target/release' / default)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    headless, cook, audio, fixture_tool = (getattr(args, f'{n}_binary').resolve() for n in ('headless', 'cook', 'audio', 'fixture'))
    require(all(p.is_file() for p in (headless, cook, audio, fixture_tool)), 'Build public headless/cook/audio/fixture tools first')
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target)
    require(kind != 'pdb', 'Runtime compiler-symbol audit supports Mach-O/ELF; PDB audit is unimplemented')
    split = 'packed' if kind == 'dsym' else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for declared shipping codegen')
    environment = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
    fixtures = []
    sentinel = 'INCANT_GAME_SOURCE_COMMENT_8e461d8f'
    with tempfile.TemporaryDirectory(prefix='native-game-authoring-', dir=out) as inputs:
        inputs = Path(inputs); project = inputs / 'game.incant.json'; journal = project.with_suffix('.journal.jsonl')
        run([headless, 'init', project, '--name', 'Native game composition', '--entities', '0'])
        with wave.open(str(inputs / 'tone.wav'), 'wb') as file:
            file.setparams((2, 2, 48000, 24000, 'NONE', 'not compressed'))
            file.writeframes(b''.join(struct.pack('<hh', v, v) for v in
                [round(8000 * math.sin(2 * math.pi * 440 * i / 48000)) for i in range(24000)]))
        imported = json.loads(run([headless, 'import', project, 'tone.wav']))
        document = json.loads(project.read_text()); scene = next(iter(document['scenes'])); clip = next(iter(document['assets']))
        require(document['settings']['tick_rate'] == 60, 'Expected public default tick rate')
        cache = inputs / '.incant/cache/audio'; manifest = next(cache.glob('*.json'))
        fingerprint = json.loads(manifest.read_text())['fingerprint']
        table = b'NCLIPS01' + struct.pack('<I', 1) + binary_id(clip) + bytes.fromhex(fingerprint)
        transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
        ids = {n: f'{n:026d}' for n in (1, 2, 3, 4)}
        entity_order = (3, 1, 2, 4) if args.render_input else (3, 1, 2)
        revision = 1
        model_import = None
        if args.render_input:
            (inputs / 'tri.bin').write_bytes(struct.pack('<9f', 0, 0, 0, 1, 0, 0, 0, 1, 0))
            (inputs / 'tri.gltf').write_text(json.dumps({'asset': {'version': '2.0'},
                'buffers': [{'byteLength': 36, 'uri': 'tri.bin'}], 'bufferViews': [{'buffer': 0, 'byteLength': 36}],
                'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 3, 'type': 'VEC3', 'min': [0,0,0], 'max': [1,1,0]}],
                'meshes': [{'primitives': [{'attributes': {'POSITION': 0}}]}], 'nodes': [{'mesh': 0}], 'scenes': [{'nodes': [0]}], 'scene': 0}))
            model_import = json.loads(run([headless, 'import', project, 'tri.gltf']))
            updated = json.loads(project.read_text())
            model = next(ident for ident in updated['assets'] if ident not in document['assets'])
            revision += 1
        for streaming in (False, True):
            name = 'stream' if streaming else 'static'
            components = {
                1: {'Transform': dict(transform, translation=[-3, 0, 0]),
                    'Velocity': {'linear': [0, 0, 0]},
                    'RigidBody': {'motion': 'kinematic', 'gravity_scale': 0, 'linear_damping': 0, 'angular_damping': 0, 'can_sleep': True, 'ccd': False},
                    'Collider': {'shape': {'type': 'box', 'half_extents': [0.5, 0.5, 0.5]}, 'density': 1000, 'friction': 0.5,
                                 'restitution': 0, 'sensor': False, 'memberships': 4294967295, 'filter': 4294967295},
                    'AudioSource': {'clip': clip, 'bus': ids[3], 'gain_db': 0, 'pan': 0, 'rate': 1,
                        'start_seconds': 0, 'playing': True, 'looping': True, 'streaming': streaming,
                        'spatial': {'min_distance': 1, 'max_distance': 100}}},
                2: {'Transform': transform, 'AudioListener': {}},
                3: {'Transform': transform, 'AudioBus': {'parent': None, 'gain_db': -6}},
            }
            if args.render_input:
                components[1]['MeshRenderer'] = {'mesh': model, 'materials': [], 'cast_shadows': True}
                components[4] = {'Transform': transform, 'Camera': {'fov_degrees': 60, 'near': 0.1, 'far': 100, 'projection': {'kind': 'perspective'}}}
            commands = ([{'op': 'delete_entity', 'scene_id': scene, 'entity_id': ids[n]} for n in entity_order] if fixtures else [])
            commands += [{'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': ids[n], 'name': f'Game {n}', 'parent': None, 'provenance': None, 'components': components[n]}} for n in entity_order]
            requests = [{'id': 1, 'method': 'command.execute', 'params': {'expected_revision': revision,
                         'description': 'Native game fixture', 'commands': commands}}, {'id': 2, 'method': 'project.save'}]
            reply = subprocess.check_output([headless, 'rpc', project, '--journal', journal],
                input=''.join(json.dumps(r)+'\n' for r in requests), text=True, encoding='utf-8')
            require(all('error' not in json.loads(line) for line in reply.splitlines()), f'Authoring failed: {reply}')
            revision += 1
            destination = inputs / f'{name}.scene'; run([cook, project, scene, destination])
            require(destination.read_bytes()[8:12] == struct.pack('<I', 4 if args.render_input else 3), 'Unexpected cooked scene version')
            fixtures.append({'name': name, 'path': destination})
        authoring_hash = sha(project)
        source = inputs / 'game.js'
        checks = '''
for (const compile of [() => eval('1'), () => (0,eval)('1'),
    () => Function('return 1'), () => (function(){}).constructor('return 1'),
    () => (async function(){}).constructor('return 1'), () => (function*(){}).constructor('yield 1')]) {
    let denied = false; try { compile(); } catch (_) { denied = true; }
    if (!denied) throw Error('shipping source compiler active');
}
'''
        behavior = (ROOT / 'crates/incant_game_player/fixtures/game.js').read_text()
        if args.render_input:
            behavior += "\nconst update = exports.default.update; exports.default.update = function(ctx) { update(ctx); if(ctx.input.actions.camera_fail.pressed) ctx.commands.despawn('00000000000000000000000004'); };\n"
        source.write_text(f'// {sentinel}\n' + checks + behavior)
        environment.update(INCANT_NATIVE_SCRIPT_SOURCE=str(source), INCANT_NATIVE_SCRIPT_NAME='embedded-game.js')
        environment.pop('INCANT_NATIVE_SCRIPT_MAP', None)
        with tempfile.TemporaryDirectory(prefix='native-game-build-', dir=out) as storage:
            command = [*cargo, 'build', '-j2', '-p', 'incant_game_player', '--profile', 'shipping', '--locked',
                       '--target', target, '--target-dir', storage, '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            if args.render_input: command += ['--features', 'render-input']
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed: {out / "build.log"}')
            flags = {}
            target_lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {target}' in line]
            forbidden = ('incant_doc', 'incant_cmd', 'incant_agent', 'incant_editor', 'incant_assets', 'incant_audio', 'incant_core', 'incant_cook', 'incant_play_legacy', 'loro')
            require(not any(re.search(r'--crate-name ('+'|'.join(forbidden)+r')\b', line) for line in target_lines), 'Authoring target dependency compiled')
            extra = ('incant_render_game', 'incant_render_native', 'incant_render_data') if args.render_input else ()
            for crate in extra + ('incant_input', 'incant_script', 'incant_runtime', 'incant_physics_runtime', 'rapier3d', 'incant_assets_runtime', 'incant_audio_runtime', 'incant_game_runtime', 'incant_game_player'):
                lines = [line for line in target_lines if re.search(r'--crate-name '+crate+r'\b', line)]
                require(len(lines) == 1, f'Missing fresh target compile for {crate}')
                require('feature="compiler"' not in lines[0], f'Compiler feature leaked into {crate}')
                values = codegen(lines[0]); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort'}
                if crate == 'incant_game_player': expected.update(lto='fat', debuginfo='2', **{'split-debuginfo': split})
                require(all(values.get(k) == v for k, v in expected.items()), f'Codegen mismatch: {values}')
                require(values.get('target-cpu') != 'native' and '--test' not in lines[0], 'Nonportable/test executable')
                flags[crate] = {k: values[k] for k in expected}
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            exe = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'incant_game_player' and a.get('executable')]
            require(len(exe) == 1, 'Missing game player executable')
            symbols = run([llvm_bin / f'llvm-nm{suffix}', '--defined-only', '--demangle', exe[0]])
            require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Bytecode runtime missing')
            for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program', 'js_parse_function_decl', 'incant_script::compiler'):
                require(name not in symbols, f'Source compiler linked: {name}')
            require(sentinel.encode() not in Path(exe[0]).read_bytes(), 'Source comment embedded')
            player, identity = package(Path(exe[0]), out, kind, llvm_bin)
            metadata_files = list(Path(storage).glob('*/shipping/build/incant_game_player-*/out/script-symbols.json'))
            require(len(metadata_files) == 1, 'Missing script symbols')
            metadata = json.loads(metadata_files[0].read_text())
            shutil.copy2(metadata_files[0], out / 'symbols/script-symbols.json')
        shutil.copytree(cache, out / 'player/audio')
        if args.render_input: shutil.copytree(inputs / '.incant/cache/models', out / 'player/models')
        (out / 'player/clips.bin').write_bytes(table)
        for fixture in fixtures:
            dest = out / 'player' / fixture['path'].name
            shutil.copy2(fixture['path'], dest); fixture['path'] = str(dest)
        require(authoring_hash == sha(project), 'Build changed authoring project')
    require(not inputs.exists() and not Path(storage).exists(), 'Authoring/build storage remains')
    # Runtime exchange files are intentionally retained and parsed once, not on ticks.
    replay = {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0, 'ticks': 120,
              'frames': [{'tick': 1, 'events': [{'type': 'key', 'code': 'KeyD', 'down': True}]}]}
    actions = {n: {'kind': 'button', 'bindings': [{'type': 'key', 'code': c}]} for n, c in (('right', 'KeyD'), ('fail', 'KeyF'))}
    if args.render_input: actions['camera_fail'] = {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'KeyC'}]}
    record_path = out / 'player/input.json'; record_path.write_text(json.dumps(replay))
    action_path = out / 'player/actions.json'; action_path.write_text(json.dumps(actions))
    cache = out / 'player/audio'; table_path = out / 'player/clips.bin'
    before = {p.name: sha(p) for p in (out / 'player').rglob('*') if p.is_file()}
    results = []
    def command(scene_path, recording, rate, dest, clips=table_path, spatial=None):
        cmd = [player, scene_path, cache, clips, recording, action_path, '120', str(rate), dest]
        if (args.render_input if spatial is None else spatial):
            cmd += ['--cpu-render-input', out/'player/models', out/'player/textures', binary_id(ids[4]).hex()]
        return cmd
    for fixture in fixtures:
        for rate in (44117, 48000):
            reports = []
            for repeat in range(2):
                dest = out / f'{fixture["name"]}-{rate}-{repeat}.pcm'
                report = json.loads(run(command(fixture['path'], record_path, rate, dest)))
                require(report['tick'] == 120 and report['frames'] == 120*rate//60 and report['pcm_sha256'] == sha(dest), 'Tick/sample/hash mismatch')
                require(len(report['sources']) == 1, 'Unexpected source count')
                source = report['sources'][0]; expected = -3 + 119*3/60
                require(abs(source['position'][0]-expected) < 0.001 and source['position'][1:] == [0, 0], 'Script/physics motion or ordering incorrect')
                require(source['physics']['position'] == source['position'] and source['physics']['velocity'] == [3, 0, 0], 'Solver state not synchronized')
                if args.render_input:
                    spatial = report['render_input']
                    require(spatial['mode'] == 'cpu-only' and not spatial['gpu_submitted'] and spatial['tick'] == 120, 'Wrong spatial publication scope')
                    require(spatial['model_count'] == 1 and spatial['decoded_payload_bytes'] > 0 and spatial['camera'] == binary_id(ids[4]).hex(), 'Missing real cooked binding')
                    require(spatial['first_model_position'] == source['position'] and spatial['camera_position'] == [0,0,0], 'CPU render input disagrees with committed simulation')
                pcm = list(struct.iter_unpack('<ff', dest.read_bytes()))
                require(len(pcm) == report['frames'] and all(math.isfinite(v) for frame in pcm for v in frame), 'Invalid PCM')
                left = sum(l*l-r*r for l, r in pcm[1000:20000]); right = sum(l*l-r*r for l, r in pcm[-20000:])
                rms = math.sqrt(sum(v*v for frame in pcm for v in frame)/(2*len(pcm)))
                require(left > 1 and right < -1 and 0.01 < rms < 0.3, f'Motion did not pan actual PCM: {left}, {right}, {rms}')
                reports.append(report)
            require(reports[0] == reports[1], 'Fresh-process replay changed output')
            results.append({'fixture': fixture['name'], 'rate': rate, 'report': reports[0], 'rms': rms, 'early_balance': left, 'late_balance': right})
    require(before == {p.name: sha(p) for p in (out / 'player').rglob('*') if p.is_file()}, 'Runtime changed retained package')
    failures = []
    def reject(label, cmd, dest, message=None):
        result = subprocess.run(cmd, capture_output=True, text=True, encoding='utf-8')
        require(result.returncode != 0 and not result.stdout and not dest.exists(), f'{label} published output')
        if message: require(message in result.stderr, f'{label} rejected at wrong boundary: {result.stderr}')
        failures.append(label)
    for label, changed in (
        ('accepted-script-failure', dict(replay, frames=[{'tick': 2, 'events': [{'type': 'key', 'code': 'KeyF', 'down': True}]}])),
        ('wrong-recording-clock', dict(replay, tick_rate=61)),
        ('invalid-input-packet', dict(replay, frames=[{'tick': 1, 'events': [{'type': 'gamepad_axis', 'id': 99, 'axis': 'left_x', 'value': 0.5}]}]))):
        path = out / f'{label}.json'; path.write_text(json.dumps(changed)); dest = out / f'{label}.pcm'
        reject(label, command(fixtures[0]['path'], path, 48000, dest), dest)
    bad_table = out / 'wrong-fingerprint.bin'; bad_table.write_bytes(table[:-1]+bytes([table[-1]^1])); dest = out / 'wrong-fingerprint.pcm'
    reject('wrong-clip-fingerprint', command(fixtures[0]['path'], record_path, 48000, dest, bad_table), dest)
    bad_scene = out / 'unknown-version.scene'; malformed = bytearray(Path(fixtures[0]['path']).read_bytes())
    malformed[8:12] = struct.pack('<I', 99); bad_scene.write_bytes(malformed); dest = out / 'unknown-version.pcm'
    reject('unknown-cooked-version', command(bad_scene, record_path, 48000, dest), dest)
    oversized = out / 'oversized-actions.json'; oversized.write_bytes(b' ' * (256*1024+1)); dest = out / 'oversized-actions.pcm'
    bad_command = command(fixtures[0]['path'], record_path, 48000, dest); bad_command[5] = oversized
    reject('oversized-startup-actions', bad_command, dest, 'runtime input size limit')
    dest = out / 'manual-input.pcm'
    reject('manual-input-route', command(fixtures[0]['path'], record_path, 48000, dest)+['--input', 'KeyD'], dest, 'manual input/source paths are unsupported')
    refusal_dir = out / 'refusal-scenes'; run([fixture_tool, refusal_dir])
    refusal_results = []
    for name in ('camera-only', 'manifest-only'):
        path = refusal_dir / f'{name}.scene'; require(path.read_bytes()[8:12] == struct.pack('<I', 4), 'Expected typed v4 fixture')
        for exe, cmd in ((player, command(path, record_path, 48000, out / f'game-{name}.pcm', spatial=False)),
                         (audio, [audio, path, cache, table_path, '1', '97', out / f'audio-{name}.pcm'])):
            dest = Path(cmd[-1]); reject(f'{exe.name}-{name}', cmd, dest, 'no spatial host'); refusal_results.append({'player': exe.name, 'fixture': name})
    # Ordinary v1 controls prove that refusal is capability-specific.
    for exe, cmd in ((player, command(refusal_dir/'plain.scene', record_path, 48000, out/'game-plain.pcm', spatial=False)),
                     (audio, [audio, refusal_dir/'plain.scene', cache, table_path, '1', '97', out/'audio-plain.pcm'])):
        run(cmd); require(Path(cmd[-1]).read_bytes() == b'\0'*Path(cmd[-1]).stat().st_size, 'Plain scene should render silence')
    if args.render_input:
        late = out/'late-camera-removal.json'
        late.write_text(json.dumps(dict(replay, frames=[{'tick': 2, 'events': [{'type': 'key', 'code': 'KeyC', 'down': True}]}])))
        dest = out/'late-camera-removal.pcm'
        reject('late-camera-removal', command(fixtures[0]['path'], late, 48000, dest), dest, 'MissingCamera')
        dest = out/'default-spatial.pcm'
        reject('feature-default-stays-strict', command(fixtures[0]['path'], record_path, 48000, dest, spatial=False), dest, 'no spatial host')
        for label, index, value, message in (
            ('missing-model-cache', 10, out/'missing-models', 'Asset'),
            ('missing-camera', 12, '00000000000000000000000000000009', 'MissingCamera'),
            ('malformed-camera', 12, 'not-hexadecimal', 'CAMERA_HEX')):
            dest = out/f'{label}.pcm'; cmd = command(fixtures[0]['path'], record_path, 48000, dest)
            cmd[index] = value; reject(label, cmd, dest, message)
    existing = out/'preserved.pcm'; existing.write_bytes(b'keep')
    result = subprocess.run(command(fixtures[0]['path'], record_path, 48000, existing), capture_output=True)
    require(result.returncode != 0 and not result.stdout and existing.read_bytes() == b'keep', 'Existing output overwritten')
    report = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags, 'symbol_identity': identity,
              'player_bytes': player.stat().st_size, 'player_sha256': sha(player), 'script_metadata': metadata,
              'source_compiler_symbols_absent': True, 'dynamic_source_routes_rejected': True,
              'headless_sha256': sha(headless), 'cook_sha256': sha(cook), 'audio_example_sha256': sha(audio), 'fixture_tool_sha256': sha(fixture_tool),
              'public_import': imported, 'public_model_import': model_import, 'cpu_render_input': args.render_input, 'gpu_submitted': False, 'project_hash_before_delete': authoring_hash,
              'authoring_and_build_storage_deleted': True, 'removed_scope': 'authoring project/WAV/glTF/bin/journal/script source and fresh Cargo storage',
              'retained_runtime_inputs': ['binary scenes', 'clips.bin', 'cooked PCM/manifests', 'startup input.json/actions.json'] + (['cooked model payloads/manifests'] if args.render_input else []),
              'repeat_exact': True, 'recorded_ticks_per_process': 120, 'audio_processes': 8, 'results': results, 'rejected': failures,
              'spatial_refusals': refusal_results, 'plain_v1_controls': 2, 'existing_output_preserved': True,
              'size_delta_from_audio_host_2885984': player.stat().st_size-2885984,
              'size_delta_from_physics_script_player_3941816': player.stat().st_size-3941816,
              'size_comparison_scope': 'whole executable graphs, not an isolated feature cost or performance gate',
              'device_output': False, 'session_saves': False, 'phone_performance_gate': False,
              'failure_semantics': 'Invalid input/buffer retry is covered in host tests. Any failure after accepted simulation destroys composite state; device cursor is not rolled back. Simulation/host failure creates no output file; output I/O may leave a partial file.'}
    (out/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'passed': True, 'bytes': player.stat().st_size, 'sha256': sha(player), 'evidence': str(out/'result.json')}))


if __name__ == '__main__':
    main()
