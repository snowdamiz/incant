#!/usr/bin/env python3
"""Build an explicitly experimental trusted numeric Android C-ABI caller.

This is not production admission or authored export. Both ends opt into the
compile-time probe feature, and the producer restricts its source to the fixture.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import shlex
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
from native_process import run

TARGET = 'aarch64-linux-android'
FEATURES = ['mobile-ffi', 'probe-android-bytecode']
NDK = '28.2.13676358'
PARSERS = ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_function_decl')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


class Commands:
    def __init__(self, directory, env):
        self.directory, self.env, self.rows = directory, env, []

    def execute(self, argv, timeout=1800):
        argv = list(map(str, argv))
        stem = self.directory / f'command-{len(self.rows):03d}'
        stdout, stderr = Path(str(stem) + '.stdout'), Path(str(stem) + '.stderr')
        row = {'argv': argv, 'cwd': str(ROOT), 'exit_code': None}
        self.rows.append(row)
        start = time.monotonic()
        try:
            with stdout.open('wb') as out, stderr.open('wb') as err:
                result = run(argv, cwd=ROOT, env=self.env, timeout=timeout, stdout=out, stderr=err)
            row['exit_code'] = result.returncode
        finally:
            row['seconds'] = time.monotonic() - start
            for key, path in [('stdout', stdout), ('stderr', stderr)]:
                row[key], row[key + '_sha256'] = str(path), digest(path)
            write(self.directory / 'commands.json', self.rows)
        if result.returncode:
            raise RuntimeError(f'command-{len(self.rows)-1:03d} failed: {stderr}')
        return stdout.read_text(), stderr.read_text()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ndk', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    ndk = args.ndk.resolve(strict=True)
    properties = (ndk / 'source.properties').read_text()
    if re.search(r'^Pkg.Revision\s*=\s*(\S+)', properties, re.M)[1] != NDK:
        parser.error('This candidate pins NDK ' + NDK)
    llvm = ndk / 'toolchains/llvm/prebuilt/darwin-x86_64'
    clang = llvm / 'bin/aarch64-linux-android24-clang'
    if not clang.is_file():
        parser.error('Expected the macOS NDK ARM64/API24 compiler')
    env = os.environ.copy()
    for key, value in env.items():
        if value and (key.startswith(('CFLAGS', 'HOST_CFLAGS', 'TARGET_CFLAGS', 'CARGO_PROFILE_', 'BINDGEN_EXTRA_CLANG_ARGS'))
                      or (key.startswith('CARGO_TARGET_') and key.endswith('_RUSTFLAGS'))
                      or key in ('RUSTC', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')):
            parser.error('Unset unreviewed compiler override: ' + key)
    for key in list(env):
        if key.startswith(('INCANT_NATIVE_', 'INCANT_MOBILE_', 'INCANT_AUTHORED_WASM')) or key == 'INCANT_WASM_BUNDLE':
            del env[key]
    source = ROOT / 'crates/incant_script/tests/fixtures/mobile_compat.js'
    expected = source.with_suffix('.expected.json')
    selected = {
        'CC_aarch64_linux_android': str(clang),
        'CXX_aarch64_linux_android': str(llvm / 'bin/aarch64-linux-android24-clang++'),
        'AR_aarch64_linux_android': str(llvm / 'bin/llvm-ar'),
        'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER': str(clang),
        'LIBCLANG_PATH': str(llvm / 'lib'),
        'BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android': shlex.join([
            '--target=aarch64-linux-android24', '--sysroot=' + str(llvm / 'sysroot'),
            '-resource-dir=' + str(llvm / 'lib/clang/19')]),
        'INCANT_NATIVE_SCRIPT_SOURCE': str(source),
        'INCANT_NATIVE_SCRIPT_NAME': 'mobile-compat.js',
    }
    env.update(selected)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    commands = Commands(output, env)
    result = {'passed': False, 'admission': 'experimental compile-time numeric probe only',
              'target': TARGET, 'features': FEATURES, 'profile': 'ci', 'ndk': NDK,
              'ndk_properties_sha256': digest(ndk / 'source.properties'), 'environment': selected,
              'source': {'path': str(source), 'sha256': digest(source)},
              'expectations': {'path': str(expected), 'sha256': digest(expected)}}
    try:
        rustup = shutil.which('rustup')
        if rustup is None:
            fallback = Path.home() / '.cargo/bin/rustup'
            if not fallback.is_file():
                raise RuntimeError('rustup is required to resolve the repository compiler')
            rustup = str(fallback)
        compiler_path, _ = commands.execute([rustup, 'which', 'rustc'])
        compiler_path = Path(compiler_path.strip()).resolve(strict=True)
        env['PATH'] = str(compiler_path.parent) + os.pathsep + env.get('PATH', '')
        result['compiler_path'] = str(compiler_path)
        result['compiler_sha256'] = digest(compiler_path)
        compiler, _ = commands.execute([compiler_path, '-vV'])
        assert re.search(r'^host: (.+)$', compiler, re.M)[1] == 'aarch64-apple-darwin'
        result['rustc'] = compiler
        result['clang'], _ = commands.execute([clang, '--version'])
        graph, _ = commands.execute([ROOT / 'tools/cargo', 'tree', '--locked', '-p', 'incant_game_player',
                                     '--no-default-features', '--features', ','.join(FEATURES), '--target', TARGET,
                                     '-e', 'normal', '--prefix', 'none', '--format', '{p}|{f}'])
        forbidden = {'incant_doc', 'incant_cmd', 'incant_cook', 'incant_agent', 'incant_wasm_dev', 'wasmtime'}
        assert not (set(line.split()[0] for line in graph.splitlines()) & forbidden)
        assert all('compiler' not in line.split('|')[-1].split(',') for line in graph.splitlines()
                   if line.startswith('incant_script '))
        result['normal_graph_sha256'] = hashlib.sha256(graph.encode()).hexdigest()
        with tempfile.TemporaryDirectory(prefix='candidate-target-', dir=output) as temporary:
            target = Path(temporary)
            stdout, _ = commands.execute([ROOT / 'tools/cargo', 'rustc', '--locked', '--profile', 'ci', '-j2',
                '-p', 'incant_game_player', '--lib', '--no-default-features', '--features', ','.join(FEATURES),
                '--crate-type', 'staticlib', '--target', TARGET, '--target-dir', target,
                '--message-format=json', '-vv'])
            messages = [json.loads(line) for line in stdout.splitlines() if line.startswith('{')]
            built = [m for m in messages if m.get('reason') == 'compiler-artifact'
                     and m['target']['name'] == 'incant_game_player' and 'staticlib' in m['target']['crate_types']]
            assert len(built) == 1
            archives = [Path(p) for p in built[0]['filenames'] if p.endswith('.a')]
            assert len(archives) == 1
            shutil.copy2(archives[0], output / 'libincant_game_player.a')
            producers = [Path(m['out_dir']) for m in messages if m.get('reason') == 'build-script-executed'
                         and m['package_id'] == built[0]['package_id']]
            assert len(producers) == 1
            for name in ['behavior.rs', 'script-symbols.json']:
                shutil.copy2(producers[0] / name, output / name)
        result['private_build_storage_removed'] = not target.exists()
        metadata = json.loads((output / 'script-symbols.json').read_text())
        assert metadata['target'] == TARGET and metadata['producer_target'] == 'aarch64-apple-darwin'
        assert metadata['source_name'] == 'mobile-compat.js' and metadata['format'] == 3
        assert metadata['bytecode_abi'] == {
            'version': 28, 'pointer_bits': 64, 'little_endian': True, 'value_bytes': 16,
            'value_alignment': 8, 'atom_bytes': 4, 'float_mantissa': 53,
            'float_max_exponent': 1024, 'write_flags': 17, 'default_c_flags': True}
        arrays = re.search(r'bytecode: &(\[[\d, ]+\]), hardening: &(\[[\d, ]+\])',
                           (output / 'behavior.rs').read_text())
        assert arrays
        for index, key in [(1, 'bytecode_sha256'), (2, 'hardening_sha256')]:
            blob = bytes(json.loads(arrays[index]))
            assert blob[0] == 28 and hashlib.sha256(blob).hexdigest() == metadata[key]
        assert digest(source) == result['source']['sha256']
        assert digest(expected) == result['expectations']['sha256']
        result['script'] = metadata
        header = output / 'incant_game.h'
        shutil.copy2(ROOT / 'crates/incant_game_player/include/incant_game.h', header)
        caller = output / 'caller-c'
        c_source = ROOT / 'tools/probes/mobile-runtime-library/caller.c'
        commands.execute([clang, '-std=c11', '-Wall', '-Wextra', '-Werror', '-I', output, c_source,
                          output / 'libincant_game_player.a', '-Wl,--gc-sections', '-ldl', '-llog', '-lm',
                          '-lc++_static', '-lc++abi', '-o', caller])
        headers, _ = commands.execute([llvm / 'bin/llvm-readelf', '--file-header', '--dynamic', caller])
        assert 'AArch64' in headers and 'DYN (Shared object file)' in headers
        required = set(re.findall(r'Shared library: \[([^]]+)\]', headers))
        assert required <= {'libc.so', 'libm.so', 'libdl.so', 'liblog.so'}, required
        names, _ = commands.execute([llvm / 'bin/llvm-nm', '--defined-only', caller])
        assert all(name in names for name in ('incant_game_run_v1', 'JS_ReadObject', 'JS_EvalFunction'))
        assert not any(name in names for name in PARSERS)
        result.update(passed=True, required_system_libraries=sorted(required),
                      caller_source_sha256=digest(c_source),
                      artifacts={name: {'path': str(output / name), 'sha256': digest(output / name),
                                        'bytes': (output / name).stat().st_size}
                                 for name in ['libincant_game_player.a', 'caller-c', 'incant_game.h',
                                              'behavior.rs', 'script-symbols.json']})
    finally:
        result['commands'] = commands.rows
        write(output / 'result.json', result)
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
