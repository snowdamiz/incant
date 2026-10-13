"""Trusted iOS static-module preparation and actual target C compiler audits."""
import json
from pathlib import Path
import re
import shlex

import native_wasm
from native_package import read, require, sha256


def program(actual, expected):
    if expected is None:
        require(actual is None, 'Unexpected static module producer')
        return
    require(isinstance(actual, dict) and actual == expected,
            'Captured iOS static module producer identity mismatch')
    require(set(actual) == {'kind', 'module', 'fingerprint', 'input_sha256',
                           'prepared_sha256', 'registry_sha256', 'translator'}
            and actual['kind'] == 'wasm' and actual['translator'] == 'wasm2c-1.0.42'
            and re.fullmatch(r'[0-7][0-9A-HJKMNP-TV-Z]{25}', actual['module']),
            'Unsupported static module producer shape')
    for key in ('fingerprint', 'input_sha256', 'prepared_sha256', 'registry_sha256'):
        require(isinstance(actual[key], str) and re.fullmatch('[0-9a-f]{64}', actual[key]),
                'Invalid captured module hash: ' + key)


def prepare(project, scene, staging, storage, cargo, env, execute, wabt, sdk, target):
    # The cooker is a host tool; the explicit iOS SDK belongs only to the archive.
    host_env = {key: value for key, value in env.items() if key != 'SDKROOT'}
    identity = native_wasm.capture(project, scene, storage, cargo, host_env, execute, wabt,
                                   audit_translation=True)
    if identity is None:
        return None
    env.update({key: value for key, value in host_env.items()
                if key.startswith('INCANT_NATIVE_WASM_') or key == 'INCANT_WASM_BUNDLE'})
    selected = native_wasm.selection(project, scene)
    require(selected['fingerprint'] == identity['fingerprint'], 'Captured selector changed')
    expected = {'kind': 'wasm', 'module': selected['module'], **identity, 'translator': 'wasm2c-1.0.42'}
    program(expected, expected)
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
    # These are build-audit data, never executable input read from the project.
    (staging / 'wasm-translation.json').write_text(json.dumps(manifest, indent=2) + '\n')
    compiler = Path(execute(['xcrun', '--sdk', sdk, '--find', 'clang']).stdout.strip())
    require(compiler.is_absolute() and compiler.is_file(), 'Missing trusted Apple C compiler')
    version = execute([compiler, '--version']).stdout
    require(version.startswith('Apple clang version '), 'Expected Apple clang for iOS static module')
    env['CC_' + target.replace('-', '_')] = str(compiler)
    env['CC_ENABLE_DEBUG_OUTPUT'] = '1'
    return {'program': expected, 'translation': manifest,
            'compiler': {'path': str(compiler), 'resolved_path': str(compiler.resolve()),
                         'sha256': sha256(compiler.resolve().read_bytes()), 'version': version},
            'bundle': str(bundle)}


def compiler_audit(log, target, sdkroot, prepared, root):
    """Audit cc-rs' actual invocations, never an assumed flag configuration."""
    expected_target = {'aarch64-apple-ios': 'arm64-apple-ios',
                       'aarch64-apple-ios-sim': 'arm64-apple-ios-simulator'}
    require(target in expected_target, 'Unsupported static module C target')
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
    found = {}
    for line in log.splitlines():
        match = re.fullmatch(r'\[incant_wasm_runtime [^\]]+\] running: (.+)', line)
        if match is None:
            continue
        args = shlex.split(match.group(1))
        if '-c' not in args:
            continue  # Compiler-family probes and archiver commands have no -c.
        require(args.count('-c') == 1 and args.index('-c') + 1 < len(args), 'Invalid target C compile command')
        source = str(Path(args[args.index('-c') + 1]).resolve())
        require(source in expected_sources and source not in found, 'Unexpected or duplicate target C source')
        executable = next((arg for arg in args if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*=.*', arg)), None)
        require(executable == prepared['compiler']['path'], 'Target C compiler identity mismatch')
        require(all(args.count(flag) == 1 for flag in required), 'Static C safety/numerical flags missing')
        require([arg for arg in args if arg.startswith('--target=')] == ['--target=' + expected_target[target]],
                'Static C destination differs from archive target')
        require(args.count('-isysroot') == 1 and args.index('-isysroot') + 1 < len(args)
                and args[args.index('-isysroot') + 1] == sdkroot,
                'Static C SDK identity mismatch')
        forbidden = ('-ffast-math', '-Ofast', '-fno-rounding-math', '-funsafe-math-optimizations')
        require(not any(flag in args for flag in forbidden)
                and not any(arg.startswith('-UWASM_RT_') for arg in args)
                and not any(arg.startswith('-DWASM_RT_') and arg not in required for arg in args)
                and [arg for arg in args if re.fullmatch(r'-O.+', arg)] == ['-O3']
                and [arg for arg in args if arg.startswith('-ffp-contract=')] == ['-ffp-contract=off'],
                'Static C safety/numerical flags overridden')
        found[source] = {'argv': args, 'command_sha256': sha256(line.encode())}
    require(set(found) == expected_sources, 'Missing actual target static-module C compilation')
    return {'target': target, 'sdkroot': sdkroot, 'compiler': prepared['compiler'], 'sources': found}
