"""Closed ARM Android authored archive contract and target audits.

The fresh-source bytecode route was exercised on API37 ARM Android. NDK API24
compile/link identity is recorded separately; it is not an API24 execution claim.
"""
import json
import os
from pathlib import Path
import re
import shlex
import shutil

import ios_archive
import ios_wasm
import native_wasm
import physics_export
from native_package import read, require, sha256

TARGET = 'aarch64-linux-android'
PRODUCER = 'aarch64-apple-darwin'
NDK_REVISION = '28.2.13676358'
API = 24
NAME = 'incant_game_player'
CAPABILITIES = ['mobile-ffi', 'project', 'render-input']
PARSERS = ios_archive.PARSERS
ABI = {'version': 28, 'pointer_bits': 64, 'little_endian': True, 'value_bytes': 16,
       'value_alignment': 8, 'atom_bytes': 4, 'float_mantissa': 53,
       'float_max_exponent': 1024, 'write_flags': 17, 'default_c_flags': True}


def admit(host, target):
    require(host == PRODUCER and target == TARGET,
            'Authored Android archive requires macOS arm64 and aarch64-linux-android')


def toolchain(ndk):
    ndk = ndk.resolve(strict=True)
    properties = read(ndk / 'source.properties', 65536).decode()
    match = re.search(r'^Pkg.Revision\s*=\s*(\S+)', properties, re.MULTILINE)
    require(match is not None and match[1] == NDK_REVISION,
            'Authored Android archive requires reviewed NDK ' + NDK_REVISION)
    llvm = ndk / 'toolchains/llvm/prebuilt/darwin-x86_64'
    paths = {'clang': llvm / f'bin/aarch64-linux-android{API}-clang',
             'clangxx': llvm / f'bin/aarch64-linux-android{API}-clang++',
             'ar': llvm / 'bin/llvm-ar', 'strip': llvm / 'bin/llvm-strip',
             'nm': llvm / 'bin/llvm-nm', 'objdump': llvm / 'bin/llvm-objdump',
             'readobj': llvm / 'bin/llvm-readobj', 'readelf': llvm / 'bin/llvm-readelf'}
    require(all(path.is_file() for path in paths.values()), 'Reviewed NDK tools are missing')
    paths.update(root=ndk, llvm=llvm, sysroot=llvm / 'sysroot', resource=llvm / 'lib/clang/19')
    require(paths['sysroot'].is_dir() and paths['resource'].is_dir(), 'NDK header roots are missing')
    return paths


def tool_hashes(ndk):
    """Hash trusted tool symlinks inside the canonical NDK, never package links.

    Keep the original invocation path: LLVM multi-call tools select their action
    from argv[0], so executing the resolved llvm-objcopy as llvm-strip is wrong.
    """
    result = {}
    for name, path in ndk.items():
        if path.is_file():
            resolved = path.resolve(strict=True)
            require(resolved.is_relative_to(ndk['root']), 'NDK tool escapes reviewed root: ' + name)
            result[name] = sha256(read(resolved, 512 * 1024**2))
    return result


def preflight_project(project, scene, wabt=None):
    payload = read(project, 64 * 1024**2)
    mode = physics_export.project_mode(payload)
    document = json.loads(payload)
    require(isinstance(document.get('scenes'), dict), 'Missing project scenes')
    scenes = document['scenes']
    if scene is None:
        require(len(scenes) == 1, 'Choose --scene when the project does not contain exactly one scene')
        scene = next(iter(scenes))
    require(scene in scenes and isinstance(scenes[scene], dict), 'Selected project scene is missing')
    if native_wasm.selection(project, scene) is not None:
        require(wabt is not None, 'Wasm export requires --wabt pointing to trusted WABT 1.0.42 tools')
    return mode


def environment(source, rustc, project, scene, ndk):
    for name, value in source.items():
        compiler = name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
                           'RUSTC_WORKSPACE_WRAPPER', 'CC', 'CXX', 'AR', 'CLANG_PATH',
                           'HOST_CC', 'TARGET_CC', 'HOST_CXX', 'TARGET_CXX', 'HOST_AR', 'TARGET_AR')
        compiler |= name.startswith(('CARGO_PROFILE_', 'CFLAGS', 'HOST_CFLAGS', 'TARGET_CFLAGS',
                                     'BINDGEN_EXTRA_CLANG_ARGS', 'CC_', 'CXX_', 'AR_'))
        compiler |= name.endswith('_RUSTFLAGS') or (name.startswith('CARGO_TARGET_') and name.endswith('_LINKER'))
        require(not (compiler and value), f'Unset {name} for the audited Android exporter')
    env = {key: value for key, value in source.items()
           if not key.startswith(('INCANT_NATIVE_', 'INCANT_MOBILE_', 'INCANT_WASM_', 'INCANT_AUTHORED_WASM'))
           and key not in ('SDKROOT', 'MACOSX_DEPLOYMENT_TARGET', 'IPHONEOS_DEPLOYMENT_TARGET')}
    selected = {'CC_aarch64_linux_android': str(ndk['clang']),
                'CXX_aarch64_linux_android': str(ndk['clangxx']),
                'AR_aarch64_linux_android': str(ndk['ar']),
                'CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER': str(ndk['clang']),
                'LIBCLANG_PATH': str(ndk['llvm'] / 'lib'),
                'BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android': shlex.join([
                    f'--target=aarch64-linux-android{API}', '--sysroot=' + str(ndk['sysroot']),
                    '-resource-dir=' + str(ndk['resource'])]),
                'INCANT_MOBILE_PROJECT': str(project)}
    if scene is not None:
        selected['INCANT_MOBILE_PROJECT_SCENE'] = scene
    env.update(selected)
    env['PATH'] = str(rustc.parent) + os.pathsep + env.get('PATH', '')
    return env, selected


def normal_dependencies(graph):
    # This target-independent selected-graph audit is shared with reviewed iOS.
    names = ios_archive.normal_dependencies(graph)
    for line in graph.splitlines():
        name, flags = line.removesuffix(' (*)').split('|')
        if name.split()[0] in (NAME, 'incant_script'):
            require(not (set(flags.split(',')) & {'compiler', 'probe-android-bytecode'}),
                    'Compiler or experimental probe feature leaked into Android runtime')
    return names


def verify_codegen(stderr, static_module=False):
    result = ios_archive.verify_codegen(stderr, TARGET, static_module)
    for line in stderr.splitlines():
        if 'Running `' in line and f'--target {TARGET}' in line:
            require('feature="probe-android-bytecode"' not in line,
                    'Experimental probe feature leaked into public Android archive')
    return result


def producer_receipt(receipt, project_hash, mode, module=None):
    require(receipt.get('format') == 'incant-native-project-build' and receipt.get('version') == 1,
            'Unsupported authored project producer receipt')
    require(receipt['target'] == TARGET and receipt['producer_target'] == PRODUCER
            and receipt['producer_kind'] == 'android-archive' and receipt['project_sha256'] == project_hash
            and receipt['startup_source_parsing'] is False,
            'Authored Android producer identity mismatch')
    ios_wasm.program(receipt['program'], module)
    if module is not None:
        require(receipt['compiler'] is None, 'Static module producer cannot include TypeScript bindings')
    physics_export.verify_producer(receipt, mode)
    script = receipt['script']
    require(script['format'] == 3 and script['target'] == TARGET and script['producer_target'] == PRODUCER
            and script['engine'] == 'rquickjs-0.14.0/quickjs-ng-0.16.2/bytecode-28'
            and script['bytecode_abi'] == ABI and script['embedded_source'] is False,
            'Authored Android script ABI mismatch')
    for field in ('build_id', 'source_sha256', 'bytecode_sha256', 'hardening_sha256'):
        require(isinstance(script[field], str) and re.fullmatch('[0-9a-f]{64}', script[field]),
                f'Invalid script {field}')
    if receipt['compiler'] is not None:
        require(isinstance(script['bindings_sha256'], str) and re.fullmatch('[0-9a-f]{64}', script['bindings_sha256'])
                and receipt['compiler']['bundleSha256'] == script['source_sha256']
                and receipt['compiler']['mapSha256'] == script['source_map_sha256'],
                'Authored Android source graph/binding mismatch')
    else:
        require(script['bindings_sha256'] is None, 'Unexpected empty-scene script binding')


def archive_platform(headers):
    require(set(re.findall(r'^Arch: (.+)$', headers, re.MULTILINE)) == {'aarch64'},
            'Archive contains a non-arm64 object')
    require(set(re.findall(r'^Format: (.+)$', headers, re.MULTILINE)) == {'elf64-littleaarch64'},
            'Archive contains a non-ELF64 ARM object')
    return {'architecture': 'aarch64', 'format': 'elf64-littleaarch64',
            'headers_sha256': sha256(headers.encode()),
            'platform_identity': 'actual Rust target and NDK compiler receipts; ELF OSABI alone is insufficient'}


def package_archive(archive, staging, ndk, execute):
    symbols, runtime = staging / 'symbols', staging / 'runtime'
    symbols.mkdir()
    runtime.mkdir()
    original = symbols / 'libincant_game_player.unstripped.a'
    shutil.copy2(archive, original)
    copied = runtime / 'libincant_game_player.a'
    before = execute([ndk['objdump'], '--section-headers', original]).stdout
    require('.debug_info' in before and '.debug_line' in before, 'Archive is missing retained DWARF')
    execute([ndk['strip'], '--strip-debug', '-o', copied, original])
    after = execute([ndk['objdump'], '--section-headers', copied]).stdout
    require('.debug_' not in after, 'Debug sections remain in runtime archive')
    names = execute([ndk['nm'], '--defined-only', '--demangle', copied]).stdout
    require(all(name in names for name in ('incant_game_run_v1', 'JS_ReadObject', 'JS_EvalFunction')),
            'Missing mobile ABI or bytecode runtime symbols')
    headers = execute([ndk['readobj'], '--file-headers', copied]).stdout
    return {'format': 'object-dwarf-archive', 'platform': archive_platform(headers),
            'unstripped_sha256': sha256(read(original, 512 * 1024**2)),
            'runtime_sha256': sha256(read(copied, 512 * 1024**2)),
            'debug_sections_removed': True, 'before_sections_sha256': sha256(before.encode()),
            'after_sections_sha256': sha256(after.encode()), 'defined_symbols_sha256': sha256(names.encode()),
            'unused_parser_definitions': [name for name in PARSERS if name in names],
            'final_link_requirements': ['gc-sections', 'independent executable symbol audit', 'separate DWARF retention']}
