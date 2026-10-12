#!/usr/bin/env python3
"""Build trusted Wasm into a native shipping executable and run after source removal.

Desktop native-compiler slice. No runtime compiler, native cache loader or JIT,
no arbitrary-source build service and no phone/AOT gate claim. MSVC /O2 is an
explicit conformance candidate until its hosted checks establish support.
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
import time
import urllib.error
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import rust_guest
from shipping import cargo_output, codegen, archive
from shipping_symbols import backend, package, require, run
import wasm2c

ASSETS = {
    ('Darwin', 'arm64'): ('macos-arm64', '3f654779b436c628db3ca4323c51538815f5fbc887a6d5f3bb1c18d36281e240'),
    ('Linux', 'x86_64'): ('linux-x64', '84895407a6bbb80e918f33b16b2fb2206021c150b6bc9ff6f761263a745ab131'),
    ('Linux', 'aarch64'): ('linux-arm64', 'a7632c1cc66f70886441842ecf5a2ffa91a98c79e026059458451234ad013ea8'),
    ('Windows', 'AMD64'): ('windows-x64', '62f1eb2b51aa57cf0f0b8ba333bf2c72a049930aa9c5de01dcad271c4fe48c88'),
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cargo_metadata(command, *, cwd, env):
    # Cargo can download target-specific packages while resolving the graph.
    # Its progress/warnings are stderr, never part of the machine JSON document.
    result = cargo_output(command, cwd=cwd, env=env)
    require(result.returncode == 0, f'Cargo metadata failed: {result.stderr[-4000:]}')
    return json.loads(result.stdout)


def cargo_build(command, *, cwd, env, log):
    # A direct cargo.exe still launches rustc by name. Preserve the explicitly
    # pinned toolchain PATH for those children, including registry build scripts.
    result = cargo_output(command, cwd=cwd, env=env)
    log.write_text(result.stdout + result.stderr, encoding='utf-8')
    require(result.returncode == 0, f'Cargo build failed; see {log}: {result.stderr[-4000:]}')
    return result


def transient_download_error(error):
    if isinstance(error, urllib.error.HTTPError):
        return error.code in (408, 429, 500, 502, 503, 504)
    if isinstance(error, urllib.error.URLError):
        error = error.reason
    # Certificate errors, permanent HTTP responses, disk errors, integrity and
    # archive validation failures are deliberately outside this narrow set.
    return isinstance(error, (ConnectionError, TimeoutError))


def provision(path):
    key = (platform.system(), platform.machine())
    require(key in ASSETS, f'Unsupported pinned WABT host: {key}')
    suffix, expected = ASSETS[key]
    archive_path = path / 'wabt.tar.gz'
    url = f'https://github.com/WebAssembly/wabt/releases/download/1.0.42/wabt-1.0.42-{suffix}.tar.gz'
    for attempt in range(1, 4):
        try:
            # Truncate a partial prior transfer. Never resume unknown bytes or
            # switch URLs, versions, mirrors or expected integrity on retry.
            with urllib.request.urlopen(url, timeout=30) as response, archive_path.open('wb') as output:
                total = 0
                while block := response.read(128*1024):
                    total += len(block); require(total <= 32*1024*1024, 'WABT download size limit')
                    output.write(block)
            break
        except Exception as error:
            if attempt == 3 or not transient_download_error(error):
                raise
            time.sleep(0.5 * attempt)
    require(sha(archive_path) == expected, 'Pinned WABT archive checksum mismatch')
    # Extract only two exact regular executable members; never archive paths/links.
    root = path / 'wabt'; (root / 'bin').mkdir(parents=True)
    with tarfile.open(archive_path) as bundle:
        for name in ('wasm2c', 'wat2wasm'):
            name += '.exe' if key[0] == 'Windows' else ''
            entry = bundle.getmember(f'wabt-1.0.42/bin/{name}')
            require(entry.isfile() and 0 < entry.size <= 16*1024*1024, 'Invalid tool archive member')
            dest = root / 'bin' / name
            with bundle.extractfile(entry) as source, dest.open('wb') as output:
                shutil.copyfileobj(source, output)
            dest.chmod(0o755)
    return root, {'release': '1.0.42', 'url': url, 'archive_sha256': expected, 'download_attempts': attempt}


def verify_c_flags(output, kind, msvc_opt_level):
    flags = ('/Od' if msvc_opt_level == '0' else '/O2', '/TC', '/std:c11',
             '/experimental:c11atomics', '/fp:strict', '-MT',
             '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64') if kind == 'pdb' else (
             '-O3', '-frounding-math', '-fno-optimize-sibling-calls', '-ffp-contract=off',
             '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64')
    for flag in flags:
        present = flag in output or (kind == 'pdb' and flag.startswith('/') and '-'+flag[1:] in output)
        require(present, f'C compile flag absent: {flag}')
    return list(flags)


def verify_module_symbols(symbols):
    require('wasm2c_m0_' in symbols, 'Translated native code missing')
    # This check supplements the complete normal-dependency and fresh rustc
    # command traversal; no claim is made about arbitrarily renamed native code.
    require(not any(n in symbols for n in ('wasmtime', 'cranelift_codegen', 'wasmparser')),
            'Runtime compiler symbols linked')


def localization_guests(rustc, inputs, sdk_guests, environment):
    """Fixed trusted sample/controls for the existing actual-host conformance."""
    sample = inputs/'localization.rs'
    shutil.copy2(ROOT/'sdk/rust/samples/localization.rs', sample)
    failure = inputs/'localization-failure.rs'
    shutil.copy2(ROOT/'sdk/rust/fixtures/localization_failure.rs', failure)
    modules = []
    for kind in (None, 'capacity', 'number', 'duplicate', 'callback'):
        name = 'rust-localization'+('-'+kind if kind else '')
        module = sdk_guests/(name+'.wasm')
        rust_guest.compile(rustc, failure if kind else sample, module,
                           env=environment, cfg=['sdk_locale_'+kind] if kind else ())
        modules.append((name, module))
    return modules


def animation_guests(rustc, inputs, sdk_guests, environment):
    """Compile real no_std animation controls and transactional failure guests."""
    sample = inputs/'animation.rs'
    shutil.copy2(ROOT/'sdk/rust/samples/animation.rs', sample)
    failure = inputs/'animation-failure.rs'
    shutil.copy2(ROOT/'sdk/rust/fixtures/animation_failure.rs', failure)
    modules = []
    for kind in (None, 'capacity', 'number', 'rest', 'clip', 'entity', 'callback'):
        name = 'rust-animation'+('-'+kind if kind else '')
        module = sdk_guests/(name+'.wasm')
        rust_guest.compile(rustc, failure if kind else sample, module,
                           env=environment, cfg=['sdk_animation_'+kind] if kind else ())
        modules.append((name, module))
    return modules


def formatting_guests(rustc, inputs, sdk_guests, environment):
    """Actual no_std primitive ICU formatting and ignored-error controls."""
    sample = inputs/'formatting.rs'
    shutil.copy2(ROOT/'sdk/rust/samples/formatting.rs', sample)
    failure = inputs/'formatting-failure.rs'
    shutil.copy2(ROOT/'sdk/rust/fixtures/formatting_failure.rs', failure)
    modules = []
    for kind in (None, 'capacity', 'number', 'range', 'date', 'year', 'quota', 'callback'):
        name = 'rust-formatting'+('-'+kind if kind else '')
        module = sdk_guests/(name+'.wasm')
        rust_guest.compile(rustc, failure if kind else sample, module,
                           env=environment, cfg=['sdk_format_'+kind] if kind else ())
        modules.append((name, module))
    return modules


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--rustup', default='rustup')
    parser.add_argument('--profile', choices=('dev', 'ci'), default='dev', help='Local conformance profile; shipping builds always use shipping')
    parser.add_argument('--conformance-only', action='store_true', help='Run combined actual static/JIT tests and development guest probe on one dev graph')
    parser.add_argument('--msvc-opt-level', choices=('0', '2'), default='0',
                        help='Windows C /Od baseline or explicit /O2 conformance candidate')
    args = parser.parse_args(); out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    rustc = Path(run([args.rustup, 'which', '--toolchain', '1.99.0', 'rustc']).strip())
    compiler = run([rustc, '-vV']); target = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(target)
    require(kind != 'pdb' or target == 'x86_64-pc-windows-msvc', 'Only x64 MSVC static host is configured')
    suffix = '.exe' if kind == 'pdb' else ''
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm = Path(run([rustc, '--print', 'sysroot']).strip()) / 'lib/rustlib' / target / 'bin'
    for key in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CFLAGS', 'HOST_CFLAGS'):
        require(not os.environ.get(key), f'Unset {key} for verified compiler flags')
    environment = dict(os.environ, PATH=str(rustc.parent)+os.pathsep+os.environ.get('PATH', ''))
    if kind == 'pdb':
        environment['INCANT_WASM_MSVC_OPT_LEVEL'] = args.msvc_opt_level
    cargo = ROOT / 'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
    built = cargo_build([str(cargo), 'build', '-j2', '-p', 'incant_wasm_cook', '--locked', '--profile', args.profile, '--message-format=json'],
                cwd=ROOT, env=environment, log=out/'cook-build.log')
    cook = next(Path(record['executable']) for record in map(json.loads, built.stdout.splitlines())
                if record.get('reason') == 'compiler-artifact' and record.get('executable')
                and record['target']['name'] == 'incant_wasm_cook')
    metadata = cargo_metadata([str(cargo), 'metadata', '--locked', '--format-version=1', '--features', 'incant_wasm_dev/wasmtime-host,incant_wasm_runtime/embedded-fixture'], cwd=ROOT, env=environment)
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
        rust_guest.compile(rustc,rust,module,env=environment)
        modules = [('bulk', module)]
        for source in sorted((ROOT / 'crates/incant_wasm_runtime/fixtures').glob('*.wat')):
            copied = inputs / source.name; shutil.copy2(source, copied); dest = copied.with_suffix('.wasm')
            run([wabt / f'bin/wat2wasm{suffix}', copied, '-o', dest]); modules.append((source.stem, dest))
        if args.conformance_only:
            sdk_guests = inputs/'sdk-guests'; sdk_guests.mkdir()
            guest_source=inputs/'sdk-gameplay.rs';shutil.copy2(ROOT/'sdk/rust/samples/gameplay.rs',guest_source)
            module_path=sdk_guests/'rust-sdk.wasm'
            rust_guest.compile(rustc,guest_source,module_path,env=environment)
            modules.append(('rust-sdk',module_path))
            failure_source=inputs/'sdk-failure.rs';shutil.copy2(ROOT/'sdk/rust/fixtures/failure.rs',failure_source)
            for kind in ('callback-error','capacity','malformed','expired','log-bounds','log-invalid'):
                name='rust-sdk-'+kind; module_path=sdk_guests/(name+'.wasm')
                rust_guest.compile(rustc,failure_source,module_path,env=environment,cfg=['sdk_'+kind.replace('-','_')])
                modules.append((name,module_path))
            modules.extend(localization_guests(rustc, inputs, sdk_guests, environment))
            modules.extend(animation_guests(rustc, inputs, sdk_guests, environment))
            modules.extend(formatting_guests(rustc, inputs, sdk_guests, environment))
            environment['INCANT_RUST_SDK_GUESTS']=str(sdk_guests)
            flipbook = inputs/'flipbook.rs'; shutil.copy2(ROOT/'crates/incant_game_player/fixtures/flipbook.rs', flipbook)
            flipbook_module = inputs/'flipbook.wasm'
            rust_guest.compile(rustc,flipbook,flipbook_module,env=environment)
            modules.append(('flipbook-game', flipbook_module))
            environment['INCANT_FLIPBOOK_GUEST'] = str(flipbook_module)
            game = inputs/'game.rs'; shutil.copy2(ROOT/'crates/incant_wasm_player/fixtures/game.rs', game)
            game_module = inputs/'game.wasm'
            rust_guest.compile(rustc,game,game_module,env=environment)
            modules.append(('game', game_module))
            for source in sorted((ROOT/'crates/incant_wasm_player/fixtures').glob('*.wat')):
                copied = inputs/source.name; shutil.copy2(source, copied); dest = copied.with_suffix('.wasm')
                run([wabt/f'bin/wat2wasm{suffix}', copied, '-o', dest]); modules.append((source.stem, dest))
        bundle = inputs / 'bundle'; manifest = wasm2c.prepare(bundle, modules, wabt, cook)
        (out / 'prepared-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
        environment.update(INCANT_WASM_BUNDLE=str(bundle), CC_ENABLE_DEBUG_OUTPUT='1')
        (out/'rust-sdk.json').write_text(json.dumps(rust_guest.provenance(),indent=2)+'\n')
        if args.conformance_only:
            commands = [
                [cargo, 'test', '--profile', args.profile, '-j2', '-p', 'incant-module', '-p', 'incant_wasm_abi', '-p', 'incant_wasm_runtime', '-p', 'incant_wasm_cook', '-p', 'incant_wasm_dev', '-p', 'incant_wasm_player', '-p', 'incant_game_runtime', '-p', 'incant_script', '--features', 'incant_wasm_dev/static-conformance,incant_wasm_player/fixture,incant_script/compiler', '--locked'],
                [cargo, 'clippy', '--profile', args.profile, '-j2', '-p', 'incant-module', '-p', 'incant_wasm_abi', '-p', 'incant_wasm_runtime', '-p', 'incant_wasm_cook', '-p', 'incant_wasm_dev', '-p', 'incant_wasm_player', '-p', 'incant_game_runtime', '-p', 'incant_script', '--features', 'incant_wasm_dev/static-conformance,incant_wasm_player/fixture,incant_script/compiler', '--all-targets', '--locked', '--', '-D', 'warnings'],
                [sys.executable, ROOT/'tools/probes/development-wasm.py', out/'development', '--rustup', args.rustup, '--static-conformance', '--profile', args.profile],
            ]
            for index, command in enumerate(commands):
                result = subprocess.run([str(x) for x in command], cwd=ROOT, env=environment, capture_output=True, text=True, encoding='utf-8')
                (out/f'conformance-{index}.log').write_text(result.stdout+result.stderr, encoding='utf-8')
                require(result.returncode == 0, f'Conformance command {index} failed; see its log')
            print('Static/development/input/composed-game conformance and actual Rust guest probe passed. No shipping/device gate claimed.')
            return

        with tempfile.TemporaryDirectory(prefix='static-wasm-build-', dir=out) as storage:
            # PE normally has no COFF symbol table. Request a fresh MSVC linker
            # map from this exact final executable, separately from matched PDBs.
            link_map = out / 'shipping_module.map'
            command = [str(cargo), 'rustc' if kind == 'pdb' else 'build', '-j2', '-p', 'incant_wasm_runtime', '--features', 'embedded-fixture', '--example', 'shipping_module', '--profile', 'shipping', '--target', target, '--target-dir', storage, '--locked', '--config', f'profile.shipping.split-debuginfo="{split}"', '--message-format=json', '-vv']
            if kind == 'pdb':
                command += ['--', '-C', f'link-arg=/MAP:{link_map}']
            build = cargo_output(command, cwd=ROOT, env=environment)
            (out / 'build.log').write_text(build.stderr, encoding='utf-8')
            require(build.returncode == 0, f'Shipping build failed: {out / "build.log"}')
            flags = {}; lines = [line for line in build.stderr.splitlines() if 'Running `' in line and f'--target {target}' in line]
            for name in ('incant_wasm_runtime', 'incant_runtime', 'incant_physics_runtime', 'shipping_module'):
                selected = [line for line in lines if re.search(r'--crate-name '+name+r'\b', line)]
                require(len(selected) == 1, f'Missing fresh compile {name}')
                values = codegen(selected[0]); expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort'}
                if kind == 'pdb': expected['target-feature'] = '+crt-static'
                if name == 'shipping_module': expected.update(lto='fat', debuginfo='2', **{'split-debuginfo': split})
                require(all(values.get(k) == v for k, v in expected.items()), f'Codegen mismatch: {values}')
                require('--test' not in selected[0] and values.get('target-cpu') != 'native', 'Test/nonportable executable')
                flags[name] = {k: values[k] for k in expected}
            require(not any(re.search(r'--crate-name (?:wasmtime\w*|cranelift\w*|wasmparser|wasm_encoder|incant_doc|incant_cmd|incant_agent)\b', line) for line in lines), 'Forbidden target dependency compiled')
            c_flags = verify_c_flags(build.stderr+build.stdout, kind, args.msvc_opt_level)
            artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
            executable = next(Path(a['executable']) for a in artifacts if a.get('reason') == 'compiler-artifact' and a['target']['name'] == 'shipping_module' and a.get('executable'))
            symbols = link_map.read_text(encoding='utf-8') if kind == 'pdb' else run([llvm/f'llvm-nm{suffix}', '--defined-only', '--demangle', executable])
            verify_module_symbols(symbols)
            msvc_compiler = None
            if kind == 'pdb':
                identities = list(Path(storage).glob('*/shipping/build/incant_wasm_runtime-*/out/msvc-compiler.txt'))
                require(len(identities) == 1, 'Missing/ambiguous actual MSVC compiler identity')
                msvc_compiler = identities[0].read_text(encoding='utf-8')
                require('Compiler Version' in msvc_compiler and f'optimization: {args.msvc_opt_level}' in msvc_compiler,
                        'Actual MSVC compiler version/configuration absent')
                shutil.copy2(identities[0], out/'msvc-compiler.txt')
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
              'c_flags': c_flags, 'symbol_audit': 'fresh MSVC linker map' if kind == 'pdb' else 'llvm-nm defined symbols',
              'msvc_compiler': msvc_compiler,
              'link_map_sha256': sha(link_map) if kind == 'pdb' else None,
              'rust_sdk': rust_guest.provenance(), 'guest_source_sha256': guest_source_hash, 'guest_wasm_sha256': guest_wasm_hash,
              'player_sha256': sha(player), 'player_bytes': player.stat().st_size, 'symbols': identity,
              'temporary_guest_sources_wasm_generated_c_tools_and_build_storage_deleted': True,
              'normal_graphs_without_compilers': roots, 'success_processes': 3, 'outputs': outputs, 'rejections': failures,
              'limitations': ['execution establishes only the recorded host/toolchain', 'MSVC /Od baseline is not optimized shipping performance; /O2 remains a conformance candidate', 'repository and validator build storage retained', 'trusted fixture build only, not an arbitrary-source sandbox', 'fixed Wasm memory/table; growth rejected', 'no complete SDK/determinism/mobile/performance gate']}
    (out/'result.json').write_text(json.dumps(report, indent=2)+'\n'); print(json.dumps(report, indent=2))


if __name__ == '__main__': main()
