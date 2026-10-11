#!/usr/bin/env python3
"""Build trusted Wasm into a native shipping executable and run after source removal.

Desktop Clang/GCC slice. No runtime compiler, native cache loader or JIT, no
MSVC support, no arbitrary-source build service and no phone/AOT gate claim.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from shipping import cargo_output, codegen, archive
from shipping_symbols import backend, package, require, run
import wasm2c

ASSETS = {
    ('Darwin', 'arm64'): ('macos-arm64', '3f654779b436c628db3ca4323c51538815f5fbc887a6d5f3bb1c18d36281e240'),
    ('Linux', 'x86_64'): ('linux-x64', '84895407a6bbb80e918f33b16b2fb2206021c150b6bc9ff6f761263a745ab131'),
    ('Linux', 'aarch64'): ('linux-arm64', 'a7632c1cc66f70886441842ecf5a2ffa91a98c79e026059458451234ad013ea8'),
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def provision(path):
    key = (platform.system(), platform.machine())
    require(key in ASSETS, f'Unsupported pinned WABT host: {key}')
    suffix, expected = ASSETS[key]
    archive_path = path / 'wabt.tar.gz'
    url = f'https://github.com/WebAssembly/wabt/releases/download/1.0.42/wabt-1.0.42-{suffix}.tar.gz'
    with urllib.request.urlopen(url, timeout=30) as response, archive_path.open('wb') as output:
        total = 0
        while block := response.read(128*1024):
            total += len(block); require(total <= 32*1024*1024, 'WABT download size limit')
            output.write(block)
    require(sha(archive_path) == expected, 'Pinned WABT archive checksum mismatch')
    # Extract only two exact regular executable members; never archive paths/links.
    root = path / 'wabt'; (root / 'bin').mkdir(parents=True)
    with tarfile.open(archive_path) as bundle:
        for name in ('wasm2c', 'wat2wasm'):
            entry = bundle.getmember(f'wabt-1.0.42/bin/{name}')
            require(entry.isfile() and 0 < entry.size <= 16*1024*1024, 'Invalid tool archive member')
            dest = root / 'bin' / name
            with bundle.extractfile(entry) as source, dest.open('wb') as output:
                shutil.copyfileobj(source, output)
            dest.chmod(0o755)
    return root, {'release': '1.0.42', 'url': url, 'archive_sha256': expected}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--rustup', default='rustup')
    parser.add_argument('--conformance-only', action='store_true', help='Run combined actual static/JIT tests and development guest probe on one dev graph')
    args = parser.parse_args(); out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run([args.rustup, 'which', '--toolchain', '1.99.0', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target); require(kind != 'pdb', 'Static module MSVC conformance is not established')
    split = 'packed' if kind == 'dsym' else 'off'
    llvm = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CFLAGS', 'HOST_CFLAGS'):
        require(not os.environ.get(key), f'Unset {key} for verified compiler flags')
    environment = dict(os.environ, PATH=str(rustc.parent)+os.pathsep+os.environ.get('PATH', ''))
    cargo = ROOT / 'tools/cargo'
    run([cargo, 'build', '-j2', '-p', 'incant_wasm_cook', '--locked'])
    cook = ROOT / 'target/debug/incant_wasm_cook'
    metadata = json.loads(run([cargo, 'metadata', '--locked', '--format-version=1', '--features', 'incant_wasm_dev/wasmtime-host,incant_wasm_runtime/embedded-fixture']))
    names = {p['id']: p['name'] for p in metadata['packages']}; nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    roots = ['incant_wasm_runtime', 'incant_runtime', 'incant_script', 'incant_game_runtime', 'incant_script_player', 'incant_game_player']
    for root in roots:
        pending = [i for i, n in names.items() if n == root]; require(len(pending) == 1, f'Missing graph root {root}')
        seen = set()
        while pending:
            ident = pending.pop()
            if ident in seen: continue
            seen.add(ident)
            require(not names[ident].startswith(('wasmtime', 'cranelift', 'wasmparser', 'wasm-encoder', 'incant_wasm_cook')), f'Compiler leaked into {root}: {names[ident]}')
            pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] is None for k in d['dep_kinds']))
    with tempfile.TemporaryDirectory(prefix='static-wasm-inputs-', dir=out) as inputs:
        inputs = Path(inputs); wabt, provenance = provision(inputs)
        rust = inputs / 'trusted.rs'; shutil.copy2(ROOT / 'crates/incant_wasm_dev/fixtures/bulk.rs', rust)
        module = inputs / 'bulk.wasm'
        run([rustc, rust, '--crate-type=cdylib', '--edition=2024', '--target=wasm32-unknown-unknown', '-C', 'panic=abort', '-C', 'opt-level=2', '-o', module])
        modules = [('bulk', module)]
        for source in sorted((ROOT / 'crates/incant_wasm_runtime/fixtures').glob('*.wat')):
            copied = inputs / source.name; shutil.copy2(source, copied); dest = copied.with_suffix('.wasm')
            run([wabt / 'bin/wat2wasm', copied, '-o', dest]); modules.append((source.stem, dest))
        bundle = inputs / 'bundle'; manifest = wasm2c.prepare(bundle, modules, wabt, cook)
        (out / 'prepared-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
        environment.update(INCANT_WASM_BUNDLE=str(bundle), CC_ENABLE_DEBUG_OUTPUT='1')
        if args.conformance_only:
            commands = [
                [cargo, 'test', '-j2', '-p', 'incant_wasm_runtime', '-p', 'incant_wasm_cook', '-p', 'incant_wasm_dev', '--features', 'incant_wasm_dev/static-conformance', '--locked'],
                [cargo, 'clippy', '-j2', '-p', 'incant_wasm_runtime', '-p', 'incant_wasm_cook', '-p', 'incant_wasm_dev', '--features', 'incant_wasm_dev/static-conformance', '--all-targets', '--locked', '--', '-D', 'warnings'],
                [sys.executable, ROOT/'tools/probes/development-wasm.py', out/'development', '--rustup', args.rustup, '--static-conformance'],
            ]
            for index, command in enumerate(commands):
                result = subprocess.run([str(x) for x in command], cwd=ROOT, env=environment, capture_output=True, text=True, encoding='utf-8')
                (out/f'conformance-{index}.log').write_text(result.stdout+result.stderr, encoding='utf-8')
                require(result.returncode == 0, f'Conformance command {index} failed; see its log')
            print('Static/development conformance and actual Rust guest probe passed. No shipping/device gate claimed.')
            return

        with tempfile.TemporaryDirectory(prefix='static-wasm-build-', dir=out) as storage:
            build = cargo_output([str(cargo), 'build', '-j2', '-p', 'incant_wasm_runtime', '--features', 'embedded-fixture', '--example', 'shipping_module', '--profile', 'shipping', '--target', target, '--target-dir', storage, '--locked', '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv'], cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed: {out / "build.log"}')
            flags = {}; lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {target}' in line]
            for name in ('incant_wasm_runtime', 'incant_runtime', 'incant_physics_runtime', 'shipping_module'):
                selected = [line for line in lines if re.search(r'--crate-name '+name+r'\b', line)]
                require(len(selected) == 1, f'Missing fresh compile {name}')
                values = codegen(selected[0]); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort'}
                if name == 'shipping_module': expected.update(lto='fat', debuginfo='2', **{'split-debuginfo': split})
                require(all(values.get(k) == v for k, v in expected.items()), f'Codegen mismatch: {values}')
                require('--test' not in selected[0] and values.get('target-cpu') != 'native', 'Test/nonportable executable')
                flags[name] = {k: values[k] for k in expected}
            require(not any(re.search(r'--crate-name (?:wasmtime\w*|cranelift\w*|wasmparser|wasm_encoder|incant_doc|incant_cmd|incant_agent)\b', line) for line in lines), 'Forbidden target dependency compiled')
            for flag in ('-O3', '-frounding-math', '-fno-optimize-sibling-calls', '-ffp-contract=off', '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64'):
                require(flag in build.stderr or flag in build.stdout, f'C compile flag absent: {flag}')
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            executable = next(Path(a['executable']) for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'shipping_module' and a.get('executable'))
            symbols = run([llvm/'llvm-nm', '--defined-only', '--demangle', executable])
            require('wasm2c_m0_' in symbols, 'Translated native code missing')
            require(not any(n in symbols for n in ('wasmtime::', 'cranelift_codegen::', 'wasmparser::')), 'Runtime compiler symbols linked')
            player, identity = package(executable, out, kind, llvm)
        guest_source_hash = sha(rust); guest_wasm_hash = sha(module)
    require(not inputs.exists() and not Path(storage).exists(), 'Temporary source/build input remains')
    scene = out / 'player/fixture.scene'; run([player, '--make-scene', scene])
    require(scene.read_bytes()[8:12] == (1).to_bytes(4,'little'), 'Expected existing cooked v1 fixture')
    outputs = [run([player, 'bulk', scene, '8']) for _ in range(3)]
    require(outputs == ['1 0.25\n2 0.125\n3 0.25\n4 0.125\n']*3, 'Static native motion differs from expected/Wasm development result')
    failures = []
    for name in ('trap', 'nan', 'quaternion', 'scale', 'cycle', 'oob', 'import-oob', 'epoch', 'loop', 'stack', 'indirect-loop', 'start-import', 'start-loop', 'bad-version', 'calls', 'commands', 'not-linked'):
        result = subprocess.run([player, name, scene, '1'], capture_output=True, text=True, encoding='utf-8', timeout=30)
        require(result.returncode != 0 and not result.stdout, f'{name}: failed run published output')
        failures.append({'name': name, 'stderr': result.stderr.strip()})
    result = subprocess.run([player, '--wasm-file', 'untrusted.wasm'], capture_output=True)
    require(result.returncode != 0 and not result.stdout, 'Runtime source loader accepted')
    shutil.copy2(ROOT/'THIRD_PARTY_NOTICES.md', out/'player/THIRD_PARTY_NOTICES.md')
    (out/'player/licenses').mkdir()
    shutil.copy2(ROOT/'licenses/Apache-2.0.txt', out/'player/licenses/Apache-2.0.txt')
    archive(out/'player', out/'player.zip'); archive(out/'symbols', out/'symbols.zip')
    report = {'passed': True, 'compiler': compiler, 'target': target, 'wabt': provenance, 'codegen': flags,
              'guest_source_sha256': guest_source_hash, 'guest_wasm_sha256': guest_wasm_hash,
              'player_sha256': sha(player), 'player_bytes': player.stat().st_size, 'symbols': identity,
              'temporary_guest_sources_wasm_generated_c_tools_and_build_storage_deleted': True,
              'normal_graphs_without_compilers': roots, 'success_processes': 3, 'outputs': outputs, 'rejections': failures,
              'limitations': ['macOS local execution only until other hosts run', 'Windows/MSVC refused', 'repository and validator build storage retained', 'trusted fixture build only, not an arbitrary-source sandbox', 'fixed Wasm memory/table; growth rejected', 'no complete SDK/determinism/mobile/performance gate']}
    (out/'result.json').write_text(json.dumps(report, indent=2)+'\n'); print(json.dumps(report, indent=2))


if __name__ == '__main__': main()
