#!/usr/bin/env python3
"""Actual ordinary iOS wasm2c C/Swift callers against native/Wasmtime per-field oracles.

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
files = prepare['files']
write = prepare['write_json']
digest = lambda path: sha256(Path(path).read_bytes())
ACTIONS = prepare['ACTIONS']
ERRORS = {name: 'static Wasm trapped (code 1)' for name, _, _ in ACTIONS[:3]}
ERRORS.update({'late-owner': 'runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist',
               'bad-role': 'invalid scene data: audio control requires final source',
               'seek-clip': 'invalid audio setting: pan, rate or start time outside supported range'})


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
            prepare['check_logs'](game['logs'], oracle, require)
            assert game['logs'] == native['logs']
            for key in ('tick', 'frames', 'pcm_sha256', 'observations'):
                assert game[key] == native[key], (name, key)
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
    require(receipt['features'] == physics_export.cargo_features(mode, ios_archive.CAPABILITIES + ['static-module'])
            and receipt['physics_variant'] == mode and receipt['profile'] == 'shipping'
            and receipt['command']['exit_code'] == 0 and receipt['private_build_storage_removed'] is True,
            'Expected a successful fresh shipping archive with removed private build storage')
    produced = json.loads(read(exported / 'project-build.json', 4 * 1024**2))
    ios_archive.producer_receipt(produced, receipt['target'], receipt['project_sha256'], mode, receipt['program'])
    require(produced['script'] == receipt['script'] and produced['scene_id'] == receipt['scene_id'], 'Export producer mismatch')
    declared = json.loads((prepared / 'result.json').read_text())
    require(declared['passed'] and declared['scene'] == receipt['scene_id']
            and declared['physics_mode'] == mode
            and declared['authoring_unchanged']['game.json'] == receipt['project_sha256'], 'Native preflight mismatch')
    require(receipt['program']['kind'] == 'wasm' and receipt['static_module'] is not None, 'Missing complete compiled scene program')
    require(receipt['program']['fingerprint'] == declared['program_asset']['sha256']
            and receipt['program']['module'] == declared['program_asset']['id'], 'Different captured module selected')
    require(digest(exported / 'runtime/data/world.scene') == declared['oracle_scene_sha256'], 'Export scene differs from native oracle')
    require(len(receipt['static_module']['compiled']['sources']) == 5, 'Incomplete actual C compilation audit')
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
                and other['features'] == physics_export.cargo_features(opposite, ios_archive.CAPABILITIES + ['static-module'])
                and other['target'] == receipt['target'] and other['physics_mode'] == other['physics_variant'] == opposite
                and other['command']['exit_code'] == 0 and other['private_build_storage_removed'],
                'Expected successful actual opposite-mode simulator export')
        foreign_producer = json.loads(read(foreign / 'project-build.json', 4 * 1024**2))
        ios_archive.producer_receipt(foreign_producer, other['target'], other['project_sha256'], opposite, other['program'])
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
    for map_path in sorted((exported / 'symbols').rglob('*')):
        if map_path.is_file() and (map_path.name.endswith('.map') or map_path.name == 'script-symbols.json'):
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
            assert all(name in names for name in ('incant_game_run_v1', 'incant_m0_update', 'wasm2c_m0_instantiate'))
            assert not any(name in names for name in ios_archive.PARSERS), language
        summary['callers'] = {language: {'path': str(exe), 'sha256': digest(exe)} for language, exe in executables.items()}
        if receipt['target'] == 'aarch64-apple-ios':
            summary['execution'] = 'unsigned_device_build_and_link_only'
        else:
            assert receipt['target'] == 'aarch64-apple-ios-sim'
            device = proof.command(['xcrun', 'simctl', 'create', 'Incant authored Wasm CPU proof',
                                    args.simulator_device_type, args.simulator_runtime]).strip()
            summary['simulator'] = {'udid': device, 'type': args.simulator_device_type, 'runtime': args.simulator_runtime}
            proof.command(['xcrun', 'simctl', 'boot', device])
            proof.command(['xcrun', 'simctl', 'bootstatus', device, '-b'], timeout=180)
            for language, exe in executables.items():
                prefix = ['xcrun', 'simctl', 'spawn', device, str(exe)]
                for index in range(3):
                    name = f'fresh-{index}'
                    request = {'version': 1, 'package': str(exported / 'runtime/data'), 'ticks': prepare['TICKS'],
                               'input_recording': str(prepared / 'positive-input.json'),
                               'observations': str(prepared / 'observe.json'),
                               'output_directory': str(args.output / f'{language}-{name}')}
                    control(proof, language + '-' + name, prefix, request, 0,
                            oracle=json.loads((prepared / 'oracle.json').read_text()),
                            native=json.loads((prepared / 'development-0.json').read_text()),
                            pcm=(prepared / 'oracle.pcm').read_bytes())
                wrong_mode = args.output / (language + '-wrong-mode-data')
                shutil.copytree(foreign / 'runtime/data', wrong_mode)
                control(proof, language + '-wrong-mode', prefix,
                        {**request, 'package': str(wrong_mode), 'output_directory': str(args.output / (language + '-wrong-mode'))},
                        1, error=f'cooked physics kernel {opposite} does not match compiled kernel {mode}')
                accepted = files(Path(request['output_directory']))
                control(proof, language + '-no-clobber', prefix, request, 3)
                assert files(Path(request['output_directory'])) == accepted
                for name, _, _ in ACTIONS:
                    failed = {**request, 'output_directory': str(args.output / (language + '-' + name)),
                              'input_recording': str(prepared / (name + '-input.json'))}
                    control(proof, language + '-' + name, prefix, failed, 1, error=ERRORS[name])
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
        assert files(exported / 'runtime') == before and not authoring.exists() and not any(Path(path).exists() for path in deletion['maps'])
        summary['passed'] = True
        summary['limits'] = ['Actual iOS simulator CPU only; device archive is unsigned build/link only.',
                             'Desired committed audio declarations are distinct from actual playback cursors.',
                             'No guest checkpoint capability is added: unknown save requests remain ABI refusals; native static-state checkpoint refusal is separately tested.',
                             'Exact late-failure reports/no PCM measure output boundary; callback rollback is separately covered by host conformance.',
                             'No mobile rendering, live audio device, performance, minimum OS, signing or phase gate.']
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
