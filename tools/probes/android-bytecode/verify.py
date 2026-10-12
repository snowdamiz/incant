#!/usr/bin/env python3
"""Execute the trusted candidate caller only in a newly owned ARM emulator.

Reuse the existing numeric C-ABI controls without changing their fixture or
admission. This adapter only transports their files across the ADB boundary.
"""
import argparse
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import socket
import subprocess
import sys
import time
import uuid

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
base = runpy.run_path(str(ROOT / 'tools/probes/mobile-runtime-library/verify.py'))
digest, files = base['digest'], base['files']


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


class AndroidProof(base['Proof']):
    def __init__(self, output, adb, serial, remote):
        super().__init__(output)
        self.adb, self.serial, self.remote = str(adb), serial, remote
        self.uploaded = {}

    def remote_path(self, path):
        relative = Path(path).relative_to(self.output)
        if not relative.parts or any(not re.fullmatch(r'[A-Za-z0-9_.-]+', p) or p in ('.', '..')
                                     for p in relative.parts):
            raise ValueError('Unexpected proof-relative path')
        return self.remote + '/' + relative.as_posix()

    def adb_command(self, *parts, expected=0, timeout=120):
        argv = [self.adb, '-s', self.serial, *parts]
        stem = self.output / f'command-{len(self.commands):03d}'
        stdout, stderr = Path(str(stem) + '.stdout'), Path(str(stem) + '.stderr')
        row = {'argv': argv, 'exit_code': None, 'timed_out': False}
        self.commands.append(row)
        start = time.monotonic()
        try:
            with stdout.open('wb') as out, stderr.open('wb') as err:
                result = base['run'](argv, timeout=timeout, stdout=out, stderr=err)
            row['exit_code'] = result.returncode
        except subprocess.TimeoutExpired:
            row['timed_out'] = True
            raise
        finally:
            row['seconds'] = time.monotonic() - start
            for key, path in [('stdout', stdout), ('stderr', stderr)]:
                row[key], row[key + '_sha256'] = str(path), digest(path)
        allowed = (expected,) if isinstance(expected, int) else expected
        if row['exit_code'] not in allowed:
            raise AssertionError(f'{stem}: expected {allowed}, got {row["exit_code"]}')
        return stdout.read_text()

    def upload(self, path):
        path = Path(path)
        if not path.exists():
            return
        manifest = files(path) if path.is_dir() else {path.name: digest(path)}
        if str(path) in self.uploaded:
            assert self.uploaded[str(path)] == manifest
            return
        remote = self.remote_path(path)
        self.adb_command('shell', 'mkdir', '-p', remote.rsplit('/', 1)[0])
        self.adb_command('push', str(path), remote)
        self.uploaded[str(path)] = manifest

    def command(self, argv, expected=0, timeout=120):
        argv = list(map(str, argv))
        if argv[0] != '@candidate-caller':
            return super().command(argv, expected, timeout)
        assert len(argv) == 2
        if argv[1] == '--null':
            return self.adb_command('shell', self.remote + '/caller-c', '--null', expected=expected)
        request_path = Path(argv[1])
        raw = request_path.read_bytes()
        request = None
        try:
            value = json.loads(raw)
        except (ValueError, UnicodeDecodeError):
            value = None
        if isinstance(value, dict):
            request = value.copy()
            for key in ('package', 'input_recording', 'observations', 'output_directory'):
                if key in value and Path(value[key]).is_absolute():
                    if key != 'output_directory':
                        self.upload(value[key])
                    value[key] = self.remote_path(value[key])
            raw = json.dumps(value).encode()
        wire = self.output / f'wire-{len(self.commands):03d}.json'
        wire.write_bytes(raw)
        self.upload(wire)
        response = self.adb_command('shell', self.remote + '/caller-c', self.remote_path(wire),
                                    expected=expected, timeout=timeout)
        if request:
            local = Path(request['output_directory'])
            remote = self.remote_path(local)
            self.adb_command('shell', 'test', '-d', remote, expected=1 if expected == 2 else 0)
            if expected != 2:
                pulled = self.output / f'pull-{len(self.commands):03d}'
                self.adb_command('pull', remote, str(pulled))
                if local.exists():
                    assert files(pulled) == files(local), 'Create-only remote output changed'
                    shutil.rmtree(pulled)
                else:
                    pulled.rename(local)
        return response

    def check_remote_inputs(self):
        for original, manifest in self.uploaded.items():
            local = Path(original)
            for name, expected in manifest.items():
                path = local / name if local.is_dir() else local
                result = self.adb_command('shell', 'sha256sum', self.remote_path(path))
                assert result.split()[0] == expected


def stop_owned_emulator(proof, process, name):
    """Always reap our Popen; never address an ADB serial with a changed owner."""
    result = {'graceful': False, 'fallback': False, 'adb_error': None}
    try:
        if process.poll() is None:
            try:
                actual = proof.adb_command('emu', 'avd', 'name', timeout=10).splitlines()
                if not actual or actual[0] != name:
                    raise RuntimeError('ADB serial no longer identifies the owned AVD')
                proof.adb_command('emu', 'kill', timeout=15)
                process.wait(timeout=20)
                result['graceful'] = True
            except (Exception, KeyboardInterrupt) as error:
                result['adb_error'] = str(error)
    finally:
        if process.poll() is None:
            result['fallback'] = True
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=10)
        else:
            process.wait(timeout=10)
    result['exit_code'] = process.returncode
    return result


def avd_command(output, label, argv, env, input_bytes=None):
    row = {'argv': argv, 'exit_code': None, 'timed_out': False}
    stdout, stderr = output / (label + '.stdout'), output / (label + '.stderr')
    try:
        with stdout.open('wb') as out, stderr.open('wb') as err:
            result = subprocess.run(argv, input=input_bytes, env=env, stdout=out, stderr=err, timeout=60)
        row['exit_code'] = result.returncode
    except subprocess.TimeoutExpired:
        row['timed_out'] = True
        raise
    finally:
        for key, path in [('stdout', stdout), ('stderr', stderr)]:
            row[key], row[key + '_sha256'] = str(path), digest(path)
        write(output / (label + '.json'), row)
    if result.returncode != 0:
        raise RuntimeError(label + ' failed')
    return row


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build', type=Path, required=True)
    parser.add_argument('--build-sha256', required=True)
    parser.add_argument('--sdk', type=Path, required=True)
    parser.add_argument('--java', type=Path, required=True)
    parser.add_argument('--fixture-generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--port', type=int, default=5584)
    args = parser.parse_args()
    build = args.build.resolve(strict=True)
    assert digest(build / 'result.json') == args.build_sha256
    receipt = json.loads((build / 'result.json').read_text())
    assert receipt['passed'] and receipt['admission'] == 'experimental compile-time numeric probe only'
    assert receipt['features'] == ['mobile-ffi', 'probe-android-bytecode']
    assert receipt['target'] == 'aarch64-linux-android' and receipt['profile'] == 'ci'
    assert receipt['private_build_storage_removed']
    for item in receipt['artifacts'].values():
        assert digest(item['path']) == item['sha256']
    for key in ('source', 'expectations'):
        assert digest(receipt[key]['path']) == receipt[key]['sha256']
    numeric = json.loads(Path(receipt['expectations']['path']).read_text())
    assert (numeric['format'], numeric['version'], numeric['tick_rate'], numeric['ticks']) == (
        'incant-mobile-bytecode-expectations', 1, 60, 3)
    assert args.port % 2 == 0 and 5554 <= args.port <= 5682
    for port in (args.port, args.port + 1):
        with socket.socket() as probe:
            probe.bind(('127.0.0.1', port))
    sdk, java = args.sdk.resolve(strict=True), args.java.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    name = 'incant-bytecode-' + uuid.uuid4().hex
    remote = '/data/local/tmp/' + name
    avd_home = output / 'avd'
    avd_home.mkdir()
    env = dict(os.environ, JAVA_HOME=str(java), ANDROID_AVD_HOME=str(avd_home), ANDROID_HOME=str(sdk))
    proof = AndroidProof(output, sdk / 'platform-tools/adb', f'emulator-{args.port}', remote)
    report = {'passed': False, 'execution': 'local ARM Android emulator CPU', 'physical_device': False,
              'build': str(build), 'build_sha256': args.build_sha256, 'serial': proof.serial,
              'avd': name, 'remote_directory': remote, 'system_image': 'system-images;android-37.0;google_apis;arm64-v8a',
              'fixture_generator_sha256': digest(args.fixture_generator),
              'sources': {p.name: digest(p) for p in HERE.glob('*.py')}}
    manager = sdk / 'cmdline-tools/latest/bin/avdmanager'
    process = None
    created = False
    try:
        create = [str(manager), 'create', 'avd', '--name', name, '--path', str(avd_home / (name + '.avd')),
                  '--package', report['system_image']]
        created = True  # Also clean up a partially created, uniquely named AVD.
        report['create'] = avd_command(output, 'avd-create', create, env, b'no\n')
        argv = [str(sdk / 'emulator/emulator'), '-avd', name, '-port', str(args.port), '-no-window',
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
        assert proof.adb_command('emu', 'avd', 'name').splitlines()[0] == name
        abi = proof.adb_command('shell', 'getprop', 'ro.product.cpu.abi').strip()
        assert abi == 'arm64-v8a'
        report['abi'] = abi
        report['api'] = proof.adb_command('shell', 'getprop', 'ro.build.version.sdk').strip()
        report['os'] = proof.adb_command('shell', 'getprop', 'ro.build.version.release_or_codename').strip()
        proof.adb_command('shell', 'mkdir', '-p', remote)
        shutil.copy2(receipt['artifacts']['caller-c']['path'], output / 'caller-c')
        proof.upload(output / 'caller-c')
        proof.adb_command('shell', 'chmod', '700', remote + '/caller-c')
        fixture = output / 'fixture'
        proof.command([args.fixture_generator, fixture])
        report['fixture_files'] = files(fixture)
        base['exercise'](proof, ['@candidate-caller'], 'c', fixture, 'android', 'ci', numeric)
        proof.check_remote_inputs()
        assert files(fixture) == report['fixture_files']
        report['passed'] = True
    finally:
        cleanup_errors = []
        try:
            if process is not None:
                report['emulator_cleanup'] = stop_owned_emulator(proof, process, name)
        except BaseException as error:
            cleanup_errors.append(repr(error))
        finally:
            try:
                if created:
                    argv = [str(manager), 'delete', 'avd', '--name', name]
                    report['delete'] = avd_command(output, 'avd-delete', argv, env)
                    assert not (avd_home / (name + '.avd')).exists()
                    assert not (avd_home / (name + '.ini')).exists()
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
