#!/usr/bin/env python3
"""Actual C/Swift callers for the separate authored iOS archive receipt.

Only the private prepare.py fixture and newly produced source map are removed.
The default/numeric mobile caller admission contract is not changed or reused.
"""
import argparse
import json
from pathlib import Path
import runpy
import shutil
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import ios_archive
import physics_export
from native_package import read, require, sha256

base = runpy.run_path(str(ROOT / 'tools/probes/mobile-runtime-library/verify.py'))
prepare = runpy.run_path(str(HERE / 'prepare.py'))
digest, files, write = prepare['digest'], prepare['files'], prepare['write']


class Proof(base['Proof']):
    def command(self, argv, expected=0, timeout=120):
        argv = list(map(str, argv))
        if argv[:1] == ['xcrun'] and len(argv) > 3 and argv[3] in ('clang', 'swiftc'):
            argv += ['-Xlinker', '-dead_strip']
        return super().command(argv, expected, timeout)


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
        assert report['os'] == 'ios' and report['architecture'] == 'aarch64'
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
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--remove-authoring', action='store_true')
    parser.add_argument('--foreign-export', type=Path)
    parser.add_argument('--foreign-export-sha256')
    parser.add_argument('--simulator-device-type', default='com.apple.CoreSimulator.SimDeviceType.iPhone-18-Pro')
    parser.add_argument('--simulator-runtime', default='com.apple.CoreSimulator.SimRuntime.iOS-27-0')
    args = parser.parse_args()
    prepared, exported = args.prepared.resolve(strict=True), args.exported.resolve(strict=True)
    require(digest(exported / 'export.json') == args.export_sha256, 'Archive export receipt checksum mismatch')
    receipt = json.loads(read(exported / 'export.json', 4 * 1024**2))
    require(receipt['format'] == 'incant-ios-project-archive' and receipt['version'] == 1,
            'Expected authored iOS archive receipt')
    mode = receipt['physics_mode']
    require(receipt['features'] == physics_export.cargo_features(mode, ios_archive.CAPABILITIES)
            and receipt['physics_variant'] == mode and receipt['profile'] == 'shipping'
            and receipt['command']['exit_code'] == 0 and receipt['private_build_storage_removed'] is True,
            'Expected a successful fresh shipping archive with removed private build storage')
    produced = json.loads(read(exported / 'project-build.json', 4 * 1024**2))
    ios_archive.producer_receipt(produced, receipt['target'], receipt['project_sha256'], mode)
    require(produced['script'] == receipt['script'] and produced['scene_id'] == receipt['scene_id'], 'Export producer mismatch')
    declared = json.loads((prepared / 'result.json').read_text())
    require(declared['passed'] and declared['scene'] == receipt['scene_id']
            and declared['physics_mode'] == mode
            and declared['authoring_unchanged']['game.json'] == receipt['project_sha256'], 'Native preflight mismatch')
    foreign = None
    if receipt['target'] == 'aarch64-apple-ios-sim':
        require(args.foreign_export is not None and args.foreign_export_sha256 is not None,
                'Simulator proof requires the actual opposite-mode export')
        foreign = args.foreign_export.resolve(strict=True)
        require(digest(foreign / 'export.json') == args.foreign_export_sha256, 'Foreign receipt checksum mismatch')
        other = json.loads(read(foreign / 'export.json', 4 * 1024**2))
        opposite = 'fast' if mode == 'deterministic' else 'deterministic'
        require(other['format'] == 'incant-ios-project-archive' and other['version'] == 1
                and other['profile'] == 'shipping'
                and other['features'] == physics_export.cargo_features(opposite, ios_archive.CAPABILITIES)
                and other['target'] == receipt['target'] and other['physics_mode'] == other['physics_variant'] == opposite
                and other['command']['exit_code'] == 0 and other['private_build_storage_removed'],
                'Expected successful actual opposite-mode simulator export')
        foreign_producer = json.loads(read(foreign / 'project-build.json', 4 * 1024**2))
        ios_archive.producer_receipt(foreign_producer, other['target'], other['project_sha256'], opposite)
        require(foreign_producer['script'] == other['script'] and foreign_producer['scene_id'] == other['scene_id'],
                'Foreign producer mismatch')
        require(files(foreign / 'runtime/data') == other['data']['files'], 'Foreign cooked package differs from receipt')
    linked = {'target': receipt['target']}
    for kind, exact in [('archive', 'runtime/libincant_game_player.a'), ('header', 'runtime/incant_game.h')]:
        require(receipt[kind]['path'] == exact, 'Unexpected runtime artifact path')
        path = exported / exact
        require(digest(path) == receipt[kind]['sha256'], kind + ' checksum mismatch')
        linked[kind] = {**receipt[kind], 'path': str(path)}
    for name, expected in receipt['data']['files'].items():
        require(name in files(exported / 'runtime/data') and digest(exported / 'runtime/data' / name) == expected,
                'Cooked package data differs from archive receipt')
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    proof = Proof(args.output)
    authoring = prepared / 'authoring'
    deletion = {'authoring': str(authoring), 'maps': {}}
    if args.remove_authoring:
        require(Path(declared['project']) == authoring / 'game.json' and files(authoring) == declared['authoring_unchanged'],
                'Private authoring changed; refusing proof cleanup')
        deletion['authoring_hashes'] = files(authoring)
        require(digest(authoring / 'game.json') == receipt['project_sha256'], 'Authored project identity differs')
        shutil.rmtree(authoring)
        write(prepared / 'source-removal.json', deletion)
    require(not authoring.exists(), 'Remove private prepared project/source/cache before this proof')
    map_path = exported / 'symbols/behavior.js.map'
    if map_path.exists():
        deletion['maps'][str(map_path)] = digest(map_path)
        map_path.unlink()
    write(args.output / 'source-removal.json', deletion)
    before = files(exported / 'runtime')
    summary = {'passed': False, 'export': str(exported), 'export_sha256': args.export_sha256,
               'target': receipt['target'], 'physics_mode': mode, 'archive': linked['archive'], 'prepared': str(prepared),
               'foreign_export': str(foreign) if foreign else None,
               'foreign_export_sha256': args.foreign_export_sha256,
               'source_removal': deletion, 'runtime_before': before,
               'probe_sources': {p.name: digest(p) for p in HERE.iterdir() if p.is_file()},
               'caller_sources': {p.name: digest(p) for p in (ROOT / 'tools/probes/mobile-runtime-library').iterdir() if p.is_file()}}
    device = None
    try:
        executables = {language: base['link'](proof, linked, language) for language in ('c', 'swift')}
        rustc = Path(proof.command(['rustup', 'which', 'rustc']).strip())
        sysroot = Path(proof.command([rustc, '--print', 'sysroot']).strip())
        llvm = sysroot / 'lib/rustlib/aarch64-apple-darwin/bin'
        for language, exe in executables.items():
            sections = proof.command([llvm / 'llvm-objdump', '--section-headers', exe])
            require('__debug_' not in sections, 'Final caller retains DWARF sections')
            names = proof.command(['xcrun', 'nm', '-U', exe])
            assert 'incant_game_run_v1' in names and 'JS_ReadObject' in names and 'JS_EvalFunction' in names
            assert not any(name in names for name in ios_archive.PARSERS), language
        summary['callers'] = {language: {'path': str(exe), 'sha256': digest(exe)} for language, exe in executables.items()}
        if receipt['target'] == 'aarch64-apple-ios':
            summary['execution'] = 'unsigned_device_build_and_link_only'
        else:
            assert receipt['target'] == 'aarch64-apple-ios-sim'
            device = proof.command(['xcrun', 'simctl', 'create', 'Incant authored project CPU proof',
                                    args.simulator_device_type, args.simulator_runtime]).strip()
            summary['simulator'] = {'udid': device, 'type': args.simulator_device_type, 'runtime': args.simulator_runtime}
            proof.command(['xcrun', 'simctl', 'boot', device])
            proof.command(['xcrun', 'simctl', 'bootstatus', device, '-b'], timeout=180)
            for language, exe in executables.items():
                prefix = ['xcrun', 'simctl', 'spawn', device, str(exe)]
                for name in ('held', 'released', 'repeat'):
                    request = {'version': 1, 'package': str(exported / 'runtime/data'), 'ticks': prepare['TICKS'],
                               'input_recording': str(prepared / (name + '-input.json')),
                               'observations': str(prepared / (name + '-observe.json')),
                               'output_directory': str(args.output / f'{language}-{name}')}
                    control(proof, language + '-' + name, prefix, request, 0,
                            oracle=json.loads((prepared / (name + '-expected.json')).read_text()),
                            native=json.loads((prepared / (name + '-native.json')).read_text()),
                            pcm=(prepared / (name + '.pcm')).read_bytes())
                wrong_mode = args.output / (language + '-wrong-mode-data')
                shutil.copytree(foreign / 'runtime/data', wrong_mode)
                control(proof, language + '-wrong-mode', prefix,
                        {**request, 'package': str(wrong_mode), 'output_directory': str(args.output / (language + '-wrong-mode'))},
                        1, error=f'cooked physics kernel {opposite} does not match compiled kernel {mode}')
                accepted = files(Path(request['output_directory']))
                control(proof, language + '-no-clobber', prefix, request, 3)
                assert files(Path(request['output_directory'])) == accepted
                failed = {**request, 'output_directory': str(args.output / (language + '-failed')),
                          'input_recording': str(prepared / 'failure-input.json')}
                control(proof, language + '-late-failure', prefix, failed, 1,
                        error='script tick failed: Error: authored iOS late failure')
                for name, extra in [('unknown-option', {'save': 'arbitrary'}), ('bad-version', {'version': 2}),
                                    ('bad-ticks', {'ticks': 0}), ('relative-package', {'package': 'relative'})]:
                    control(proof, language + '-' + name, prefix,
                            {**request, 'output_directory': str(args.output / (language + '-' + name)), **extra}, 2)
                corrupt = args.output / (language + '-corrupt-data')
                shutil.copytree(exported / 'runtime/data', corrupt)
                (corrupt / 'world.scene').write_bytes(b'corrupt')
                control(proof, language + '-checksum-failure', prefix,
                        {**request, 'package': str(corrupt), 'output_directory': str(args.output / (language + '-corrupt'))},
                        1, error='native package startup checksum mismatch')
            require(files(foreign / 'runtime/data') == other['data']['files'], 'Foreign package changed')
            for language in ('c', 'swift'):
                require(files(args.output / (language + '-wrong-mode-data')) == other['data']['files'],
                        'Checksum-valid wrong-mode package changed')
            summary['execution'] = 'actual_ios_simulator_cpu'
        assert files(exported / 'runtime') == before and not authoring.exists() and not map_path.exists()
        summary['passed'] = True
    finally:
        try:
            if device:
                try:
                    proof.command(['xcrun', 'simctl', 'shutdown', device])
                finally:
                    proof.command(['xcrun', 'simctl', 'delete', device])
        except BaseException:
            summary['passed'] = False
            raise
        finally:
            summary.update(commands=proof.commands, controls=proof.controls)
            write(args.output / 'result.json', summary)
    print(json.dumps({'passed': True, 'result': str(args.output / 'result.json')}))


if __name__ == '__main__':
    main()
