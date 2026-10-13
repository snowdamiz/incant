"""Trusted captured wasm2c input and closed NDK ARM64 compilation audit."""
import json
from pathlib import Path
import re
import shlex

import android_archive
import ios_wasm
import native_wasm
from native_package import read, require, sha256, write_json


def compiler_identity(ndk, execute):
    wrapper = ndk['clang']
    driver = (ndk['llvm'] / 'bin/clang').resolve(strict=True)
    require(driver.is_relative_to(ndk['root']), 'NDK compiler escapes reviewed root')
    # The reviewed NDK wrapper supplies the API-specific target internally; cc-rs
    # deliberately does not add another --target for this compiler name.
    macros = execute([wrapper, '-dM', '-E', '-x', 'c', '/dev/null']).stdout
    definitions = dict(re.findall(r'^#define (\S+) (.+)$', macros, re.MULTILINE))
    expected = {'__ANDROID__': '1', '__aarch64__': '1', '__SIZEOF_POINTER__': '8',
                '__ANDROID_API__': '__ANDROID_MIN_SDK_VERSION__', '__ANDROID_MIN_SDK_VERSION__': '24',
                '__BYTE_ORDER__': '__ORDER_LITTLE_ENDIAN__', '__ORDER_LITTLE_ENDIAN__': '1234',
                '__SIZEOF_DOUBLE__': '8', '__DBL_MANT_DIG__': '53', '__DBL_MAX_EXP__': '1024'}
    require(all(definitions.get(k) == v for k, v in expected.items()), 'NDK compiler target macros mismatch')
    diagnostic = execute([wrapper, '-###', '-x', 'c', '-c', '/dev/null', '-o', '/dev/null']).stderr
    rows = [shlex.split(line) for line in diagnostic.splitlines() if '"-cc1"' in line]
    require(len(rows) == 1 and Path(rows[0][0]).resolve() == driver,
            'Missing exact NDK compiler driver invocation')
    args = rows[0]
    require(args.count('-triple') == 1 and args[args.index('-triple') + 1] == 'aarch64-unknown-linux-android24',
            'NDK compiler destination mismatch')
    require(args.count('-resource-dir') == 1
            and Path(args[args.index('-resource-dir') + 1]).resolve() == ndk['resource'].resolve(),
            'NDK compiler resource identity mismatch')
    require(str(ndk['sysroot'] / 'usr/include') in diagnostic.replace('/bin/../', '/'),
            'NDK compiler system headers mismatch')
    return {'path': str(wrapper), 'wrapper_sha256': sha256(read(wrapper, 65536)),
            'driver': str(driver), 'driver_sha256': sha256(read(driver, 512 * 1024**2)),
            'target': android_archive.TARGET, 'api': android_archive.API,
            'macros_sha256': sha256(macros.encode()), 'driver_probe_sha256': sha256(diagnostic.encode())}


def prepare(project, scene, staging, storage, cargo, env, execute, wabt, ndk):
    identity = native_wasm.capture(project, scene, storage, cargo, env, execute, wabt,
                                   audit_translation=True)
    if identity is None:
        return None
    selected = native_wasm.selection(project, scene)
    require(selected['fingerprint'] == identity['fingerprint'], 'Captured selector changed')
    expected = {'kind': 'wasm', 'module': selected['module'], **identity, 'translator': 'wasm2c-1.0.42'}
    ios_wasm.program(expected, expected)
    bundle = storage / 'wasm-bundle'
    manifest = json.loads(read(bundle / 'manifest.json', 1024 * 1024))
    require(manifest['wabt'] == '1.0.42' and manifest['modules'] == [{
        'name': 'scene-program', 'prefix': 'm0', 'input_sha256': identity['input_sha256'],
        'prepared_sha256': identity['prepared_sha256']}], 'Translated module identity mismatch')
    names = {'m0.original.wasm', 'm0.wasm', 'm0.c', 'm0.h', 'm0-facade.c', 'linked.rs'}
    require(set(manifest['files']) == names, 'Unexpected translated module file set')
    for name in names:
        require(sha256(read(bundle / name, 128 * 1024**2)) == manifest['files'][name],
                'Translated module file checksum mismatch')
    write_json(staging / 'wasm-translation.json', manifest)
    compiler = compiler_identity(ndk, execute)
    env['CC_ENABLE_DEBUG_OUTPUT'] = '1'
    return {'program': expected, 'translation': manifest, 'compiler': compiler, 'bundle': str(bundle)}


def audit_build(stdout, stderr, target, prepared, root):
    messages = [json.loads(line) for line in stdout.splitlines() if line.startswith('{')]
    packages = {row['package_id'] for row in messages if row.get('reason') == 'compiler-artifact'
                and row['target']['name'] == 'incant_wasm_runtime'}
    require(len(packages) == 1, 'Missing unique static runtime target artifact')
    outputs = [Path(row['out_dir']) for row in messages if row.get('reason') == 'build-script-executed'
               and row['package_id'] in packages]
    require(len(outputs) == 1, 'Missing unique static runtime build-script output')
    directory = outputs[0]
    require(directory.is_absolute() and directory.name == 'out'
            and re.fullmatch('incant_wasm_runtime-[0-9a-f]+', directory.parent.name)
            and directory.parents[1].name == 'build' and directory.parents[2].name == 'shipping'
            and directory.parents[3].name == target, 'Unexpected static runtime build-script directory')
    compiler = prepared['compiler']
    require(sha256(read(Path(compiler['path']), 65536)) == compiler['wrapper_sha256']
            and sha256(read(Path(compiler['driver']), 512 * 1024**2)) == compiler['driver_sha256'],
            'NDK C compiler changed during archive build')
    return compiler_audit(stdout + '\n' + stderr, target, prepared, root, directory)


def compiler_audit(log, target, prepared, root, probe_dir=None):
    require(target == android_archive.TARGET and prepared['compiler']['target'] == target
            and prepared['compiler']['api'] == android_archive.API, 'Unsupported static module C target')
    bundle = Path(prepared['bundle'])
    expected_sources = {str((root / 'third_party/wabt' / name).resolve())
                        for name in ('wasm-rt-impl.c', 'wasm-rt-mem-impl.c')}
    expected_sources.add(str((root / 'crates/incant_wasm_runtime/c/imports.c').resolve()))
    expected_sources.update(str((bundle / name).resolve()) for name in ('m0.c', 'm0-facade.c'))
    required = {'-O3', '-std=c11', '-fno-optimize-sibling-calls', '-frounding-math',
                '-ffp-contract=off', '-Werror=implicit-function-declaration',
                '-DWASM_RT_USE_MMAP=0', '-DWASM_RT_MEMCHECK_BOUNDS_CHECK=1',
                '-DWASM_RT_STACK_DEPTH_COUNT=1', '-DWASM_RT_MAX_CALL_STACK_DEPTH=64',
                '-DWASM_RT_USE_SEGUE=0'}
    found, probes = {}, []
    for line in log.splitlines():
        match = re.fullmatch(r'\[incant_wasm_runtime [^\]]+\] running: (.+)', line)
        if match is None:
            continue
        args = shlex.split(match.group(1))
        if '-c' not in args:
            continue
        require(args.count('-c') == 1 and args.index('-c') + 1 < len(args), 'Invalid target C compile command')
        source = str(Path(args[args.index('-c') + 1]).resolve())
        executable = next((arg for arg in args if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*=.*', arg)), None)
        require(executable == prepared['compiler']['path'], 'Target C compiler identity mismatch')
        # The hash-bound API24 NDK wrapper is authoritative. Command-line target,
        # sysroot, toolchain, or response-file overrides must never retarget it.
        require(not any(arg.startswith(('--target', '-target', '--sysroot', '-isysroot', '-resource-dir',
                                        '--gcc-toolchain', '-B', '@', '-arch', '-march', '-mcpu', '-mtune'))
                        or arg in ('-m32', '-m64', '-cc1') for arg in args),
                'Static C destination/toolchain override')
        if probe_dir is not None and source == str((probe_dir / 'flag_check.c').resolve()):
            require(not probes and args.count('-o') == 1 and args.index('-o') + 1 < len(args)
                    and str(Path(args[args.index('-o') + 1]).resolve()) == str((probe_dir / 'flag_check').resolve())
                    and [arg for arg in args if re.fullmatch(r'-O.+', arg)] == ['-O0'],
                    'Unexpected compiler flag probe')
            probes.append({'kind': 'cc_flag_check', 'argv': args, 'command_sha256': sha256(line.encode())})
            continue
        require(source in expected_sources and source not in found, 'Unexpected or duplicate target C source')
        require(all(args.count(flag) == 1 for flag in required), 'Static C safety/numerical flags missing')
        require(not any(flag in args for flag in ('-ffast-math', '-Ofast', '-fno-rounding-math', '-funsafe-math-optimizations'))
                and not any(arg.startswith('-UWASM_RT_') for arg in args)
                and not any(arg.startswith('-DWASM_RT_') and arg not in required for arg in args)
                and [arg for arg in args if re.fullmatch(r'-O.+', arg)] == ['-O3']
                and [arg for arg in args if arg.startswith('-ffp-contract=')] == ['-ffp-contract=off'],
                'Static C safety/numerical flags overridden')
        found[source] = {'argv': args, 'command_sha256': sha256(line.encode())}
    require(set(found) == expected_sources, 'Missing actual target static-module C compilation')
    return {'target': target, 'api': android_archive.API, 'compiler': prepared['compiler'], 'sources': found,
            'build_script_output': None if probe_dir is None else str(probe_dir), 'compiler_probes': probes}
