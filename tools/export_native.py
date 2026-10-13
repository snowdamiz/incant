#!/usr/bin/env python3
"""Build a native desktop player from an authored project with a fresh shipping target.

Usage: python3 tools/export_native.py PROJECT OUTPUT [--scene ULID] [--camera ULID]
       [--mode headless|window] [--ui-viewport WIDTHxHEIGHT]

The trusted engine and validated, metered Wasm translated by pinned wasm2c are
compiled. Project data never selects Cargo configuration or native build scripts. Existing cooked data
caches are required. This host==target exporter is unsigned and does not satisfy
phone, store, performance, full export, or phase gates.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

import native_process
import native_profiling
import physics_export
from physics_features import selected_graph_errors, validate_artifacts

from native_package import data_package, publish, read, require, select_camera, sha256, write_json
from shipping import codegen
from shipping_symbols import backend, package

ROOT = Path(__file__).resolve().parents[1]
NAME = 'incant_game_player'


def execute(command, *, cwd=ROOT, env=None, timeout=120):
    result = native_process.run(list(map(str, command)), cwd=cwd, env=env,
                            text=True, encoding='utf-8', timeout=timeout)
    require(result.returncode == 0, f'{Path(command[0]).name} failed: {result.stderr[-4000:]}')
    return result


def normal_dependencies(graph):
    """Audit the selected player's normal graph, not workspace feature unification."""
    names = []
    forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook',
                 'incant_source', 'incant_play', 'incant_play_legacy', 'incant_wasm_cook', 'incant_wasm_dev', 'loro', 'automerge'}
    for line in graph.splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
        require(match is not None, 'Malformed selected native dependency graph')
        name = match.group(1)
        require(name not in forbidden and not name.startswith(('wasmtime', 'cranelift')),
                f'Authoring/JIT normal dependency: {name}')
        names.append(name)
    require(names and names[0] == NAME, 'Missing selected native player root')
    return sorted(set(names))


def verify_codegen(stderr, host, split):
    flags = {}
    target_lines = [line for line in stderr.splitlines() if 'Running `' in line and f'--target {host}' in line]
    # The player library owns the runtime; the executable is now a thin caller.
    # Audit both target units instead of confusing their shared crate name, or
    # auditing only the executable while allowing a weaker library build.
    units = [('incant_runtime', 'lib', 'incant_runtime'),
             ('incant_script', 'lib', 'incant_script'),
             (NAME, 'lib', NAME + '_library'), (NAME, 'bin', NAME)]
    for name, kind, key in units:
        commands = [line for line in target_lines
                    if re.search(r'--crate-name ' + name + r'\b', line)
                    and re.search(r'--crate-type ' + kind + r'(?:\s|$)', line)]
        require(len(commands) == 1, f'Missing unique fresh target invocation for {name} ({kind})')
        require('feature="compiler"' not in commands[0], 'Runtime source compiler feature leaked')
        require('--test' not in commands[0], 'Test harness cannot verify shipping codegen')
        actual = codegen(commands[0])
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
        if kind == 'bin':
            expected['lto'] = 'fat'
        require(all(actual.get(k) == v for k, v in expected.items()) and actual.get('target-cpu') != 'native',
                f'Incorrect shipping codegen for {name} ({kind}): {actual}')
        flags[key] = expected
    forbidden = r'--crate-name (?:incant_doc|incant_cmd|incant_agent|incant_cook|incant_core|incant_source|incant_play|loro|wasmtime)\b'
    require(not any(re.search(forbidden, line) for line in target_lines), 'Authoring dependency compiled into player target')
    return flags


def cargo_messages(path):
    # Cargo's JSON stream is UTF-8 even on Windows with a legacy ANSI locale.
    return [json.loads(line) for line in path.read_text(encoding='utf-8').splitlines()
            if line.startswith('{')]


def runtime_symbol_audit(executable, kind, llvm_bin, link_map):
    # Linked PE images normally have no COFF symbol table. The fresh final-link
    # map provides defined symbols; matched PE/PDB packaging remains independent.
    if kind == 'pdb':
        payload = read(link_map, 64 * 1024**2)
        symbols = payload.decode('utf-8')
        evidence = {'source': 'fresh MSVC linker map', 'file': link_map.name,
                    'sha256': sha256(payload)}
    else:
        symbols = execute([llvm_bin / 'llvm-nm', '--defined-only', '--demangle', executable]).stdout
        evidence = {'source': 'llvm-nm defined symbols', 'sha256': sha256(symbols.encode('utf-8'))}
    require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Missing bytecode runtime')
    require(not any(name in symbols for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program',
                                                 'js_parse_function_decl', 'incant_script::compiler'))
            # MSVC maps retain Rust mangling rather than llvm-nm's demangled text.
            and not re.search(r'incant_script[0-9]+compiler', symbols), 'Source compiler linked')
    return evidence


def build(project, staging, storage, scene, camera, mode, wabt=None, profiling=None):
    profile_features = native_profiling.features(profiling, mode)
    native_profiling.environment(profiling, os.environ)
    rustc = Path(execute(['rustup', 'which', 'rustc']).stdout.strip())
    compiler = execute([rustc, '-vV']).stdout
    host = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(host)
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    llvm_bin = Path(execute([rustc, '--print', 'sysroot']).stdout.strip()) / 'lib/rustlib' / host / 'bin'
    # Ambient overrides are not an alternate export configuration. Actual target
    # rustc commands are checked below as well as this early refusal.
    for name, value in os.environ.items():
        if (name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')
                or name.startswith('CARGO_PROFILE_') or name.endswith('_RUSTFLAGS')):
            require(not value, f'Unset {name} for the audited shipping exporter')
    env = {k: v for k, v in os.environ.items() if not k.startswith(('INCANT_NATIVE_SCRIPT_', 'INCANT_NATIVE_PROJECT', 'INCANT_NATIVE_WASM_', 'INCANT_MOBILE_', 'INCANT_WASM_BUNDLE', 'INCANT_AUTHORED_WASM'))}
    env.update(PATH=str(rustc.parent) + os.pathsep + env.get('PATH', ''), INCANT_NATIVE_PROJECT=str(project))
    if scene is not None:
        env['INCANT_NATIVE_PROJECT_SCENE'] = scene
    cargo = ROOT / 'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
    project_bytes = read(project, 64 * 1024**2)
    project_hash = sha256(project_bytes)
    physics_mode = physics_export.project_mode(project_bytes)
    features = ','.join(physics_export.cargo_features(physics_mode,
                       ['project', 'native-window' if mode == 'window' else 'render-input'] + profile_features))
    from native_wasm import capture
    module = capture(project, scene, storage, cargo, env, execute, wabt)
    if module is not None:
        features += ',static-module'
    selected_graph = execute([cargo, 'tree', '--locked', '--no-default-features', '-p', NAME,
                              '--features', features, '--target', host, '-e', 'normal',
                              '--prefix', 'none', '--format', '{p}|{f}'], env=env).stdout
    graph = normal_dependencies(selected_graph)
    profile_graph = native_profiling.graph(selected_graph, profiling)
    errors = selected_graph_errors(selected_graph, physics_mode, host)
    require(not errors, 'Selected physics graph differs from project mode: ' + '; '.join(errors))
    (staging / 'normal-dependencies.txt').write_text(selected_graph, encoding='utf-8')
    link_map = staging / 'player-link.map'
    require(not link_map.exists(), 'Linker map must not preexist')
    command = [cargo, 'rustc' if kind == 'pdb' else 'build', '-j2', '-p', NAME, '--bin', NAME,
               '--no-default-features', '--features', features, '--profile', 'shipping', '--locked',
               '--target', host, '--target-dir', storage / 'target', '--config', f'profile.shipping.split-debuginfo="{split}"',
               '--message-format=json', '-vv']
    if kind == 'pdb':
        command += ['--', '-C', f'link-arg=/MAP:{link_map}']
    print(f'Compiling fresh {host} shipping player ({mode})...', flush=True)
    with (staging / 'build.log').open('w', encoding='utf-8') as stderr, (storage / 'cargo.jsonl').open('w', encoding='utf-8') as stdout:
        result = native_process.run(list(map(str, command)), cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=2400)
    log = (staging / 'build.log').read_text(encoding='utf-8')
    require(result.returncode == 0, 'Native player build failed; no package was published:\n' + log[-8000:])
    flags = verify_codegen(log, host, split)
    messages = cargo_messages(storage / 'cargo.jsonl')
    profile_units = native_profiling.artifacts(messages, profiling, target_dir=storage / 'target',
                                              target=host, profile='shipping')
    physics = validate_artifacts(messages, physics_mode, target_dir=storage / 'target',
                                 target=host, profile='shipping')
    artifacts = [a for a in messages if a.get('reason') == 'compiler-artifact' and a['target']['name'] == NAME and a.get('executable')]
    require(len(artifacts) == 1, 'Missing unique actual player artifact')
    artifact = artifacts[0]
    executable = Path(artifact['executable'])
    outputs = [Path(a['out_dir']) for a in messages if a.get('reason') == 'build-script-executed'
               and a['package_id'] == artifact['package_id'] and (Path(a['out_dir']) / 'project-export.json').is_file()]
    require(len(outputs) == 1, 'Missing unique project cook output')
    producer = outputs[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    require(receipt['project_sha256'] == project_hash == sha256(read(project, 64 * 1024**2)), 'Project changed during build')
    require(receipt['target'] == host and receipt['startup_source_parsing'] is False, 'Project producer target/mode mismatch')
    if module is not None:
        require(all(receipt['program'].get(k) == v for k, v in module.items()), 'Wasm program changed during export')
    physics_export.verify_producer(receipt, physics_mode)
    chosen_camera = select_camera(receipt, camera, mode == 'window')
    symbol_audit = runtime_symbol_audit(executable, kind, llvm_bin, link_map)
    player, identity = package(executable, staging, kind, llvm_bin)
    data = data_package(project.parent, producer, receipt, player.parent / 'data', chosen_camera)
    # Source maps are developer crash artifacts, never part of the runtime tree.
    for name in ('script-symbols.json', 'behavior.js.map'):
        source = producer / name
        if source.exists():
            shutil.copy2(source, staging / 'symbols' / name)
    write_json(staging / 'project-build.json', receipt)
    report = {'format': 'incant-native-export', 'version': 1, 'target': host, 'mode': mode,
                    'project_sha256': project_hash, 'scene_id': receipt['scene_id'], 'camera': chosen_camera,
                    'physics_mode': physics_mode, 'physics_variant': physics_mode, 'physics': physics,
                    'program': receipt.get('program'), 'compiler': compiler, 'codegen': flags, 'normal_dependencies': graph, 'symbols': identity,
                    'symbol_audit': symbol_audit,
                    'player_sha256': sha256(read(player, 256 * 1024**2)), 'data': data,
                    'startup_source_parsing': False, 'signed': False,
                    'limits': ['host==target desktop only', 'existing cooked caches required',
                               'window export GPU execution requires a desktop', 'no phone or performance gate']}
    if profiling is not None:
        report['profiling'] = {'mode': profiling, 'normal_graph': profile_graph, 'compiler': profile_units,
                               'startup_capture': False, 'runtime_opt_in': '--profile-tracy' if profiling == 'tracy' else '--profile',
                               'transport': 'explicit loopback process lifetime' if profiling == 'tracy' else 'none'}
    return player, report


def ui_startup_options(mode, viewport):
    if viewport is None:
        return []
    require(mode == 'headless', 'Screen-space UI drawing in native windows is not implemented')
    require(isinstance(viewport, str) and re.fullmatch(r'[1-9][0-9]{0,4}x[1-9][0-9]{0,4}', viewport),
            'UI viewport must be WIDTHxHEIGHT logical pixels')
    require(all(int(n) <= 65536 for n in viewport.split('x')), 'UI viewport exceeds bounds')
    return ['--ui-viewport', viewport]


def export(project, output, scene=None, camera=None, mode='headless', wabt=None, ui_viewport=None, profiling=None):
    require(mode in ('headless', 'window'), 'Unknown native export mode')
    native_profiling.features(profiling, mode)
    native_profiling.environment(profiling, os.environ)
    ui_options = ui_startup_options(mode, ui_viewport)
    project = project.resolve(strict=True)
    output = output.absolute()
    require(not output.exists() and not output.is_symlink(), 'Output already exists; choose a new directory')
    require(output.parent.is_dir(), 'Output parent does not exist')
    # Private staging is outside project/build output. Failed publication can
    # leave an incomplete new output directory; export.json is published last.
    # Never overwrite or recursively delete a pre-existing user directory.
    with tempfile.TemporaryDirectory(prefix='incant-native-export-') as temporary:
        staging = Path(temporary).resolve()
        with tempfile.TemporaryDirectory(prefix='incant-project-build-') as scratch:
            storage = Path(scratch).resolve()
            player, report = build(project, staging, storage, scene, camera, mode, wabt, profiling)
        require(not storage.exists(), 'Private build storage was not removed')
        print('Checking copied player after removal of private build storage...', flush=True)
        runtime_env = {'PATH': str(player.parent),
                       'TMPDIR': str(staging), 'TEMP': str(staging), 'TMP': str(staging)}
        runtime_env.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})
        result = execute([player, '--package', player.parent / 'data', '--ticks', '1', '--output', staging / 'startup.pcm'] + ui_options,
                         cwd=player.parent, env=runtime_env, timeout=60)
        report['startup'] = json.loads(result.stdout)
        report['build_storage_removed_before_startup'] = True
        report['startup_backend'] = 'CPU render input; no window or GPU device'
        if ui_viewport is not None:
            report['startup_ui_viewport'] = ui_viewport
        (staging / 'startup.pcm').unlink()
        # Keep detailed compiler diagnostics beside evidence, outside player.
        write_json(staging / 'export.json', report)
        publish(staging, output)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--wabt', type=Path, help='Trusted pinned WABT 1.0.42 directory for authored Wasm export')
    parser.add_argument('--profiling', choices=('summary', 'tracy'), help='Trusted opt-in offline diagnostics build; runtime capture remains explicit')
    parser.add_argument('--scene')
    parser.add_argument('--camera')
    parser.add_argument('--mode', choices=('headless', 'window'), default='headless')
    parser.add_argument('--ui-viewport', help='Explicit CPU UI startup viewport in logical pixels; no GPU UI drawing')
    args = parser.parse_args()
    try:
        result = export(args.project, args.output, args.scene, args.camera, args.mode,
                        wabt=args.wabt, ui_viewport=args.ui_viewport, profiling=args.profiling)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Native export failed: {error}\n')
    print(json.dumps({'output': str(args.output.absolute()), 'target': result['target'], 'player_sha256': result['player_sha256']}))


if __name__ == '__main__':
    main()
