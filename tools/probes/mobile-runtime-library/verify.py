#!/usr/bin/env python3
"""Link real C/Swift callers to a hash-bound mobile archive, then verify CPU runs.

The trusted default build fixture is deliberately explicit. No original document
smoke, authored mobile export, hardware-device or frame-time claim is made.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from native_process import run  # noqa: E402


def digest(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def files(root):
    return {str(path.relative_to(root)): digest(path)
            for path in sorted(root.rglob('*')) if path.is_file()}


def read_receipt(path, expected):
    if digest(path) != expected:
        raise ValueError('archive receipt checksum mismatch')
    data = json.loads(path.read_text())
    for key in ('archive', 'header'):
        if digest(Path(data[key]['path'])) != data[key]['sha256']:
            raise ValueError(f'{key} checksum mismatch')
    if 'bytes' in data['archive'] and Path(data['archive']['path']).stat().st_size != data['archive']['bytes']:
        raise ValueError('archive size mismatch')
    if data['features'] != ['mobile-ffi'] or data['command']['exit_code'] != 0:
        raise ValueError('expected successful default mobile-ffi archive')
    linked = data.get('linked_guest')
    if linked is None:
        fixture = 'crates/incant_game_player/fixtures/game.js'
        if not data.get('fixture', '').startswith('default trusted fixtures/game.js'):
            raise ValueError('archive does not declare the default trusted guest')
        if data['sources'][fixture] != digest(ROOT / fixture):
            raise ValueError('build-linked default guest differs from the typed fixture oracle')
    else:
        if linked.get('kind') != 'mobile-compat':
            raise ValueError('unsupported build-linked probe guest')
        for key in ('source', 'expectations'):
            if digest(linked[key]['path']) != linked[key]['sha256']:
                raise ValueError(f'linked guest {key} checksum mismatch')
        expected_path = Path(linked['expectations']['path'])
        if expected_path.stat().st_size > 65536:
            raise ValueError('numeric expectation limit')
        expected = json.loads(expected_path.read_text())
        if (expected['format'], expected['version'], expected['tick_rate'], expected['ticks']) != (
                'incant-mobile-bytecode-expectations', 1, 60, 3):
            raise ValueError('incompatible numeric fixture oracle')
        for key, translation in [('held', [0.1, 0, 0]), ('released', [0.05, 0, 0])]:
            scenario = expected['scenarios'][key]
            if scenario['translation'] != translation or len(scenario['messages']) != 15:
                raise ValueError('numeric fixture scenario mismatch')
            if any(not isinstance(v, str) or len(v.encode()) > 1024 for v in scenario['messages']):
                raise ValueError('numeric fixture message limit')
    return data


class Proof:
    def __init__(self, output):
        self.output = output
        self.commands = []
        self.controls = []

    def command(self, argv, expected=0, timeout=120):
        argv = [str(v) for v in argv]
        stem = f'command-{len(self.commands):03d}'
        stdout = self.output / (stem + '.stdout')
        stderr = self.output / (stem + '.stderr')
        start = time.monotonic()
        with stdout.open('wb') as out, stderr.open('wb') as err:
            result = run(argv, timeout=timeout, stdout=out, stderr=err, text=False)
        self.commands.append({'argv': argv, 'exit_code': result.returncode,
                              'seconds': time.monotonic() - start,
                              'stdout': str(stdout), 'stderr': str(stderr),
                              'stdout_sha256': digest(stdout), 'stderr_sha256': digest(stderr)})
        if result.returncode != expected:
            raise AssertionError(f'{stem}: expected status {expected}, got {result.returncode}; {stderr}')
        return stdout.read_text()

    def control(self, name, prefix, request, expected, *, error=None, pcm=None,
                malformed=False, expected_observations=True):
        path = self.output / (name + '.request.json')
        data = request if isinstance(request, bytes) else json.dumps(request).encode()
        path.write_bytes(data)
        response = json.loads(self.command([*prefix, path], expected))
        assert response == {'abi_status': expected}, (name, response)
        report = None
        if isinstance(request, dict):
            dest = Path(request['output_directory'])
            if malformed:
                assert not dest.exists(), name
            elif expected != 3:
                report = json.loads((dest / 'result.json').read_text())
                assert report['format'] == 'incant-mobile-runtime' and report['version'] == 1
                assert report['passed'] is (expected == 0)
                if expected == 0:
                    assert report['error'] is None
                assert report['execution'] == 'offline_cpu'
                assert report['architecture'] == 'aarch64'
                if error is not None:
                    assert report['error'] == error, (name, report['error'])
                if pcm is None:
                    assert not (dest / 'audio.pcm').exists(), name
                    assert report['game'] is None
                else:
                    actual = (dest / 'audio.pcm').read_bytes()
                    assert actual == pcm, name
                    game = report['game']
                    assert game['tick'] == 3 and game['frames'] == 2400
                    assert game['observations']['passed'] is expected_observations
                    assert game['pcm_sha256'] == hashlib.sha256(actual).hexdigest()
        self.controls.append({'name': name, 'expected_status': expected,
                              'request': str(path), 'request_sha256': digest(path),
                              'result': report})
        return report


def link(proof, receipt, language):
    target = receipt['target']
    header = Path(receipt['header']['path'])
    archive = Path(receipt['archive']['path'])
    exe = proof.output / f'caller-{language}'
    sdk, triple = {
        'aarch64-apple-darwin': ('macosx', 'arm64-apple-macos13.0'),
        'aarch64-apple-ios-sim': ('iphonesimulator', 'arm64-apple-ios15.0-simulator'),
        'aarch64-apple-ios': ('iphoneos', 'arm64-apple-ios15.0'),
    }[target]
    sdk_path = proof.command(['xcrun', '--sdk', sdk, '--show-sdk-path']).strip()
    common = ['-target', triple, '-o', exe]
    native = ['-framework', 'Foundation', '-framework', 'Security', '-liconv', '-lresolv', '-lc++']
    if language == 'c':
        argv = ['xcrun', '--sdk', sdk, 'clang', '-std=c11', '-Wall', '-Wextra', '-Werror',
                '-isysroot', sdk_path, *common, '-I', header.parent, HERE / 'caller.c', archive, *native]
    else:
        argv = ['xcrun', '--sdk', sdk, 'swiftc', '-warnings-as-errors', '-sdk', sdk_path,
                *common, '-import-objc-header', header, HERE / 'caller.swift', archive, *native]
    proof.command(argv)
    proof.command(['xcrun', 'nm', '-gU', exe])
    return exe


def checked_game(report, scenario, numeric=None):
    game = report['game']
    actual = game['observations']['entities']['00000000000000000000000001']['translation']
    assert actual == ([0.1, 0, 0] if scenario == 'held' else [0.05, 0, 0])
    expected_entries = [] if numeric is None else [
        {'tick': index // 5 + 1, 'level': 'info', 'message': message}
        for index, message in enumerate(numeric['scenarios'][scenario]['messages'])]
    assert game['logs'] == {'entries': expected_entries, 'dropped': 0}


def exercise(proof, prefix, language, fixture, expected_os, expected_profile, numeric=None):
    package_before = files(fixture / 'package')
    request = {'version': 1, 'package': str(fixture / 'package'), 'ticks': 3,
               'observations': str(fixture / 'observe.json')}
    def req(suffix, **extra):
        return {**request, 'output_directory': str(proof.output / f'{language}-{suffix}'), **extra}
    pcm = bytes(2400 * 8)
    accepted = req('accepted')
    report = proof.control(f'{language}-accepted', prefix, accepted, 0, pcm=pcm)
    assert report['os'] == expected_os
    assert report['panic'] == ('abort' if expected_profile == 'shipping' else 'unwind')
    checked_game(report, 'held', numeric)
    outputs = files(Path(accepted['output_directory']))
    proof.control(f'{language}-no-clobber', prefix, accepted, 3)
    assert outputs == files(Path(accepted['output_directory']))
    repeated = proof.control(f'{language}-repeat', prefix, req('repeat'), 0, pcm=pcm)
    assert repeated == report
    released = proof.control(f'{language}-release', prefix, req('release',
                  observations=str(fixture / 'observe-release.json'),
                  input_recording=str(fixture / 'release.json')), 0, pcm=pcm)
    checked_game(released, 'released', numeric)
    proof.control(f'{language}-input-io-failure', prefix, req('input-io-failure',
                  input_recording=str(fixture / 'absent-input.json')), 1,
                  error='No such file or directory (os error 2)')
    proof.control(f'{language}-guest-failure', prefix, req('failure',
                  input_recording=str(fixture / 'failure.json')), 1,
                  error='script tick failed: Error: accepted game input failure')
    observed_failure = proof.control(f'{language}-observation-failure', prefix, req('observation-failure',
                  observations=str(fixture / 'observe-wrong.json')), 1, pcm=pcm,
                  error='native observation assertions failed (see completed report)',
                  expected_observations=False)
    checked_game(observed_failure, 'held', numeric)
    corrupt = proof.output / f'{language}-corrupt-package'
    shutil.copytree(fixture / 'package', corrupt)
    (corrupt / 'world.scene').write_bytes(b'corrupt')
    proof.control(f'{language}-package-failure', prefix, req('corrupt', package=str(corrupt)), 1,
                  error='native package startup checksum mismatch')
    for suffix, extra in [('version', {'version': 2}), ('ticks', {'ticks': 0}),
                          ('relative', {'package': 'relative'}), ('unknown', {'window': True}),
                          ('viewport', {'ui_viewport': [0, 720]})]:
        proof.control(f'{language}-invalid-{suffix}', prefix, req(suffix, **extra), 2, malformed=True)
    proof.control(f'{language}-empty', prefix, b'', 2)
    proof.control(f'{language}-oversized', prefix, b' ' * 65537, 2)
    assert json.loads(proof.command([*prefix, '--null'], 2)) == {'abi_status': 2}
    proof.controls.append({'name': f'{language}-null', 'expected_status': 2})
    assert files(fixture / 'package') == package_before


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive-receipt', type=Path, required=True)
    parser.add_argument('--receipt-sha256', required=True)
    parser.add_argument('--fixture-generator', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--simulator-device-type', default='com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro')
    parser.add_argument('--simulator-runtime', default='com.apple.CoreSimulator.SimRuntime.iOS-27-0')
    args = parser.parse_args()
    receipt = read_receipt(args.archive_receipt, args.receipt_sha256)
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    proof = Proof(args.output)
    numeric = None
    if receipt.get('linked_guest') is not None:
        numeric = json.loads(Path(receipt['linked_guest']['expectations']['path']).read_text())
    passed, device = False, None
    summary = {'archive_receipt': str(args.archive_receipt), 'receipt_sha256': args.receipt_sha256,
               'archive': receipt['archive'], 'header': receipt['header'],
               'target': receipt['target'], 'profile': receipt['profile'],
               'linked_guest': receipt.get('linked_guest'),
               'fixture_generator': {'path': str(args.fixture_generator), 'sha256': digest(args.fixture_generator)},
               'probe_sources': {str(p.relative_to(HERE)): digest(p) for p in sorted(HERE.rglob('*')) if p.is_file() and '__pycache__' not in p.parts}, 'claims': 'explicit trusted build-linked cooked CPU fixture only'}
    try:
        fixture = args.output / 'fixture'
        proof.command([args.fixture_generator, fixture])
        summary['fixture_files'] = files(fixture)
        executables = {language: link(proof, receipt, language) for language in ('c', 'swift')}
        summary['callers'] = {key: {'path': str(path), 'sha256': digest(path)} for key, path in executables.items()}
        target = receipt['target']
        if target == 'aarch64-apple-ios':
            summary['execution'] = 'build_and_link_only_no_device'
        else:
            if target == 'aarch64-apple-ios-sim':
                device = proof.command(['xcrun', 'simctl', 'create', 'Incant mobile CPU ABI probe',
                                        args.simulator_device_type, args.simulator_runtime]).strip()
                summary['simulator'] = {'udid': device, 'type': args.simulator_device_type,
                                        'runtime': args.simulator_runtime}
                proof.command(['xcrun', 'simctl', 'boot', device])
                proof.command(['xcrun', 'simctl', 'bootstatus', device, '-b'], timeout=180)
            for language, exe in executables.items():
                prefix = ['xcrun', 'simctl', 'spawn', device, str(exe)] if device else [str(exe)]
                exercise(proof, prefix, language, fixture, 'ios' if device else 'macos', receipt['profile'], numeric)
            summary['execution'] = 'ios_simulator_cpu' if device else 'macos_cpu'
        assert digest(receipt['archive']['path']) == receipt['archive']['sha256']
        assert digest(receipt['header']['path']) == receipt['header']['sha256']
        passed = True
    finally:
        try:
            if device:
                # Only this freshly created device is touched, never other simulators.
                try:
                    proof.command(['xcrun', 'simctl', 'shutdown', device])
                finally:
                    proof.command(['xcrun', 'simctl', 'delete', device])
        except BaseException:
            passed = False
            raise
        finally:
            summary.update(passed=passed, commands=proof.commands, controls=proof.controls)
            (args.output / 'result.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(args.output / 'result.json')


if __name__ == '__main__':
    main()
