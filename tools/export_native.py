#!/usr/bin/env python3
"""Build a native desktop player from an authored project with a fresh shipping target.

Usage: python3 tools/export_native.py PROJECT OUTPUT [--scene ULID] [--camera ULID]
       [--mode headless|window]

Only the trusted engine is compiled. Project JSON/TypeScript is bounded compiler
input, never Cargo configuration or a native build script. Existing cooked data
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


def normal_dependencies(metadata):
    names = {p['id']: p['name'] for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    pending = [ident for ident, name in names.items() if name == NAME]
    require(len(pending) == 1, 'Missing unique native player')
    seen = set()
    forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook',
                 'incant_source', 'incant_play_legacy', 'loro', 'automerge'}
    while pending:
        ident = pending.pop()
        if ident in seen:
            continue
        seen.add(ident)
        require(names[ident] not in forbidden and not names[ident].startswith(('wasmtime', 'cranelift')),
                f'Authoring/JIT normal dependency: {names[ident]}')
        pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] is None for k in d['dep_kinds']))
    return sorted(names[ident] for ident in seen)


def verify_codegen(stderr, host, split):
    flags = {}
    target_lines = [line for line in stderr.splitlines() if 'Running `' in line and f'--target {host}' in line]
    for name in ('incant_runtime', 'incant_script', NAME):
        commands = [line for line in target_lines if re.search(r'--crate-name ' + name + r'\b', line)]
        require(len(commands) == 1, f'Missing fresh target invocation for {name}')
        require('feature="compiler"' not in commands[0], 'Runtime source compiler feature leaked')
        actual = codegen(commands[0])
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': split}
        if name == NAME:
            expected['lto'] = 'fat'
        require(all(actual.get(k) == v for k, v in expected.items()) and actual.get('target-cpu') != 'native',
                f'Incorrect shipping codegen for {name}: {actual}')
        flags[name] = expected
    forbidden = r'--crate-name (?:incant_doc|incant_cmd|incant_agent|incant_cook|incant_core|incant_source|loro|wasmtime)\b'
    require(not any(re.search(forbidden, line) for line in target_lines), 'Authoring dependency compiled into player target')
    return flags


def build(project, staging, storage, scene, camera, mode):
    rustc = Path(execute(['rustup', 'which', 'rustc']).stdout.strip())
    compiler = execute([rustc, '-vV']).stdout
    host = re.search(r'^host: (.+)$', compiler, re.MULTILINE).group(1)
    kind = backend(host)
    split = 'packed' if kind in ('dsym', 'pdb') else 'off'
    suffix = '.exe' if os.name == 'nt' else ''
    llvm_bin = Path(execute([rustc, '--print', 'sysroot']).stdout.strip()) / 'lib/rustlib' / host / 'bin'
    # Ambient overrides are not an alternate export configuration. Actual target
    # rustc commands are checked below as well as this early refusal.
    for name, value in os.environ.items():
        if (name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')
                or name.startswith('CARGO_PROFILE_') or name.endswith('_RUSTFLAGS')):
            require(not value, f'Unset {name} for the audited shipping exporter')
    env = {k: v for k, v in os.environ.items() if not k.startswith(('INCANT_NATIVE_SCRIPT_', 'INCANT_NATIVE_PROJECT'))}
    env.update(PATH=str(rustc.parent) + os.pathsep + env.get('PATH', ''), INCANT_NATIVE_PROJECT=str(project))
    if scene is not None:
        env['INCANT_NATIVE_PROJECT_SCENE'] = scene
    cargo = ROOT / 'tools/cargo' if os.name != 'nt' else rustc.with_name('cargo.exe')
    features = 'project,' + ('native-window' if mode == 'window' else 'render-input')
    project_hash = sha256(read(project, 64 * 1024**2))
    metadata = json.loads(execute([cargo, 'metadata', '--locked', '--format-version=1', '--filter-platform', host,
                                   '--features', ','.join(NAME + '/' + f for f in features.split(','))], env=env).stdout)
    graph = normal_dependencies(metadata)
    command = [cargo, 'build', '-j2', '-p', NAME, '--features', features, '--profile', 'shipping', '--locked',
               '--target', host, '--target-dir', storage / 'target', '--config', f'profile.shipping.split-debuginfo="{split}"',
               '--message-format=json', '-vv']
    print(f'Compiling fresh {host} shipping player ({mode})...', flush=True)
    with (staging / 'build.log').open('w', encoding='utf-8') as stderr, (storage / 'cargo.jsonl').open('w', encoding='utf-8') as stdout:
        result = native_process.run(list(map(str, command)), cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=2400)
    log = (staging / 'build.log').read_text(encoding='utf-8')
    require(result.returncode == 0, 'Native player build failed; no package was published:\n' + log[-8000:])
    flags = verify_codegen(log, host, split)
    messages = [json.loads(line) for line in (storage / 'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
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
    chosen_camera = select_camera(receipt, camera, mode == 'window')
    symbols = execute([llvm_bin / ('llvm-nm' + suffix), '--defined-only', '--demangle', executable]).stdout
    require('JS_ReadObject' in symbols and 'JS_EvalFunction' in symbols, 'Missing bytecode runtime')
    require(not any(name in symbols for name in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program',
                                                 'js_parse_function_decl', 'incant_script::compiler')), 'Source compiler linked')
    player, identity = package(executable, staging, kind, llvm_bin)
    data = data_package(project.parent, producer, receipt, player.parent / 'data', chosen_camera)
    # Source maps are developer crash artifacts, never part of the runtime tree.
    for name in ('script-symbols.json', 'behavior.js.map'):
        source = producer / name
        if source.exists():
            shutil.copy2(source, staging / 'symbols' / name)
    write_json(staging / 'project-build.json', receipt)
    return player, {'format': 'incant-native-export', 'version': 1, 'target': host, 'mode': mode,
                    'project_sha256': project_hash, 'scene_id': receipt['scene_id'], 'camera': chosen_camera,
                    'compiler': compiler, 'codegen': flags, 'normal_dependencies': graph, 'symbols': identity,
                    'player_sha256': sha256(read(player, 256 * 1024**2)), 'data': data,
                    'startup_source_parsing': False, 'signed': False,
                    'limits': ['host==target desktop only', 'existing cooked caches required',
                               'window export GPU execution requires a desktop', 'no phone or performance gate']}


def export(project, output, scene=None, camera=None, mode='headless'):
    require(mode in ('headless', 'window'), 'Unknown native export mode')
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
            player, report = build(project, staging, storage, scene, camera, mode)
        require(not storage.exists(), 'Private build storage was not removed')
        print('Checking copied player after removal of private build storage...', flush=True)
        runtime_env = {'PATH': str(player.parent), 'HOME': str(staging / 'absent-home'),
                       'TMPDIR': str(staging), 'TEMP': str(staging), 'TMP': str(staging)}
        runtime_env.update({key: os.environ[key] for key in ('SystemRoot', 'SystemDrive', 'WINDIR') if key in os.environ})
        result = execute([player, '--package', player.parent / 'data', '--ticks', '1', '--output', staging / 'startup.pcm'],
                         cwd=player.parent, env=runtime_env, timeout=60)
        report['startup'] = json.loads(result.stdout)
        report['build_storage_removed_before_startup'] = True
        report['startup_backend'] = 'CPU render input; no window or GPU device'
        (staging / 'startup.pcm').unlink()
        # Keep detailed compiler diagnostics beside evidence, outside player.
        write_json(staging / 'export.json', report)
        publish(staging, output)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--scene')
    parser.add_argument('--camera')
    parser.add_argument('--mode', choices=('headless', 'window'), default='headless')
    args = parser.parse_args()
    try:
        result = export(args.project, args.output, args.scene, args.camera, args.mode)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f'Native export failed: {error}\n')
    print(json.dumps({'output': str(args.output.absolute()), 'target': result['target'], 'player_sha256': result['player_sha256']}))


if __name__ == '__main__':
    main()
