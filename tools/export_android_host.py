#!/usr/bin/env python3
"""Fresh authored TypeScript project -> unsigned API26 Vulkan/AAudio APK.

Trusted build tooling only. No emulator, signing or physical execution occurs.
The existing API24 CPU/static-Wasm archive contract is not modified.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import zipfile

import android_archive
import android_host
import physics_export
from export_ios import CommandAudit
from native_package import data_package, publish, read, require, select_camera, sha256, write_json
from physics_features import selected_graph_errors, validate_artifacts

ROOT = Path(__file__).resolve().parents[1]


def build(project, staging, storage, ndk_root, sdk, java, scene, camera):
    execute = CommandAudit(staging / 'audit').execute
    target = android_archive.TARGET
    rustc = Path(execute(['rustup', 'which', 'rustc']).stdout.strip())
    compiler = execute([rustc, '-vV']).stdout
    host = re.search(r'^host: (.+)$', compiler, re.MULTILINE)
    require(host is not None, 'Compiler host missing')
    android_archive.admit(host[1], target)
    ndk = android_host.toolchain(ndk_root)
    tools = android_host.java_tools(sdk, java)
    ndk_hashes = android_archive.tool_hashes(ndk)
    sdk_hashes = {k: sha256(p.read_bytes()) for k, p in tools.items()}
    macros = android_host.compiler(ndk, execute)
    env, selected = android_host.environment(os.environ, rustc, project, scene, ndk)
    env['JAVA_HOME'] = str(java)
    env['PATH'] = str(java / 'bin') + os.pathsep + env['PATH']
    mode = physics_export.project_mode(read(project, 64 * 1024**2))
    project_hash = sha256(read(project, 64 * 1024**2))
    features = physics_export.cargo_features(mode, android_host.FEATURES)
    cargo = ROOT / 'tools/cargo'
    args = ['--locked', '--no-default-features', '-p', android_archive.NAME,
            '--features', ','.join(features), '--target', target]
    graph = execute([cargo, 'tree', *args, '-e', 'normal', '--prefix', 'none', '--format', '{p}|{f}'], env=env).stdout
    dependencies = android_host.graph(graph)
    errors = selected_graph_errors(graph, mode, target)
    require(not errors, 'Android host physics graph: ' + '; '.join(errors))
    (staging / 'normal-dependencies.txt').write_text(graph)
    # Fresh target makes codegen/producer audits meaningful. No host-provided
    # code, compiler path or flags are read from the project document.
    result = execute([cargo, 'rustc', *args, '--profile', 'shipping', '-j2', '--lib',
                      '--crate-type', 'staticlib', '--target-dir', storage / 'target',
                      '--config', 'profile.shipping.split-debuginfo="off"', '--message-format=json', '-vv'],
                     env=env, timeout=3600)
    codegen = android_archive.verify_codegen(result.stderr)
    messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    physics = validate_artifacts(messages, mode, target_dir=storage / 'target', target=target, profile='shipping')
    artifacts = [m for m in messages if m.get('reason') == 'compiler-artifact'
                 and m['target']['name'] == android_archive.NAME and 'staticlib' in m['target']['crate_types']]
    require(len(artifacts) == 1, 'Missing unique Android host archive')
    artifact = artifacts[0]
    archives = [Path(p) for p in artifact['filenames'] if p.endswith('.a')]
    require(len(archives) == 1, 'Missing static archive')
    outputs = [Path(m['out_dir']) for m in messages if m.get('reason') == 'build-script-executed'
               and m['package_id'] == artifact['package_id'] and (Path(m['out_dir']) / 'project-export.json').is_file()]
    require(len(outputs) == 1, 'Missing unique authored producer')
    producer = outputs[0]
    receipt = json.loads(read(producer / 'project-export.json', 4 * 1024**2))
    android_archive.producer_receipt(receipt, project_hash, mode)
    require(sha256(read(project, 64 * 1024**2)) == project_hash, 'Project changed during build')
    camera = select_camera(receipt, camera, True)
    assets = storage / 'assets/incant'
    assets.mkdir(parents=True)
    data = data_package(project.parent, producer, receipt, assets / 'data', camera)
    rows = android_host.index_files(assets / 'data')
    (assets / 'index.txt').write_text(''.join(f'{sha256(b)} {len(b)} {n}\n' for n, b in rows), encoding='ascii')
    symbols = staging / 'symbols'
    symbols.mkdir()
    library = symbols / 'libincant_game_player.so'
    exports = storage / 'exports.map'
    exports.write_text('{ global: ' + '; '.join(android_host.JNI) + '; local: *; };\n')
    execute([ndk['clang'], '-shared', archives[0],
             *['-Wl,-u,' + name for name in android_host.JNI],
             '-Wl,--gc-sections', '-Wl,--no-undefined', '-Wl,-z,max-page-size=16384',
             '-Wl,--version-script=' + str(exports), '-Wl,-soname,libincant_game_player.so',
             '-landroid', '-laaudio', '-llog', '-ldl', '-lm', '-lc++_static', '-lc++abi', '-o', library], timeout=300)
    names = execute([ndk['nm'], '--defined-only', '--demangle', library]).stdout
    require(all(name in names for name in android_host.JNI), 'Missing JNI entrypoints')
    require(not any(name in names for name in android_archive.PARSERS), 'Runtime source parser retained')
    headers = execute([ndk['readobj'], '--file-headers', library]).stdout
    android_archive.archive_platform(headers)
    runtime = storage / 'libincant_game_player.so'
    shutil.copy2(library, runtime)
    execute([ndk['strip'], '--strip-debug', runtime])
    classes, dex = storage / 'classes', storage / 'dex'
    classes.mkdir(); dex.mkdir()
    sources = sorted((ROOT / 'platforms/android-player/src').rglob('*.java'))
    execute([tools['javac'], '-Xlint:all', '-Werror', '--release', '17', '-classpath', tools['android'],
             '-d', classes, *sources], env=env)
    execute([tools['d8'], '--release', '--min-api', '26', '--lib', tools['android'], '--output', dex,
             *sorted(classes.rglob('*.class'))], env=env)
    apk = storage / 'unaligned.apk'
    execute([tools['aapt2'], 'link', '-I', tools['android'], '--manifest', ROOT / 'platforms/android-player/AndroidManifest.xml',
             '--min-sdk-version', '26', '--target-sdk-version', '36', '-A', storage / 'assets', '-o', apk], env=env)
    with zipfile.ZipFile(apk, 'a', compression=zipfile.ZIP_STORED) as output:
        output.write(runtime, 'lib/arm64-v8a/libincant_game_player.so')
        output.write(dex / 'classes.dex', 'classes.dex')
    final = staging / 'IncantPlayer-unsigned.apk'
    execute([tools['zipalign'], '-P', '16', '-f', '4', apk, final], env=env)
    execute([tools['zipalign'], '-c', '-P', '16', '4', final], env=env)
    require(android_archive.tool_hashes(ndk) == ndk_hashes, 'NDK changed during build')
    require({k: sha256(p.read_bytes()) for k,p in tools.items()} == sdk_hashes, 'SDK changed during build')
    write_json(staging / 'project-build.json', receipt)
    return {'format':'incant-android-device-host', 'version':1, 'minimum_api':26, 'target':target,
            'features':features, 'physics':physics, 'codegen':codegen, 'normal_dependencies':dependencies,
            'project_sha256':project_hash, 'data':data, 'compiler':compiler, 'environment':selected,
            'compiler_macros_sha256':sha256(macros.encode()), 'ndk_tools':ndk_hashes, 'sdk_tools':sdk_hashes,
            'unstripped_library_sha256':sha256(read(library, 512*1024**2)),
            'runtime_library_sha256':sha256(read(runtime, 512*1024**2)),
            'apk_sha256':sha256(read(final, 1024**3)), 'signed':False, 'executed':False,
            'limits':['API26 AAudio and Vulkan required; no silent offline fallback',
                      'TypeScript only; static Wasm remains on separate API24 archive route',
                      'Touch only; keyboard/IME/gamepad/haptics adaptation not supplied',
                      'Activity recreation starts a fresh game; surface replacement preserves it',
                      'No device execution, physical audio, appearance or performance claim']}


def export(project, output, ndk, sdk, java, scene=None, camera=None, evidence=None):
    project = project.resolve(strict=True)
    output = output.absolute()
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(), 'Choose a new output')
    android_host.preflight(project, scene)
    if evidence is not None:
        evidence = evidence.absolute()
        require(not evidence.exists() and not evidence.is_symlink() and evidence.parent.is_dir(), 'Choose new evidence directory')
        require(evidence != output and output not in evidence.parents and evidence not in output.parents, 'Evidence/output overlap')
        evidence.mkdir()
    with tempfile.TemporaryDirectory(prefix='incant-android-host-') as temporary:
        staging = Path(temporary) / 'staging'; staging.mkdir()
        storage = Path(temporary) / 'build'; storage.mkdir()
        try:
            manifest = build(project, staging, storage, ndk.resolve(strict=True), sdk.resolve(strict=True), java.resolve(strict=True), scene, camera)
            write_json(staging / 'export.json', manifest)
            publish(staging, output)
        finally:
            if evidence is not None:
                shutil.copytree(staging, evidence, dirs_exist_ok=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path); parser.add_argument('output', type=Path)
    parser.add_argument('--ndk', type=Path, required=True); parser.add_argument('--sdk', type=Path, required=True)
    parser.add_argument('--java-home', type=Path, required=True); parser.add_argument('--scene'); parser.add_argument('--camera')
    parser.add_argument('--evidence', type=Path)
    args = parser.parse_args()
    export(args.project, args.output, args.ndk, args.sdk, args.java_home, args.scene, args.camera, args.evidence)
