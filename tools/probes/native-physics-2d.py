#!/usr/bin/env python3
"""Fresh shipping profile execution of retained native 2D tile collision.

A trusted typed fixture producer encodes a checked cooked v7 scene. Its temporary
copy and all fresh shipping build storage are deleted before the copied player
executes. No authoring project/texture is required or claimed to be imported;
public authoring/cook contracts have separate Rust tests. No visual/device gate.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from shipping import cargo_output, codegen
from shipping_symbols import backend, package, require, run

spec = importlib.util.spec_from_file_location('native_parity', ROOT / 'tools/probes/native-physics/run.py')
parity = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parity)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--fixture-tool', type=Path, default=ROOT / f'tools/probes/native-physics/target/release/incant-native-physics-parity{suffix}')
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    require(args.fixture_tool.is_file(), 'Build the existing native parity probe first')
    rustc = Path(run(['rustup', 'which', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target); split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm_bin = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER'):
        require(not os.environ.get(key), f'Unset {key} for declared shipping codegen')
    environment = dict(os.environ, PATH=str(rustc.parent) + os.pathsep + os.environ.get('PATH', ''))
    cargo = [str(ROOT / 'tools/cargo')] if os.name != 'nt' else [str(rustc.with_name('cargo.exe'))]
    with tempfile.TemporaryDirectory(prefix='planar-fixture-', dir=out) as inputs:
        inputs = Path(inputs)
        producer = inputs / ('fixture' + suffix); shutil.copy2(args.fixture_tool, producer)
        scene = inputs / 'planar.scene'; reference = inputs / 'reference.bin'
        run([producer, '--fixture-2d', scene]); run([producer, '1', reference, '--2d'])
        require(scene.read_bytes()[8:12] == (7).to_bytes(4, 'little'), 'Expected cooked v7')
        baseline = reference.read_bytes(); coverage = parity.inspect(baseline, '2d')
        with tempfile.TemporaryDirectory(prefix='planar-shipping-build-', dir=out) as storage:
            command = [*cargo, 'build', '-j2', '-p', 'incant_runtime', '--example', 'native_physics_2d',
                       '--profile', 'shipping', '--locked', '--target', target, '--target-dir', storage,
                       '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed: {out / "build.log"}')
            flags = {}
            for crate in ('incant_runtime', 'incant_physics_2d_runtime', 'rapier2d', 'native_physics_2d'):
                lines = [line for line in build.stderr.splitlines() if 'Running `' in line
                         and re.search(r'--crate-name ' + crate + r'\b', line) and f'--target {target}' in line]
                require(len(lines) == 1, f'Missing fresh target codegen for {crate}')
                values = codegen(lines[0]); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
                if crate == 'native_physics_2d': expected['lto'] = 'fat'
                if target.endswith('-windows-msvc'): expected['target-feature'] = '+crt-static'
                require(all(values.get(k) == v for k, v in expected.items()), f'Codegen mismatch: {values}')
                require(values.get('target-cpu') != 'native' and '--test' not in lines[0], 'Nonportable/test executable')
                flags[crate] = {k: values[k] for k in expected}
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            built = {a['target']['name'] for a in artifacts if a.get('reason') == 'compiler-artifact'}
            forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_editor', 'incant_assets', 'incant_core', 'incant_cook', 'incant_play_legacy', 'loro', 'wasmtime', 'incant_script', 'incant_render', 'wgpu'}
            require(not built & forbidden, f'Unexpected authoring/VM/renderer dependency: {built & forbidden}')
            exe = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'native_physics_2d' and a.get('executable')]
            require(len(exe) == 1, 'Missing standalone planar executable')
            player, identity = package(Path(exe[0]), out, kind, llvm_bin)
        destination = out / 'player/planar.scene'; shutil.copy2(scene, destination)
    require(not inputs.exists() and not Path(storage).exists(), 'Fixture/build storage remains')
    package_before = {p.name: sha(p) for p in player.parent.iterdir() if p.is_file()}
    processes = []
    for repeat in range(3):
        output = out / f'shipping-{repeat}.bin'
        result = subprocess.run([player, destination, output], cwd=player.parent, capture_output=True, text=True, encoding='utf-8', timeout=120)
        require(result.returncode == 0 and result.stdout.strip() == f'tick=640 bytes={len(baseline)}', f'Player failed: {result.stderr}')
        parity.compare(baseline, output.read_bytes(), '2d')
        processes.append({'output': output.name, **parity.inspect(output.read_bytes(), '2d')})
    invalid = [b'not a scene', destination.read_bytes()[:-1], bytearray(destination.read_bytes())]
    invalid[2][8:12] = (9).to_bytes(4, 'little')
    for index, data in enumerate(invalid):
        source = out / f'invalid-{index}.scene'; source.write_bytes(data)
        output = out / f'rejected-{index}.bin'
        result = subprocess.run([player, source, output], cwd=player.parent, capture_output=True, text=True, encoding='utf-8', timeout=30)
        require(result.returncode != 0 and not result.stdout and not output.exists(), 'Invalid scene published output')
    preserved = out / 'preserve.bin'; preserved.write_bytes(b'keep')
    result = subprocess.run([player, destination, preserved], cwd=player.parent, capture_output=True, text=True, encoding='utf-8', timeout=120)
    require(result.returncode != 0 and not result.stdout and preserved.read_bytes() == b'keep', 'Existing output overwritten')
    require(package_before == {p.name: sha(p) for p in player.parent.iterdir() if p.is_file()}, 'Runtime changed retained package')
    sources = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / '.cargo/config.toml', Path(__file__), ROOT / 'tools/probes/native-physics/src/planar.rs']
    for crate in ('incant_runtime', 'incant_physics_2d_runtime', 'incant_types', 'incant_2d'):
        sources += list((ROOT / 'crates' / crate / 'src').glob('*.rs')) + [ROOT / 'crates' / crate / 'Cargo.toml']
    sources += [ROOT / 'crates/incant_runtime/examples/native_physics_2d.rs']
    report = {'passed': True, 'target': target, 'compiler': compiler, 'codegen': flags, 'symbol_identity': identity,
              'player_bytes': player.stat().st_size, 'player_sha256': sha(player), 'built_targets': sorted(built),
              'fixture_tool_sha256': sha(args.fixture_tool), 'cooked_scene_sha256': sha(destination),
              'fixture_kind': 'trusted typed checked v7 fixture; authoring/cook API tested separately',
              'temporary_fixture_and_build_storage_deleted': True, 'successful_processes': 3, 'refusal_processes': 4,
              'source_sha256': {p.relative_to(ROOT).as_posix(): sha(p) for p in sources},
              'coverage': coverage, 'results': processes, 'shipping_matches_release_reference': True,
              'runtime_package_retained': ['copied native executable', 'cooked v7 scene; no texture cache'],
              'visual_or_full_game_session': False, 'physics_saves': False, 'reference_phone_gate': False,
              'failure_semantics': 'Simulation failure stops before output creation. Output I/O is not atomic and may leave a partial stream.'}
    (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'passed': True, 'processes': 7, 'bytes': player.stat().st_size, 'player_sha256': sha(player)}))


if __name__ == '__main__': main()
