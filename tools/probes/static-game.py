#!/usr/bin/env python3
"""Run a build-linked Wasm program through native input/physics/audio sessions.

Desktop host==target proof only. The versioned incant_input_v1 snapshot drives
actual guest motion and structural commands. No renderer/device/save gate.
"""
import argparse
import importlib.util
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

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('static_shipping', ROOT/'tools/probes/shipping-wasm2c.py')
static = importlib.util.module_from_spec(spec)
spec.loader.exec_module(static)
require, run, sha = static.require, static.run, static.sha


def graph(cargo, env):
    metadata = static.cargo_metadata([cargo, 'metadata', '--locked', '--format-version=1',
        '--features', 'incant_wasm_player/fixture'], cwd=ROOT, env=env)
    names = {p['id']: p['name'] for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    pending = [i for i, name in names.items() if name == 'incant_wasm_player']
    require(len(pending) == 1, 'Missing unique player graph')
    seen = set()
    forbidden = ('wasmtime', 'cranelift', 'wasmparser', 'wasm-encoder', 'incant_wasm_cook')
    authoring = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook', 'incant_play_legacy', 'loro'}
    while pending:
        ident = pending.pop()
        if ident in seen: continue
        seen.add(ident)
        require(not names[ident].startswith(forbidden) and names[ident] not in authoring,
                f'Forbidden normal player dependency: {names[ident]}')
        pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] is None for k in d['dep_kinds']))
    # QuickJS remains a normal dependency of incant_script in this focused slice.
    require(any(names[i] == 'rquickjs' for i in seen), 'Update the explicit QuickJS graph limitation')
    return sorted(names[i] for i in seen)


def execute(player, args):
    return subprocess.run([str(player), *map(str, args)], capture_output=True, text=True,
                          encoding='utf-8', timeout=30)


def verify_player(player, out):
    successes, failures = [], []
    for streaming in (False, True):
        fixture = out/('stream' if streaming else 'static')
        key = run([player, '--make-fixture', fixture, *(['--stream'] if streaming else [])]).strip()
        scene, recording = fixture/'fixture.scene', fixture/'input.json'
        require(scene.read_bytes()[8:12] == struct.pack('<I', 3), 'Expected real physics/audio v3 scene')
        base = [scene, fixture, key, recording]
        digests = []
        for index in range(3):
            pcm = out/f'{fixture.name}-{index}.pcm'
            result = execute(player, ['game', *base, pcm])
            require(result.returncode == 0, result.stderr)
            require(result.stdout == 'tick=64 frames=44117 source=[2.078125, 1.0, 0.0] spawned=1.96875 pressed=1 released=1\n',
                    f'Wrong combined state/timeline: {result.stdout}')
            data = pcm.read_bytes()
            require(len(data) == 44117*2*4, 'Rational sample clock drift')
            values = struct.unpack('<'+'f'*(len(data)//4), data)
            require(all(math.isfinite(v) for v in values), 'Nonfinite PCM')
            energy = [sum(abs(v) for v in values[channel::2]) for channel in (0, 1)]
            require(energy[1] > energy[0] > 0, f'Spatial movement not reflected in PCM: {energy}')
            digests.append(sha(pcm))
            successes.append({'streaming': streaming, 'stdout': result.stdout.strip(), 'pcm_sha256': digests[-1], 'energy': energy})
        require(len(set(digests)) == 1, 'Repeated-process PCM changed')
        # A later accepted press must delay the guest's structural spawn. Merely
        # counting the native input frame cannot satisfy this executable control.
        delayed = fixture/'delayed-input.json'
        packet = json.loads(recording.read_text())
        packet['frames'][0]['tick'] = 5; packet['frames'][1]['tick'] = 7
        delayed.write_text(json.dumps(packet))
        pcm = out/f'{fixture.name}-delayed.pcm'
        result = execute(player, ['game', scene, fixture, key, delayed, pcm])
        require(result.returncode == 0, result.stderr)
        require(result.stdout == 'tick=64 frames=44117 source=[2.078125, 1.0, 0.0] spawned=1.84375 pressed=1 released=1\n',
                f'Guest ignored delayed accepted action: {result.stdout}')
        require(pcm.stat().st_size == 44117*2*4, 'Delayed input altered sample clock')
        successes.append({'streaming': streaming, 'delayed_action': True,
                          'stdout': result.stdout.strip(), 'pcm_sha256': sha(pcm)})
        cases = [(name, [name, *base], None) for name in ('trap', 'nan', 'cycle', 'loop', 'remove-listener', 'audio-pose', 'not-linked')]
        for name in ('camera', 'manifest'):
            spatial = fixture/f'{name}.scene'
            require(spatial.read_bytes()[8:12] == struct.pack('<I', 4), 'Expected real v4 refusal')
            cases.append((name, ['start-loop', spatial, fixture, key, recording], 'SpatialHost'))
        cases.append(('overflow', ['loop', fixture/'overflow.scene', fixture, key, recording], 'Runtime'))
        invalid_recording = fixture/'bad-input.json'
        invalid_recording.write_text(recording.read_text().replace('"tick_rate":64', '"tick_rate":60'))
        cases.append(('recording-clock', ['game', scene, fixture, key, invalid_recording], None))
        cases.append(('clip-fingerprint', ['game', scene, fixture, '0'*64, recording], None))
        for name, command, expected in cases:
            pcm = out/f'rejected-{fixture.name}-{name}.pcm'
            result = execute(player, [*command, pcm])
            require(result.returncode != 0 and not result.stdout and not pcm.exists(), f'{name}: rejected run published output')
            require(expected is None or expected in result.stderr, f'{name}: wrong failure order: {result.stderr}')
            failures.append({'streaming': streaming, 'name': name, 'stderr': result.stderr.strip()})
    for command in (['--wasm-file', 'untrusted.wasm'], ['--script-file', 'untrusted.js']):
        result = execute(player, command)
        require(result.returncode != 0 and not result.stdout, 'Source loader accepted')
        failures.append({'name': command[0], 'stderr': result.stderr.strip()})
    return successes, failures


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--rustup', default='rustup')
    parser.add_argument('--conformance-only', action='store_true')
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run([args.rustup, 'which', '--toolchain', '1.99.0', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = static.backend(target); suffix = '.exe' if kind == 'pdb' else ''
    require(kind != 'pdb' or target == 'x86_64-pc-windows-msvc', 'Only x64 MSVC configured')
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm = Path(run([rustc, '--print', 'sysroot']).strip())/'lib/rustlib'/target/'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CFLAGS', 'HOST_CFLAGS'):
        require(not os.environ.get(key), f'Unset {key} for declared flags')
    env = dict(os.environ, PATH=str(rustc.parent)+os.pathsep+os.environ.get('PATH', ''))
    if kind == 'pdb': env['INCANT_WASM_MSVC_OPT_LEVEL'] = '2'
    cargo = str(ROOT/'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe'))
    dependencies = graph(cargo, env)
    static.cargo_build([cargo, 'build', '-j2', '-p', 'incant_wasm_cook', '--locked'],
                       cwd=ROOT, env=env, log=out/'cook-build.log')
    with tempfile.TemporaryDirectory(prefix='static-game-inputs-', dir=out) as inputs:
        inputs = Path(inputs); wabt, provenance = static.provision(inputs)
        rust = inputs/'game.rs'; shutil.copy2(ROOT/'crates/incant_wasm_player/fixtures/game.rs', rust)
        module = inputs/'game.wasm'
        run([rustc, rust, '--crate-type=cdylib', '--edition=2024', '--target=wasm32-unknown-unknown', '-C', 'panic=abort', '-C', 'opt-level=2', '-o', module])
        modules = [('game', module)]
        if args.conformance_only:
            flipbook = inputs/'flipbook.rs'; shutil.copy2(ROOT/'crates/incant_game_player/fixtures/flipbook.rs', flipbook)
            flipbook_module = inputs/'flipbook.wasm'
            run([rustc, flipbook, '--crate-type=cdylib', '--edition=2024', '--target=wasm32-unknown-unknown', '-C', 'panic=abort', '-C', 'opt-level=2', '-o', flipbook_module])
            modules.append(('flipbook-game', flipbook_module))
        for source in [*(ROOT/'crates/incant_wasm_runtime/fixtures').glob('*.wat'), *(ROOT/'crates/incant_wasm_player/fixtures').glob('*.wat')]:
            copied = inputs/source.name; shutil.copy2(source, copied); dest = copied.with_suffix('.wasm')
            run([wabt/f'bin/wat2wasm{suffix}', copied, '-o', dest]); modules.append((source.stem, dest))
        bundle = inputs/'bundle'
        manifest = static.wasm2c.prepare(bundle, modules, wabt, ROOT/f'target/debug/incant_wasm_cook{suffix}')
        (out/'prepared-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
        env.update(INCANT_WASM_BUNDLE=str(bundle), CC_ENABLE_DEBUG_OUTPUT='1')
        if args.conformance_only:
            packages = ['-p', 'incant_wasm_player', '-p', 'incant_game_runtime', '-p', 'incant_script']
            features = 'incant_wasm_player/fixture,incant_script/compiler'
            for index, command in enumerate([
                [cargo, 'test', '-j2', *packages, '--features', features, '--locked'],
                [cargo, 'clippy', '-j2', *packages, '--features', features, '--all-targets', '--locked', '--', '-D', 'warnings']]):
                static.cargo_build(command, cwd=ROOT, env=env, log=out/f'conformance-{index}.log')
            print('Actual composed-module session tests and Clippy passed; no shipping/device gate claimed.')
            return
        with tempfile.TemporaryDirectory(prefix='static-game-build-', dir=out) as storage:
            link_map = out/'player.map'
            command = [cargo, 'rustc' if kind == 'pdb' else 'build', '-j2', '-p', 'incant_wasm_player', '--bin', 'incant_wasm_player', '--features', 'fixture', '--profile', 'shipping', '--target', target, '--target-dir', storage, '--locked', '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            if kind == 'pdb': command += ['--', '-C', f'link-arg=/MAP:{link_map}']
            build = static.cargo_output(command, cwd=ROOT, env=env)
            (out/'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Fresh shipping build failed; see {out/"build.log"}')
            lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {target}' in line]
            forbidden = r'wasmtime\w*|cranelift\w*|wasmparser|wasm_encoder|incant_doc|incant_cmd|incant_agent|incant_cook|incant_wasm_cook|loro'
            require(not any(re.search(r'--crate-name (?:'+forbidden+r')\b', line) for line in lines), 'Forbidden runtime compile')
            flags = {}
            for name in ('incant_wasm_runtime', 'incant_script', 'incant_game_runtime', 'incant_runtime', 'incant_physics_runtime', 'incant_input', 'incant_audio_runtime', 'incant_assets_runtime', 'incant_wasm_player'):
                selected = [line for line in lines if re.search(r'--crate-name '+name+r'\b', line)]
                # The proof package contains both its fixture-data lib and binary.
                require(len(selected) == (2 if name == 'incant_wasm_player' else 1), f'Missing compile: {name}')
                for line in selected:
                    values = static.codegen(line); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort'}
                    if kind == 'pdb': expected['target-feature'] = '+crt-static'
                    if name == 'incant_wasm_player' and '--crate-type bin' in line: expected.update(lto='fat', debuginfo='2', **{'split-debuginfo': split})
                    require(all(values.get(k) == v for k, v in expected.items()), f'Wrong flags: {values}')
                    require('feature="compiler"' not in line and '--test' not in line and values.get('target-cpu') != 'native', 'Compiler/test/host-native leakage')
                flags[name] = expected
            require(sum('--crate-name incant_wasm_player ' in line and '--crate-type bin' in line for line in lines) == 1, 'Missing unique final binary command')
            c_flags = static.verify_c_flags(build.stderr+build.stdout, kind, '2')
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            executable = next(Path(a['executable']) for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'incant_wasm_player' and a.get('executable'))
            symbols = link_map.read_text(encoding='utf-8') if kind == 'pdb' else run([llvm/f'llvm-nm{suffix}', '--defined-only', '--demangle', executable])
            static.verify_module_symbols(symbols)
            require(not any(name in symbols for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program', 'js_parse_function_decl', 'incant_script::compiler')), 'Runtime source compiler linked')
            player, identity = static.package(executable, out, kind, llvm)
        source_hash, wasm_hash = sha(rust), sha(module)
    require(not inputs.exists() and not Path(storage).exists(), 'Temporary compile input remains')
    successes, failures = verify_player(player, out)
    shutil.copy2(ROOT/'THIRD_PARTY_NOTICES.md', out/'player/THIRD_PARTY_NOTICES.md')
    (out/'player/licenses').mkdir(); shutil.copy2(ROOT/'licenses/Apache-2.0.txt', out/'player/licenses/Apache-2.0.txt')
    static.archive(out/'player', out/'player.zip'); static.archive(out/'symbols', out/'symbols.zip')
    report = {'passed': True, 'target': target, 'compiler': compiler, 'wabt': provenance, 'codegen': flags,
        'c_flags': c_flags, 'normal_dependencies': dependencies, 'player_bytes': player.stat().st_size,
        'player_sha256': sha(player), 'symbols': identity, 'source_sha256': source_hash, 'wasm_sha256': wasm_hash,
        'temporary_compile_sources_tools_generated_c_wasm_and_build_storage_deleted': True,
        'successes': successes, 'rejections': failures,
        'limitations': ['versioned copied input snapshot, not a complete gameplay SDK', 'QuickJS remains in normal dependency graph; this is not a QuickJS-free architecture', 'retained PCM and recording JSON are runtime exchange files, not deleted compile inputs', 'repository and validator build storage retained', 'copied bounded numeric transfer, not zero-copy', 'no renderer, full session save, mobile/device or performance gate', 'simulation/module/audio failures publish no PCM; output I/O itself is not atomic', 'trusted build environment; C flag audit checks global presence, not every translation unit']}
    (out/'result.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__': main()
