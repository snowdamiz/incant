#!/usr/bin/env python3
"""Cook a native audio scene, remove authoring/build inputs, run its shipping host.

Actual native fixed ticks, committed hierarchy poses and Kira offline rendering.
No device output, VM/input integration, cursor saves or phone performance claim.
"""
import argparse
import ast
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
    alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    n = 0
    for char in value:
        n = n * 32 + alphabet.index(char)
    return n.to_bytes(16, 'big')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    parser.add_argument('--cook-binary', type=Path, default=ROOT / f'target/release/examples/cook_scene{suffix}')
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    headless, cook = args.headless_binary.resolve(), args.cook_binary.resolve()
    require(headless.is_file() and cook.is_file(), 'Build the public headless/cook tools first')
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target); split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for declared shipping codegen')
    environment = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
    fixtures = []
    with tempfile.TemporaryDirectory(prefix='native-audio-authoring-', dir=out) as inputs:
        inputs = Path(inputs); project = inputs / 'audio.incant.json'; journal = project.with_suffix('.journal.jsonl')
        run([headless, 'init', project, '--name', 'Native scene audio', '--entities', '0'])
        with wave.open(str(inputs / 'tone.wav'), 'wb') as file:
            file.setparams((2, 2, 48000, 24000, 'NONE', 'not compressed'))
            file.writeframes(b''.join(struct.pack('<hh', v, v) for v in
                [round(8000 * math.sin(2 * math.pi * 440 * i / 48000)) for i in range(24000)]))
        imported = json.loads(run([headless, 'import', project, 'tone.wav']))
        document = json.loads(project.read_text()); scene = next(iter(document['scenes'])); clip = next(iter(document['assets']))
        cache = inputs / '.incant/cache/audio'; manifest = next(cache.glob('*.json'))
        fingerprint = json.loads(manifest.read_text())['fingerprint']
        # This package binding is created once from the author's actual imported
        # ID and expected cooked fingerprint; the host never loads a project.
        table = b'NCLIPS01' + struct.pack('<I', 1) + binary_id(clip) + bytes.fromhex(fingerprint)
        transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
        ids = {n: f'{n:026d}' for n in range(1, 6)}
        revision = 1  # public import transaction
        for streaming in (False, True):
            for rate in (0.5, 1.7):
                name = f'{"stream" if streaming else "static"}-{rate}'
                node = {'clip': clip, 'bus': ids[1], 'gain_db': 0, 'pan': 0, 'rate': rate,
                        'start_seconds': 0.1, 'playing': True, 'looping': True, 'streaming': streaming,
                        'spatial': {'min_distance': 1, 'max_distance': 100}}
                components = {
                    1: {'Transform': transform, 'AudioBus': {'parent': None, 'gain_db': -6}},
                    2: {'Transform': dict(transform, translation=[-3, 0, 0]), 'AudioSource': node},
                    3: {'Transform': transform, 'AudioListener': {}},
                    4: {'Transform': transform, 'Velocity': {'linear': [3, 0, 0]}},
                    5: {'Transform': dict(transform, scale=[2, 3, 4])},
                }
                commands = ([{'op': 'delete_entity', 'scene_id': scene, 'entity_id': ids[n]}
                             for n in (2, 3, 1, 4, 5)] if fixtures else [])
                commands += [{'op': 'create_entity', 'scene_id': scene, 'entity': {
                    'id': ids[n], 'name': f'Audio {n}', 'parent': ids[4] if n == 2 else ids[5] if n == 3 else None,
                    'provenance': None, 'components': components[n]}} for n in (1, 4, 5, 2, 3)]
                requests = [{'id': 1, 'method': 'command.execute', 'params': {'expected_revision': revision,
                             'description': 'Native cooked audio fixture', 'commands': commands}}, {'id': 2, 'method': 'project.save'}]
                reply = subprocess.check_output([headless, 'rpc', project, '--journal', journal],
                    input=''.join(json.dumps(r)+'\n' for r in requests), text=True)
                require(all('error' not in json.loads(line) for line in reply.splitlines()), f'Authoring failed: {reply}')
                revision += 1
                destination = inputs / f'{name}.scene'; run([cook, project, scene, destination])
                require(destination.read_bytes()[8:12] == struct.pack('<I', 3), 'Expected audio cooked version3')
                fixtures.append({'name': name, 'path': destination, 'streaming': streaming, 'rate': rate})
        authoring_hash = sha(project)
        with tempfile.TemporaryDirectory(prefix='native-audio-build-', dir=out) as storage:
            command = [*cargo, 'build', '-j2', '-p', 'incant_audio_runtime', '--features', 'scene', '--example', 'native_scene_audio',
                       '--profile', 'shipping', '--locked', '--target', target, '--target-dir', storage,
                       '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed: {out / "build.log"}')
            flags = {}
            for crate in ('incant_assets_runtime', 'incant_audio_runtime', 'incant_runtime', 'native_scene_audio'):
                lines = [line for line in build.stderr.splitlines() if 'Running `' in line
                         and re.search(r'--crate-name '+crate+r'\b', line) and f'--target {target}' in line]
                require(len(lines) == 1, f'Missing fresh target codegen for {crate}')
                values = codegen(lines[0]); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
                if crate == 'native_scene_audio': expected['lto'] = 'fat'
                if target.endswith('-windows-msvc'): expected['target-feature'] = '+crt-static'
                require(all(values.get(k) == v for k, v in expected.items()), f'Codegen mismatch: {values}')
                require(values.get('target-cpu') != 'native' and '--test' not in lines[0], 'Nonportable/test executable')
                flags[crate] = {k: values[k] for k in expected}
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            built = {a['target']['name'] for a in artifacts if a.get('reason') == 'compiler-artifact'}
            forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_editor', 'incant_assets', 'incant_audio', 'incant_core', 'incant_cook', 'incant_play_legacy', 'loro'}
            require(not built & forbidden and not any(n.startswith('symphonia') for n in built), f'Authoring/importer compiled: {built & forbidden}')
            exe = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'native_scene_audio' and a.get('executable')]
            require(len(exe) == 1, 'Missing standalone native audio executable')
            player, identity = package(Path(exe[0]), out, kind, llvm_bin)
        shutil.copytree(cache, out / 'player/audio')
        (out / 'player/clips.bin').write_bytes(table)
        for fixture in fixtures:
            destination = out / 'player' / fixture['path'].name
            shutil.copy2(fixture['path'], destination); fixture['path'] = str(destination)
        require(authoring_hash == sha(project), 'Build changed authoring project')
    require(not inputs.exists() and not Path(storage).exists(), 'Authoring/build input storage remains')
    cache = out / 'player/audio'; table_path = out / 'player/clips.bin'
    before = {p.name: sha(p) for p in cache.iterdir()}
    results = []
    for fixture in fixtures:
        for batch in (97, 8192):
            hashes = []
            for repeat in range(2):
                dest = out / f'{fixture["name"]}-{batch}-{repeat}.pcm'
                report = run([player, fixture['path'], cache, table_path, '120', str(batch), dest]).strip()
                digest = report.split()[0]; require(digest == sha(dest), 'PCM digest mismatch')
                pose = ast.literal_eval(report.split('position=', 1)[1])
                require('tick=120 frames=96000' in report and abs(pose[0]-3) < 1e-10 and pose[1:] == [0, 0],
                        f'Native hierarchy did not advance: {report}')
                pcm = list(struct.iter_unpack('<ff', dest.read_bytes()))
                require(len(pcm) == 96000 and all(math.isfinite(v) for f in pcm for v in f), 'Invalid PCM')
                left = sum(l*l-r*r for l, r in pcm[1000:20000]); right = sum(l*l-r*r for l, r in pcm[-20000:])
                require(left > 1 and right < -1, f'Committed source/listener motion did not move stereo energy: {left}, {right}')
                rms = math.sqrt(sum(v*v for f in pcm for v in f)/(len(pcm)*2)); require(0.01 < rms < 0.3, f'Unexpected RMS {rms}')
                hashes.append(digest)
            require(hashes[0] == hashes[1], 'Fresh process replay changed PCM')
            results.append({'fixture': fixture['name'], 'batch': batch, 'pcm_sha256': hashes[0], 'rms': rms, 'left_early_balance': left, 'right_late_balance': right})
    require(before == {p.name: sha(p) for p in cache.iterdir()}, 'Runtime changed cooked cache')
    # Incorrect package bindings and malformed/oversized inputs fail closed.
    bad_table = out / 'bad-clips.bin'; bad_table.write_bytes(table[:-1] + bytes([table[-1] ^ 1]))
    bad_scene = out / 'bad.scene'; bad_scene.write_bytes(b'not a scene')
    for scene_path, clips_path in ((fixtures[0]['path'], bad_table), (bad_scene, table_path)):
        dest = out / 'rejected.pcm'
        result = subprocess.run([player, scene_path, cache, clips_path, '1', '97', dest], capture_output=True, text=True)
        require(result.returncode != 0 and not result.stdout and not dest.exists(), 'Invalid package published output')
    existing = out / 'preserve.pcm'; existing.write_bytes(b'keep')
    result = subprocess.run([player, fixtures[0]['path'], cache, table_path, '1', '97', existing], capture_output=True, text=True)
    require(result.returncode != 0 and not result.stdout and existing.read_bytes() == b'keep', 'Existing output overwritten')
    report = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags, 'symbol_identity': identity,
              'player_bytes': player.stat().st_size, 'player_sha256': sha(player), 'built_targets': sorted(built),
              'headless_sha256': sha(headless), 'cook_sha256': sha(cook), 'public_import': imported,
              'project_hash_before_delete': authoring_hash, 'authoring_and_build_storage_deleted': True,
              'runtime_package_retained': ['binary scenes', 'clips.bin ID/fingerprint table', 'cooked manifests and PCM'],
              'native_fixed_ticks_per_process': 120, 'runs': 16, 'results': results,
              'repeat_exact': True, 'moving_parameter_cross_batch_bit_equality_claimed': False,
              'audio_device_opened': False, 'vm_input_audio_session_integration': False, 'audio_cursor_saves': False,
              'reference_phone_gate': False, 'failure_semantics': 'Simulation/sync/render failure stops before output creation; no world rollback. Output I/O is not atomic and may leave a partial PCM file.'}
    (out / 'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({'passed': True, 'runs': 16, 'bytes': player.stat().st_size, 'player_sha256': sha(player)}))


if __name__ == '__main__':
    main()
