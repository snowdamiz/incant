#!/usr/bin/env python3
"""Fresh authored project -> ARM Android CPU runtime archive + data.

This is an archive exporter, not an app/signing/window host. Emulator execution
and physical-device execution are distinct; this command records compilation only.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import time

import android_archive
import android_wasm
import physics_export
from export_ios import CommandAudit
import native_process
from physics_features import selected_graph_errors, validate_artifacts
from native_package import data_package, publish, read, require, select_camera, sha256, write_json

ROOT = Path(__file__).resolve().parents[1]


def build(project, staging, storage, target, scene, camera, ndk_root, wabt=None):
    audit = CommandAudit(staging / 'audit')
    execute = audit.execute
    rustc = Path(execute(['rustup', 'which', 'rustc']).stdout.strip())
    compiler = execute([rustc, '-vV']).stdout
    match = re.search(r'^host: (.+)$', compiler, re.MULTILINE)
    require(match is not None, 'Missing compiler host identity')
    host = match.group(1)
    android_archive.admit(host, target)
    ndk = android_archive.toolchain(ndk_root)
    ndk_hashes = android_archive.tool_hashes(ndk)
    clang = execute([ndk['clang'], '--version']).stdout
    require('clang version 19.0.1' in clang, 'Unexpected reviewed NDK compiler identity')
    env, selected_env = android_archive.environment(os.environ, rustc, project, scene, ndk)
    cargo = ROOT / 'tools/cargo'
    project_bytes = read(project, 64 * 1024**2)
    mode = physics_export.project_mode(project_bytes)
    module = android_wasm.prepare(project, scene, staging, storage, cargo, env, execute, wabt, ndk)
    capabilities = android_archive.CAPABILITIES + (['static-module'] if module else [])
    selected_features = physics_export.cargo_features(mode, capabilities)
    if module is not None:
        selected_env.update({key: value for key, value in env.items()
                             if key.startswith(('INCANT_NATIVE_WASM_', 'INCANT_WASM_', 'CC_'))})
    features = ','.join(selected_features)
    project_hash = sha256(project_bytes)
    selected_graph = execute([cargo, 'tree', '--locked', '--no-default-features', '-p', android_archive.NAME,
                              '--features', features, '--target', target, '-e', 'normal',
                              '--prefix', 'none', '--format', '{p}|{f}'], env=env).stdout
    graph = android_archive.normal_dependencies(selected_graph)
    errors = selected_graph_errors(selected_graph, mode, target)
    require(not errors, 'Selected Android physics graph differs from project mode: ' + '; '.join(errors))
    (staging / 'normal-dependencies.txt').write_text(selected_graph, encoding='utf-8')
    command = [cargo, 'rustc', '--locked', '--profile', 'shipping', '-j2', '-p', android_archive.NAME,
               '--lib', '--no-default-features', '--features', features, '--crate-type', 'staticlib', '--target', target,
               '--target-dir', storage / 'target', '--config', 'profile.shipping.split-debuginfo="off"',
               '--message-format=json', '-vv']
    command_receipt = {'argv': list(map(str, command)), 'exit_code': None,
                       'env': selected_env}
    if scene is not None:
        command_receipt['env']['INCANT_MOBILE_PROJECT_SCENE'] = scene
    write_json(staging / 'build-command.json', command_receipt)
    started = time.monotonic()
    print(f'Compiling fresh {target} authored shipping archive...', flush=True)
    with (staging / 'build.log').open('w') as stderr, (staging / 'cargo.jsonl').open('w') as stdout:
        result = native_process.run(list(map(str, command)), cwd=ROOT, env=env, stdout=stdout, stderr=stderr, timeout=3600)
    command_receipt.update(exit_code=result.returncode, seconds=time.monotonic() - started,
                           stderr_sha256=sha256(read(staging / 'build.log', 64 * 1024**2)),
                           stdout_sha256=sha256(read(staging / 'cargo.jsonl', 64 * 1024**2)))
    write_json(staging / 'build-command.json', command_receipt)
    log = (staging / 'build.log').read_text()
    require(result.returncode == 0, 'Authored Android build failed; no archive was published:\n' + log[-8000:])
    flags = android_archive.verify_codegen(log, module is not None)
    messages = [json.loads(line) for line in (staging / 'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
    physics = validate_artifacts(messages, mode, target_dir=storage / 'target',
                                                    target=target, profile='shipping')
    artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                 and m['target']['name'] == android_archive.NAME and 'staticlib' in m['target']['crate_types']]
    require(len(artifacts) == 1, 'Missing unique compiled mobile archive')
    artifact = artifacts[0]
    archives = [Path(p) for p in artifact['filenames'] if p.endswith('.a')]
    require(len(archives) == 1, 'Missing actual static archive filename')
    outputs = [Path(m['out_dir']) for m in messages if m.get('reason') == 'build-script-executed'
               and m['package_id'] == artifact['package_id'] and (Path(m['out_dir']) / 'project-export.json').is_file()]
    require(len(outputs) == 1, 'Missing unique authored project producer')
    producer = outputs[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    expected_program = module['program'] if module else None
    android_archive.producer_receipt(receipt, project_hash, mode, expected_program)
    compiled_module = None if module is None else android_wasm.audit_build(
        (staging / 'cargo.jsonl').read_text(), log, target, module, ROOT)
    require(sha256(read(project, 64 * 1024**2)) == project_hash, 'Project changed during archive build')
    camera = select_camera(receipt, camera, False)
    symbols = android_archive.package_archive(archives[0], staging, ndk, execute)
    runtime = staging / 'runtime'
    data = data_package(project.parent, producer, receipt, runtime / 'data', camera)
    header = ROOT / 'crates/incant_game_player/include/incant_game.h'
    shutil.copy2(header, runtime / header.name)
    for name in ('script-symbols.json', 'behavior.js.map'):
        source = producer / name
        if source.is_file():
            shutil.copy2(source, staging / 'symbols' / name)
    write_json(staging / 'project-build.json', receipt)
    require(android_archive.tool_hashes(ndk) == ndk_hashes, 'NDK tools changed during archive build')
    return {'format': 'incant-android-project-archive', 'version': 1, 'target': target, 'producer_target': host,
            'profile': 'shipping', 'features': selected_features, 'command': command_receipt,
            'compiler': compiler, 'ndk': {'revision': android_archive.NDK_REVISION,
                 'path': str(ndk['root']), 'api': android_archive.API, 'clang': clang,
                 'properties_sha256': sha256(read(ndk['root'] / 'source.properties', 65536)),
                 'tool_sha256': ndk_hashes},
            'codegen': flags, 'physics_mode': mode, 'physics_variant': mode,
            'physics': physics,
            'program': expected_program,
            'static_module': None if module is None else {'translation': module['translation'], 'compiled': compiled_module},
            'normal_dependencies': graph, 'project_sha256': project_hash, 'scene_id': receipt['scene_id'],
            'script': receipt['script'], 'camera': camera, 'data': data, 'symbols': symbols,
            'archive': {'path': 'runtime/libincant_game_player.a', 'sha256': symbols['runtime_sha256'],
                        'bytes': (runtime / 'libincant_game_player.a').stat().st_size},
            'header': {'path': 'runtime/incant_game.h', 'sha256': sha256(read(header, 65536))},
            'startup_source_parsing': False, 'signed': False, 'execution': 'not executed; archive compilation only',
            'limits': ['One scene program; mixed Wasm and per-entity TypeScript unsupported',
                       'Static module checkpoints remain unsupported',
                       'CPU package API; no mobile window, touch or device audio host',
                       'final application requires gc-sections and separate executable/symbol audit',
                       'API24 compiler selection is not a minimum-OS execution claim',
                       'no device, performance, signing, store or phase gate']}


def export(project, output, target, ndk, scene=None, camera=None, evidence=None, wabt=None):
    require(target == android_archive.TARGET, 'Unsupported Android destination; only reviewed ARM64 is admitted')
    project = project.resolve(strict=True)
    output = output.absolute()
    require(not output.exists() and not output.is_symlink(), 'Output already exists; choose a new directory')
    require(output.parent.is_dir(), 'Output parent does not exist')
    android_archive.preflight_project(project, scene, wabt)
    if evidence is not None:
        evidence = evidence.absolute()
        require(not evidence.exists() and not evidence.is_symlink() and evidence.parent.is_dir(),
                'Build evidence requires a new directory with an existing parent')
        require(evidence != output and output not in evidence.parents and evidence not in output.parents,
                'Build evidence and archive output must be separate directories')
        evidence.mkdir()
    with tempfile.TemporaryDirectory(prefix='incant-android-export-') as temporary:
        staging = Path(temporary).resolve()
        try:
            with tempfile.TemporaryDirectory(prefix='incant-android-build-') as scratch:
                storage = Path(scratch).resolve()
                report = build(project, staging, storage, target, scene, camera, ndk, wabt)
            require(not storage.exists(), 'Private archive build storage was not removed')
            report['private_build_storage_removed'] = True
            write_json(staging / 'export.json', report)
            publish(staging, output)
        finally:
            # Failed or timed-out builds retain their actual logs on explicit
            # request; no output archive is advertised after producer failure.
            if evidence is not None:
                for name in ('build-command.json', 'build.log', 'cargo.jsonl'):
                    if (staging / name).is_file():
                        shutil.copy2(staging / name, evidence / name)
                if (staging / 'audit').is_dir():
                    shutil.copytree(staging / 'audit', evidence / 'audit')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--target', required=True, choices=[android_archive.TARGET])
    parser.add_argument('--ndk', type=Path, required=True, help='reviewed local NDK installation')
    parser.add_argument('--scene')
    parser.add_argument('--camera')
    parser.add_argument('--evidence', type=Path, help='new directory for raw build logs, including failed builds')
    parser.add_argument('--wabt', type=Path, help='trusted WABT 1.0.42 bin directory for a captured Wasm scene')
    args = parser.parse_args()
    result = export(args.project, args.output, args.target, args.ndk, args.scene, args.camera, args.evidence, args.wabt)
    print(json.dumps({'target': result['target'], 'archive': result['archive'], 'execution': result['execution']}))


if __name__ == '__main__':
    main()
