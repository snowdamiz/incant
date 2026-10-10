#!/usr/bin/env python3
"""Build and execute a cooked-only offline audio player with shipping codegen.

Authoring uses the public command-backed importer. Then both the source project
and fresh build directory are deleted before the packaged runtime is executed.
This proves a desktop cooked-audio boundary, not native ECS audio integration,
device latency, browser streaming or reference-phone performance.
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--headless-binary', type=Path, default=ROOT / f'target/release/incant_headless{suffix}')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    headless = args.headless_binary.resolve()
    require(headless.is_file(), 'Build the command-backed headless importer first')
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV'])
    target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target)
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for the declared shipping build')
    with tempfile.TemporaryDirectory(prefix='audio-inputs-', dir=out) as inputs:
        inputs = Path(inputs)
        project = inputs / 'audio.incant.json'
        run([headless, 'init', project, '--name', 'Cooked audio probe', '--entities', '0'])
        with wave.open(str(inputs / 'tone.wav'), 'wb') as file:
            file.setparams((2, 2, 48000, 24000, 'NONE', 'not compressed'))
            samples = [round(8000 * math.sin(2 * math.pi * 440 * i / 48000)) for i in range(24000)]
            file.writeframes(b''.join(struct.pack('<hh', v, -v) for v in samples))
        shutil.copy2(ROOT / 'crates/incant_assets/tests/fixtures/tone-440hz.ogg', inputs / 'music.ogg')
        imported = {name: json.loads(run([headless, 'import', project, name])) for name in ('tone.wav', 'music.ogg')}
        cache = inputs / '.incant/cache/audio'
        manifests = {p.name: json.loads(p.read_text()) for p in cache.glob('*.json')}
        require(len(manifests) == 2, 'Expected two public importer audio manifests')
        keys = {m['dependency']['path']: m['fingerprint'] for m in manifests.values()}
        require(set(keys) == set(imported), 'Unexpected cooked source identities')
        authoring_hash = sha(project)
        (ROOT / 'target').mkdir(exist_ok=True)
        environment = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
        cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
        with tempfile.TemporaryDirectory(prefix='audio-shipping-', dir=ROOT / 'target') as storage:
            command = [*cargo, 'build', '-j2', '-p', 'incant_audio_runtime', '--example', 'cooked_audio',
                       '--profile', 'shipping', '--locked', '--target', target, '--target-dir', storage,
                       '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed; see {out / "build.log"}')
            flags = {}
            for crate in ('incant_assets_runtime', 'incant_audio_runtime', 'cooked_audio'):
                lines = [line for line in build.stderr.splitlines() if 'Running `' in line
                         and re.search(r'--crate-name ' + crate + r'\b', line) and f'--target {target}' in line]
                require(len(lines) == 1, f'Missing fresh target invocation for {crate}')
                values = codegen(lines[0])
                expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
                if crate == 'cooked_audio':
                    expected['lto'] = 'fat'
                if target.endswith('-windows-msvc'):
                    expected['target-feature'] = '+crt-static'
                require(all(values.get(k) == v for k, v in expected.items()), f'Incorrect codegen: {values}')
                require(values.get('target-cpu') != 'native' and '--test' not in lines[0], 'Nonportable CPU/test executable')
                flags[crate] = {k: values[k] for k in expected}
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            built = {a['target']['name'] for a in artifacts if a.get('reason') == 'compiler-artifact'}
            forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_editor', 'incant_assets',
                         'incant_audio', 'incant_import', 'incant_cook', 'incant_play_legacy', 'loro', 'gltf', 'image'}
            require(not built & forbidden and not any(n.startswith('symphonia') for n in built),
                    f'Authoring/importer code compiled for runtime: {built & forbidden}')
            executables = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
                           and a['target']['name'] == 'cooked_audio' and a.get('executable')]
            require(len(executables) == 1, 'Missing standalone cooked audio executable')
            player, identity = package(Path(executables[0]), out, kind, llvm_bin)
        shutil.copytree(cache, out / 'player/audio')
        require(authoring_hash == sha(project), 'Runtime build changed project content')
    require(not inputs.exists() and not Path(storage).exists(), 'Source/build storage still present')
    cache = out / 'player/audio'
    before_cache = {p.name: sha(p) for p in cache.iterdir()}
    results = []
    for name, key in keys.items():
        for mode in ('static', 'stream'):
            for rate in (0.25, 1, 1.7, 4):
                hashes = []
                for batch in (97, 8192):
                    for repeat in range(2):
                        dest = out / f'{name}-{mode}-{rate}-{batch}-{repeat}.pcm'
                        digest = run([player, cache, key, mode, str(rate), '44000', str(batch), dest]).strip()
                        require(digest == sha(dest), 'Mixed PCM report mismatch')
                        pcm = list(struct.iter_unpack('<ff', dest.read_bytes()))
                        require(len(pcm) == 44000 and all(math.isfinite(v) for f in pcm for v in f), 'Invalid PCM')
                        rms = math.sqrt(sum(f[0] ** 2 for f in pcm[1000:]) / 43000)
                        require(0.1 < rms < 0.3, f'Unexpected level: {rms}')
                        sign = -1 if name == 'tone.wav' else 1
                        require(all(abs(f[0] - sign*f[1]) < 1e-5 for f in pcm), 'Stereo signal changed')
                        hashes.append(digest)
                require(len(set(hashes)) == 1, 'Fresh process/batch size changed PCM')
                results.append({'clip': name, 'mode': mode, 'rate': rate, 'pcm_sha256': hashes[0], 'rms': rms})
    require(before_cache == {p.name: sha(p) for p in cache.iterdir()}, 'Runtime mutated cooked cache')
    # Failures do not publish outputs or overwrite existing ones.
    existing = out / 'preserve.pcm'; existing.write_bytes(b'keep')
    checks = [('missing', next(iter(keys.values())), existing),
              ('static', '../escape', out / 'rejected.pcm'),
              ('static', next(iter(keys.values())), existing)]
    for mode, key, dest in checks:
        result = subprocess.run([player, cache, key, mode, '1', '100', '97', dest], capture_output=True, text=True)
        require(result.returncode != 0 and not result.stdout, 'Invalid playback reported success')
    require(existing.read_bytes() == b'keep' and not (out / 'rejected.pcm').exists(), 'Failure modified output')
    report = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags, 'symbol_identity': identity,
              'player_sha256': sha(player), 'player_bytes': player.stat().st_size, 'headless_sha256': sha(headless),
              'built_targets': sorted(built), 'public_command_imports': imported, 'project_hash_before_delete': authoring_hash,
              'source_and_build_storage_deleted': True, 'read_only_cache': before_cache, 'runs': 64,
              'audio_device_opened': False, 'batch_and_process_repeat_exact': True, 'results': results,
              'native_ecs_audio_integration': False, 'reference_phone_gate': False}
    (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'runs': 64, 'bytes': player.stat().st_size, 'player_sha256': sha(player)}))


if __name__ == '__main__':
    main()
