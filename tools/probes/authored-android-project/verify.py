#!/usr/bin/env python3
"""Run a fresh authored Android archive through source-free, owned emulator C calls.

The accepted candidate transport/lifecycle is reused; authored receipt admission,
full per-tick oracle and shipping artifact checks are independent of that probe.
"""
import argparse
import json
import hashlib
import os
from pathlib import Path
import runpy
import shutil
import socket
import subprocess
import sys
import time
import uuid

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import android_archive
import physics_export
from native_package import read, require

transport = runpy.run_path(str(ROOT / 'tools/probes/android-bytecode/verify.py'))
prepare = runpy.run_path(str(HERE / 'prepare.py'))
digest, files, write = prepare['digest'], prepare['files'], prepare['write']
AndroidProof = transport['AndroidProof']
stop_owned_emulator, avd_command = transport['stop_owned_emulator'], transport['avd_command']

def control(proof, name, prefix, request, status, *, error=None, oracle=None, native=None, pcm=None):
    path = proof.output / (name + '-request.json')
    write(path, request)
    response = json.loads(proof.command([*prefix, path], status))
    assert response == {'abi_status': status}
    output = Path(request['output_directory'])
    report = None
    if status == 2:
        assert not output.exists()
    elif status != 3:
        report = json.loads((output / 'result.json').read_text())
        assert report['format'] == 'incant-mobile-runtime' and report['version'] == 1
        assert report['passed'] is (status == 0) and report['error'] == error
        assert report['os'] == 'android' and report['architecture'] == 'aarch64'
        assert report['panic'] == 'abort' and report['debug_assertions'] is False
        assert report['execution'] == 'offline_cpu'
        if status == 0:
            game = report['game']
            assert game['logs'] == oracle['logs'] == native['logs']
            for key in ('tick', 'frames', 'pcm_sha256', 'observations'):
                assert game[key] == native[key], (name, key)
            for entity, x in oracle['positions'].items():
                assert game['observations']['entities'][entity]['translation'] == [x, 0, 0]
            assert (output / 'audio.pcm').read_bytes() == pcm
        else:
            assert report['game'] is None and not (output / 'audio.pcm').exists()
    proof.controls.append({'name': name, 'expected_status': status, 'request': str(path),
                           'request_sha256': digest(path), 'report': report})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prepared', type=Path, required=True)
    parser.add_argument('--export', dest='exported', type=Path, required=True)
    parser.add_argument('--export-sha256', required=True)
    parser.add_argument('--sdk', type=Path, required=True)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--ndk', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--remove-authoring', action='store_true')
    parser.add_argument('--port', type=int, default=5584)
    args = parser.parse_args()
    prepared, exported = args.prepared.resolve(strict=True), args.exported.resolve(strict=True)
    require(digest(exported / 'export.json') == args.export_sha256, 'Archive export checksum mismatch')
    receipt = json.loads(read(exported / 'export.json', 4 * 1024**2))
    require(receipt['format'] == 'incant-android-project-archive' and receipt['version'] == 1,
            'Expected authored Android archive receipt')
    mode = receipt['physics_mode']
    require(receipt['target'] == android_archive.TARGET and receipt['profile'] == 'shipping'
            and receipt['features'] == physics_export.cargo_features(mode, android_archive.CAPABILITIES)
            and receipt['command']['exit_code'] == 0 and receipt['private_build_storage_removed'] is True,
            'Expected matching successful shipping archive and removed private build storage')
    produced = json.loads(read(exported / 'project-build.json', 4 * 1024**2))
    android_archive.producer_receipt(produced, receipt['project_sha256'], mode)
    require(produced['script'] == receipt['script'] and produced['scene_id'] == receipt['scene_id'], 'Producer mismatch')
    declared = json.loads((prepared / 'result.json').read_text())
    require(declared['passed'] and declared['scene'] == receipt['scene_id']
            and declared['physics_mode'] == mode, 'Native preflight mismatch')
    for kind, exact in [('archive', 'runtime/libincant_game_player.a'), ('header', 'runtime/incant_game.h')]:
        require(receipt[kind]['path'] == exact and digest(exported / exact) == receipt[kind]['sha256'],
                kind + ' identity mismatch')
    require(files(exported / 'runtime/data') == receipt['data']['files'], 'Data package differs from receipt')
    ndk = android_archive.toolchain(args.ndk)
    require(str(ndk['root']) == receipt['ndk']['path'], 'Linker NDK differs from archive producer')
    for name, checksum in receipt['ndk']['tool_sha256'].items():
        require(digest(ndk[name]) == checksum, 'Linker tool changed: ' + name)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    authoring = prepared / 'authoring'
    deletion = {'authoring': str(authoring), 'maps': {}}
    if args.remove_authoring:
        require(Path(declared['project']) == authoring / 'game.json' and files(authoring) == declared['authoring_unchanged'],
                'Private authoring changed; refusing cleanup')
        require(digest(authoring / 'game.json') == receipt['project_sha256'], 'Project identity differs')
        deletion['authoring_hashes'] = files(authoring)
        shutil.rmtree(authoring)
        write(prepared / 'source-removal.json', deletion)
    require(not authoring.exists(), 'Remove private prepared source/cache before this proof')
    map_path = exported / 'symbols/behavior.js.map'
    if map_path.exists():
        deletion['maps'][str(map_path)] = digest(map_path)
        map_path.unlink()
    write(output / 'source-removal.json', deletion)
    before = files(exported / 'runtime')
    shutil.copytree(exported / 'runtime/data', output / 'data')
    inputs = output / 'inputs'
    inputs.mkdir()
    for path in prepared.iterdir():
        if path.is_file() and (path.name.endswith(('-input.json', '-observe.json', '-expected.json', '-native.json', '.pcm'))):
            shutil.copy2(path, inputs / path.name)
    assert args.port % 2 == 0 and 5554 <= args.port <= 5682
    for port in (args.port, args.port + 1):
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', port))
    sdk, java = args.sdk.resolve(strict=True), args.java.resolve(strict=True)
    avd_name = 'incant-authored-android-' + uuid.uuid4().hex
    remote = '/data/local/tmp/' + avd_name
    avd_home = output / 'avd'
    avd_home.mkdir()
    env = dict(os.environ, JAVA_HOME=str(java), ANDROID_AVD_HOME=str(avd_home), ANDROID_HOME=str(sdk))
    proof = AndroidProof(output, sdk / 'platform-tools/adb', f'emulator-{args.port}', remote)
    report = {'passed': False, 'execution': 'local ARM Android emulator CPU', 'physical_device': False,
              'export': str(exported), 'export_sha256': args.export_sha256, 'physics_mode': mode,
              'prepared': str(prepared), 'serial': proof.serial, 'avd': avd_name, 'remote_directory': remote,
              'source_removal': deletion, 'runtime_before': before, 'inputs': files(inputs),
              'system_image': 'system-images;android-37.0;google_apis;arm64-v8a',
              'sources': {str(p.relative_to(ROOT)): digest(p) for p in [*HERE.glob('*.py'), *HERE.glob('*.ts'),
                  ROOT / 'tools/probes/android-bytecode/verify.py', ROOT / 'tools/probes/mobile-runtime-library/caller.c']}}
    manager = sdk / 'cmdline-tools/latest/bin/avdmanager'
    process = None
    created = False
    try:
        caller = output / 'caller-c'
        proof.command([ndk['clang'], '-std=c11', '-Wall', '-Wextra', '-Werror', '-I', exported / 'runtime',
                       ROOT / 'tools/probes/mobile-runtime-library/caller.c', exported / receipt['archive']['path'],
                       '-Wl,--gc-sections', '-ldl', '-llog', '-lm', '-lc++_static', '-lc++abi', '-o', caller])
        symbols = proof.command([ndk['nm'], '--defined-only', '--demangle', caller])
        require(all(name in symbols for name in ('incant_game_run_v1', 'JS_ReadObject', 'JS_EvalFunction')),
                'Final caller lacks mobile or bytecode runtime symbols')
        require(not any(name in symbols for name in android_archive.PARSERS), 'Final caller retains source compiler')
        sections = proof.command([ndk['objdump'], '--section-headers', caller])
        require('.debug_' not in sections, 'Final caller retains debug sections')
        headers = proof.command([ndk['readobj'], '--file-headers', caller])
        report['caller_platform'] = android_archive.archive_platform(headers)
        dynamic = proof.command([ndk['readelf'], '-d', caller])
        report['caller'] = {'path': str(caller), 'sha256': digest(caller), 'bytes': caller.stat().st_size,
                            'symbols_sha256': hashlib.sha256(symbols.encode()).hexdigest(),
                            'dynamic_sha256': hashlib.sha256(dynamic.encode()).hexdigest()}
        create = [str(manager), 'create', 'avd', '--name', avd_name, '--path', str(avd_home / (avd_name + '.avd')),
                  '--package', report['system_image']]
        created = True  # Also clean up a partially created, uniquely named AVD.
        report['create'] = avd_command(output, 'avd-create', create, env, b'no\n')
        argv = [str(sdk / 'emulator/emulator'), '-avd', avd_name, '-port', str(args.port), '-no-window',
                '-no-audio', '-no-boot-anim', '-no-snapshot', '-wipe-data', '-gpu', 'software',
                '-accel', 'on', '-camera-back', 'none', '-camera-front', 'none']
        with (output / 'emulator.log').open('wb') as log:
            process = subprocess.Popen(argv, env=env, stdout=log, stderr=subprocess.STDOUT)
        report['emulator'] = {'argv': argv, 'pid': process.pid}
        deadline = time.monotonic() + 240
        while time.monotonic() < deadline:
            assert process.poll() is None, 'Owned emulator exited before boot'
            try:
                boot = proof.adb_command('shell', 'getprop', 'sys.boot_completed', expected=(0, 1), timeout=10)
                if boot.strip() == '1':
                    break
            except subprocess.TimeoutExpired:
                pass  # The timeout receipt is retained; the overall deadline still applies.
            time.sleep(2)
        else:
            raise TimeoutError('Owned emulator boot deadline exceeded')
        assert proof.adb_command('emu', 'avd', 'name').splitlines()[0] == avd_name
        abi = proof.adb_command('shell', 'getprop', 'ro.product.cpu.abi').strip()
        assert abi == 'arm64-v8a'
        report['abi'] = abi
        report['api'] = proof.adb_command('shell', 'getprop', 'ro.build.version.sdk').strip()
        report['os'] = proof.adb_command('shell', 'getprop', 'ro.build.version.release_or_codename').strip()
        proof.adb_command('shell', 'mkdir', '-p', remote)
        proof.upload(output / 'caller-c')
        proof.adb_command('shell', 'chmod', '700', remote + '/caller-c')
        prefix = ['@candidate-caller']
        for name in ('held', 'released', 'repeat'):
            request = {'version': 1, 'package': str(output / 'data'), 'ticks': prepare['TICKS'],
                       'input_recording': str(inputs / (name + '-input.json')),
                       'observations': str(inputs / (name + '-observe.json')),
                       'output_directory': str(output / ('c-' + name))}
            control(proof, 'c-' + name, prefix, request, 0,
                    oracle=json.loads((inputs / (name + '-expected.json')).read_text()),
                    native=json.loads((inputs / (name + '-native.json')).read_text()),
                    pcm=(inputs / (name + '.pcm')).read_bytes())
        accepted = files(Path(request['output_directory']))
        control(proof, 'c-no-clobber', prefix, request, 3)
        assert files(Path(request['output_directory'])) == accepted
        failed = {**request, 'output_directory': str(output / 'c-failed'),
                  'input_recording': str(inputs / 'failure-input.json')}
        control(proof, 'c-late-failure', prefix, failed, 1,
                error='script tick failed: Error: authored Android late failure')
        for name, extra in [('unknown-option', {'save': 'arbitrary'}), ('bad-version', {'version': 2}),
                            ('bad-ticks', {'ticks': 0}), ('relative-package', {'package': 'relative'})]:
            control(proof, 'c-' + name, prefix,
                    {**request, 'output_directory': str(output / ('c-' + name)), **extra}, 2)
        corrupt = output / 'corrupt-data'
        shutil.copytree(output / 'data', corrupt)
        (corrupt / 'world.scene').write_bytes(b'corrupt')
        control(proof, 'c-checksum-failure', prefix,
                {**request, 'package': str(corrupt), 'output_directory': str(output / 'c-corrupt')},
                1, error='native package startup checksum mismatch')
        proof.check_remote_inputs()
        assert files(exported / 'runtime') == before and files(inputs) == report['inputs']
        assert not authoring.exists() and not map_path.exists()
        report['passed'] = True
    finally:
        cleanup_errors = []
        try:
            if process is not None:
                report['emulator_cleanup'] = stop_owned_emulator(proof, process, avd_name)
        except BaseException as error:
            cleanup_errors.append(repr(error))
        finally:
            try:
                if created:
                    argv = [str(manager), 'delete', 'avd', '--name', avd_name]
                    report['delete'] = avd_command(output, 'avd-delete', argv, env)
                    assert not (avd_home / (avd_name + '.avd')).exists()
                    assert not (avd_home / (avd_name + '.ini')).exists()
            except BaseException as error:
                cleanup_errors.append(repr(error))
            finally:
                if cleanup_errors:
                    report['passed'] = False
                report.update(cleanup_errors=cleanup_errors, commands=proof.commands, controls=proof.controls)
                write(output / 'result.json', report)
        if cleanup_errors:
            raise RuntimeError('Owned emulator cleanup failed: ' + '; '.join(cleanup_errors))
    print(json.dumps({'passed': True, 'result': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
