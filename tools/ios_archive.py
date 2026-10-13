"""Closed iOS archive admission and audits, separate from desktop export policy."""
import os
from pathlib import Path
import re

import physics_export
import ios_wasm
import native_wasm
from physics_features import SOLVERS, feature_errors

from native_package import read, require, sha256
from shipping import codegen

TARGETS = {'aarch64-apple-ios-sim': 'iphonesimulator', 'aarch64-apple-ios': 'iphoneos'}
CAPABILITIES = ['mobile-ffi', 'project', 'render-input']
# Preserve the historical deterministic caller probe's explicit admission.
FEATURES = physics_export.cargo_features('deterministic', CAPABILITIES)
NAME = 'incant_game_player'
FORBIDDEN = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook',
             'incant_source', 'incant_play', 'incant_play_legacy', 'incant_wasm_cook',
             'incant_wasm_dev', 'loro', 'automerge'}
PARSERS = ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program',
           'js_parse_function_decl', 'incant_script::compiler')


def admit(host, target):
    require(host == 'aarch64-apple-darwin' and target in TARGETS,
            'Authored iOS archive requires macOS arm64 and an explicitly supported iOS destination')


def preflight_project(project, scene, wabt=None):
    """Early bounded refusal only; the Rust producer owns schema admission."""
    import json
    payload = read(project, 64 * 1024**2)
    mode = physics_export.project_mode(payload)
    document = json.loads(payload)
    require(isinstance(document, dict) and isinstance(document.get('scenes'), dict), 'Missing project scenes')
    scenes = document['scenes']
    if scene is None:
        require(len(scenes) == 1, 'Choose --scene when the project does not contain exactly one scene')
        scene = next(iter(scenes))
    require(scene in scenes and isinstance(scenes[scene], dict), 'Selected project scene is missing')
    if native_wasm.selection(project, scene) is not None:
        require(wabt is not None, 'Wasm export requires --wabt pointing to trusted WABT 1.0.42 tools')
    return mode


def environment(source, rustc, project, scene):
    for name, value in source.items():
        if (name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
                     'RUSTC_WORKSPACE_WRAPPER', 'CFLAGS', 'HOST_CFLAGS', 'TARGET_CFLAGS')
                or name in ('CC', 'HOST_CC', 'TARGET_CC', 'AR', 'HOST_AR', 'TARGET_AR')
                or name.startswith(('CARGO_PROFILE_', 'CFLAGS_', 'CC_', 'AR_')) or name.endswith('_RUSTFLAGS')):
            require(not value, f'Unset {name} for the audited iOS exporter')
    env = {k: v for k, v in source.items() if not k.startswith(('INCANT_NATIVE_', 'INCANT_MOBILE_',
                                                               'INCANT_WASM_', 'INCANT_AUTHORED_WASM'))}
    env.update(PATH=str(rustc.parent) + os.pathsep + env.get('PATH', ''), INCANT_MOBILE_PROJECT=str(project))
    if scene is not None:
        env['INCANT_MOBILE_PROJECT_SCENE'] = scene
    return env


def normal_dependencies(graph):
    """Audit only the selected target's normal graph, preserving compiler refusal."""
    names = []
    for line in graph.splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
        require(match is not None, 'Malformed selected iOS dependency graph')
        name = match.group(1)
        require(name not in FORBIDDEN and not name.startswith(('wasmtime', 'cranelift')),
                f'Authoring/JIT normal dependency: {name}')
        names.append(name)
    require(names and names[0] == NAME, 'Missing selected mobile player root')
    return sorted(set(names))


def target_invocations(stderr, target):
    # Device triples are prefixes of simulator triples; compare the complete token.
    pattern = re.compile(r'--target ' + re.escape(target) + r'(?=\s|`)')
    return [line for line in stderr.splitlines() if 'Running `' in line and pattern.search(line)]


def verify_codegen(stderr, target, static_module=False):
    target_lines = target_invocations(stderr, target)
    flags = {}
    crates = ['incant_runtime', 'incant_script', NAME]
    if static_module:
        crates.append('incant_wasm_runtime')
    for name in crates:
        commands = [line for line in target_lines if re.search(r'--crate-name ' + name + r'\b', line)]
        require(len(commands) == 1, f'Missing fresh target invocation for {name}')
        require('feature="compiler"' not in commands[0], 'Runtime source compiler feature leaked')
        if static_module and name == 'incant_wasm_runtime':
            require('feature="embedded-fixture"' in commands[0], 'Missing trusted static module compilation')
        actual = codegen(commands[0])
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort',
                    'debuginfo': '2', 'split-debuginfo': 'off'}
        if name == NAME:
            expected['lto'] = 'fat'
            require('--crate-type staticlib' in commands[0], 'Expected actual static archive compilation')
        require(all(actual.get(k) == v for k, v in expected.items()) and actual.get('target-cpu') != 'native',
                f'Incorrect shipping codegen for {name}: {actual}')
        flags[name] = actual
    for line in target_lines:
        crate = re.search(r'--crate-name (\w+)\b', line)
        if crate:
            name = crate.group(1)
            require(name not in FORBIDDEN and not name.startswith(('wasmtime', 'cranelift')),
                    f'Authoring/JIT crate compiled for archive target: {name}')
    return flags


def kernel_provenance(stderr, target, mode='deterministic'):
    """Audit all five fresh target solver invocations for the requested family."""
    require(mode in physics_export.MODES, 'Unsupported project physics mode')
    result = {}
    for name in SOLVERS:
        lines = [line for line in target_invocations(stderr, target)
                 if re.search(r'--crate-name ' + name + r'\b', line)]
        require(len(lines) == 1, f'Missing fresh target physics invocation for {name}')
        features = sorted(set(re.findall(r'--cfg \'feature="([^"\']+)"\'', lines[0])))
        result[name] = {'features': features, 'command_sha256': sha256(lines[0].encode())}
    errors = feature_errors({name: row['features'] for name, row in result.items()}, mode, target)
    require(not errors, 'Target physics invocation differs from selected mode: ' + '; '.join(errors))
    return {'mode': mode, 'target': target, 'crates': result}


def producer_receipt(receipt, target, project_hash, mode='deterministic', module=None):
    require(receipt.get('format') == 'incant-native-project-build' and receipt.get('version') == 1,
            'Unsupported authored project producer receipt')
    physics_export.verify_producer(receipt, mode)
    require(receipt['target'] == target and receipt['producer_target'] == 'aarch64-apple-darwin'
            and receipt['producer_kind'] == 'ios-archive' and receipt['project_sha256'] == project_hash
            and receipt['startup_source_parsing'] is False,
            'Authored iOS producer identity mismatch')
    ios_wasm.program(receipt['program'], module)
    script = receipt['script']
    expected_abi = {'version': 28, 'pointer_bits': 64, 'little_endian': True, 'value_bytes': 16,
                    'value_alignment': 8, 'atom_bytes': 4, 'float_mantissa': 53,
                    'float_max_exponent': 1024, 'write_flags': 17, 'default_c_flags': True}
    require(script['format'] == 3 and script['target'] == target
            and script['producer_target'] == 'aarch64-apple-darwin'
            and script['engine'] == 'rquickjs-0.14.0/quickjs-ng-0.16.2/bytecode-28'
            and script['bytecode_abi'] == expected_abi and script['embedded_source'] is False,
            'Authored iOS script ABI mismatch')
    for field in ('build_id', 'source_sha256', 'bytecode_sha256', 'hardening_sha256'):
        require(isinstance(script[field], str) and re.fullmatch('[0-9a-f]{64}', script[field]),
                f'Invalid script {field}')
    if receipt['compiler'] is not None:
        require(isinstance(script['bindings_sha256'], str)
                and re.fullmatch('[0-9a-f]{64}', script['bindings_sha256'])
                and receipt['compiler']['bundleSha256'] == script['source_sha256']
                and receipt['compiler']['mapSha256'] == script['source_map_sha256'],
                'Authored iOS source graph/binding mismatch')
    else:
        require(script['bindings_sha256'] is None, 'Unexpected empty-scene script binding')


def archive_platform(headers, target):
    require(target in TARGETS, 'Unsupported archive destination')
    expected = 'iossimulator' if target == 'aarch64-apple-ios-sim' else 'ios'
    require(set(re.findall(r'^Arch: (.+)$', headers, re.MULTILINE)) == {'aarch64'},
            'Archive contains a non-arm64 object')
    platforms = set(re.findall(r'^  Platform: (.+)$', headers, re.MULTILINE))
    require(platforms == {expected}, 'Archive object platform differs from its declared destination')
    return {'architecture': 'aarch64', 'platform': expected,
            'object_minimum_versions': sorted(set(re.findall(r'^  Version: (.+)$', headers, re.MULTILINE))),
            'headers_sha256': sha256(headers.encode())}


def package_archive(archive, staging, llvm, execute, target):
    """Preserve object DWARF separately; this is not an application dSYM claim.

    Static archives may retain unused QuickJS parser definitions. Final callers
    must use dead stripping and independently audit the executable symbol set.
    """
    import shutil
    symbols = staging / 'symbols'
    runtime = staging / 'runtime'
    symbols.mkdir()
    runtime.mkdir()
    original = symbols / 'libincant_game_player.unstripped.a'
    shutil.copy2(archive, original)
    copied = runtime / 'libincant_game_player.a'
    before = execute([llvm / 'llvm-objdump', '--section-headers', original]).stdout
    require('__debug_info' in before and '__debug_line' in before, 'Archive is missing retained DWARF')
    execute(['xcrun', 'strip', '-S', '-o', copied, original])
    after = execute([llvm / 'llvm-objdump', '--section-headers', copied]).stdout
    require('__debug_' not in after, 'Debug sections remain in runtime archive')
    names = execute([llvm / 'llvm-nm', '--defined-only', '--demangle', copied]).stdout
    require(all(name in names for name in ('incant_game_run_v1', 'JS_ReadObject', 'JS_EvalFunction')),
            'Missing mobile ABI or bytecode runtime symbols')
    headers = execute([llvm / 'llvm-readobj', '--file-headers', '--macho-version-min', copied]).stdout
    platform = archive_platform(headers, target)
    return {'format': 'object-dwarf-archive', 'platform': platform,
            'unstripped_sha256': sha256(read(original, 512 * 1024**2)),
            'runtime_sha256': sha256(read(copied, 512 * 1024**2)),
            'debug_sections_removed': True, 'before_sections_sha256': sha256(before.encode()),
            'after_sections_sha256': sha256(after.encode()), 'defined_symbols_sha256': sha256(names.encode()),
            'unused_parser_definitions': [name for name in PARSERS if name in names],
            'final_link_requirements': ['dead_strip', 'independent executable symbol audit', 'application dSYM generation']}
