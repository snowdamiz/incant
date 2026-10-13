"""Read-only producer re-audit of retained fixed-physics iOS shipping evidence."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT = Path(__file__).parent
FREEZE = json.loads((OUT / 'freeze.json').read_text())
ROOT = Path(FREEZE['root'])
sys.path.insert(0, str(ROOT / 'tools'))
import ios_archive
from physics_features import selected_graph_errors, validate_artifacts


def sha(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def files(path):
    return {str(p.relative_to(path)): sha(p) for p in sorted(path.rglob('*')) if p.is_file()}


def raw_commands(rows, directory=None):
    for row in rows:
        for stream in ('stdout', 'stderr'):
            path = Path(row[stream])
            if not path.is_absolute():
                path = directory / path
            assert sha(path) == row[stream + '_sha256'], path
    return 2 * len(rows)


records = []
for mode in ('deterministic', 'fast'):
    prepared = OUT.parent / 'authored-ios-physics-preflight-001' / mode
    declared = read(prepared / 'result.json')
    assert declared['passed'] and declared['physics_mode'] == mode and declared['authoring_undo_redo_reopen']
    assert not (prepared / 'authoring').exists()
    for platform in ('simulator', 'device'):
        label = mode + '-' + platform
        exported = OUT / (label + '-export')
        caller = OUT / (label + '-callers')
        receipt = read(exported / 'export.json')
        result = read(caller / 'result.json')
        target = 'aarch64-apple-ios-sim' if platform == 'simulator' else 'aarch64-apple-ios'
        assert receipt['target'] == result['target'] == target and receipt['physics_mode'] == result['physics_mode'] == mode
        assert receipt['physics_variant'] == mode and result['passed'] and receipt['private_build_storage_removed']
        assert sha(exported / 'export.json') == result['export_sha256']
        producer = read(exported / 'project-build.json')
        ios_archive.producer_receipt(producer, target, declared['authoring_unchanged']['game.json'], mode)
        assert producer['script'] == receipt['script']
        graph = (exported / 'normal-dependencies.txt').read_text()
        assert ios_archive.normal_dependencies(graph) == receipt['normal_dependencies']
        assert selected_graph_errors(graph, mode, target) == []
        command = read(exported / 'build-command.json')
        assert command == receipt['command'] and command['exit_code'] == 0
        assert sha(exported / 'build.log') == command['stderr_sha256']
        assert sha(exported / 'cargo.jsonl') == command['stdout_sha256']
        target_dir = Path(command['argv'][command['argv'].index('--target-dir') + 1])
        assert not target_dir.parent.exists()
        log = (exported / 'build.log').read_text()
        assert ios_archive.verify_codegen(log, target) == receipt['codegen']
        actual_physics = ios_archive.kernel_provenance(log, target, mode)
        messages = [json.loads(line) for line in (exported / 'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
        actual_physics['compiled_target'] = validate_artifacts(messages, mode, target_dir=target_dir, target=target, profile='shipping')
        assert actual_physics == receipt['physics']
        assert files(exported / 'runtime') == result['runtime_before']
        assert files(exported / 'runtime/data') == receipt['data']['files']
        assert sha(exported / receipt['archive']['path']) == receipt['archive']['sha256']
        assert sha(exported / 'symbols/libincant_game_player.unstripped.a') == receipt['symbols']['unstripped_sha256']
        assert not (exported / 'symbols/behavior.js.map').exists()
        audits = read(exported / 'audit/commands.json')
        assert all(row['exit_code'] == 0 for row in audits)
        streams = raw_commands(audits, exported / 'audit') + raw_commands(result['commands'])
        for language, item in result['callers'].items():
            assert sha(item['path']) == item['sha256']
            link = [row for row in result['commands'] if row['argv'][:3] == ['xcrun', '--sdk', 'iphonesimulator' if platform == 'simulator' else 'iphoneos'] and row['argv'][3] == ('clang' if language == 'c' else 'swiftc')]
            assert len(link) == 1 and link[0]['exit_code'] == 0 and link[0]['argv'][-2:] == ['-Xlinker', '-dead_strip']
            sections = [row for row in result['commands'] if Path(row['argv'][0]).name == 'llvm-objdump' and row['argv'][-1] == item['path']]
            symbols = [row for row in result['commands'] if row['argv'][:3] == ['xcrun', 'nm', '-U'] and row['argv'][-1] == item['path']]
            assert len(sections) == len(symbols) == 1
            assert '__debug_' not in Path(sections[0]['stdout']).read_text()
            names = Path(symbols[0]['stdout']).read_text()
            assert all(name in names for name in ('incant_game_run_v1', 'JS_ReadObject', 'JS_EvalFunction'))
            assert not any(name in names for name in ios_archive.PARSERS)
        positive = refused = 0
        if platform == 'simulator':
            assert result['execution'] == 'actual_ios_simulator_cpu'
            udid = result['simulator']['udid']
            assert [row['argv'] for row in result['commands'][-2:]] == [['xcrun', 'simctl', 'shutdown', udid], ['xcrun', 'simctl', 'delete', udid]]
            assert all(row['exit_code'] == 0 for row in result['commands'][-2:])
            assert [row['expected_status'] for row in result['controls']] == [0, 0, 0, 1, 3, 1, 2, 2, 2, 2, 1] * 2
            opposite = 'fast' if mode == 'deterministic' else 'deterministic'
            foreign = OUT / (opposite + '-simulator-export')
            assert sha(foreign / 'export.json') == result['foreign_export_sha256']
            for control in result['controls']:
                assert sha(control['request']) == control['request_sha256']
                request = read(control['request'])
                destination = Path(request['output_directory'])
                status = control['expected_status']
                calls = [row for row in result['commands'] if row['argv'][-1] == control['request']]
                assert len(calls) == 1 and calls[0]['exit_code'] == status
                assert read(calls[0]['stdout']) == {'abi_status': status}
                if status == 0:
                    positive += 1
                    scenario = control['name'].split('-', 1)[1]
                    native = read(prepared / (scenario + '-native.json'))
                    oracle = read(prepared / (scenario + '-expected.json'))
                    report = read(destination / 'result.json')
                    assert report == control['report']
                    assert report['passed'] and report['error'] is None and report['os'] == 'ios' and report['architecture'] == 'aarch64'
                    assert report['panic'] == 'abort' and not report['debug_assertions'] and report['execution'] == 'offline_cpu'
                    game = report['game']
                    assert game['logs'] == native['logs'] == oracle['logs']
                    assert len(game['logs']['entries']) == 36 and game['logs']['dropped'] == 0
                    for key in ('tick', 'frames', 'observations', 'pcm_sha256'):
                        assert game[key] == native[key]
                    assert game['observations']['passed'] and len(game['observations']['checks']) == 72
                    pcm = (destination / 'audio.pcm').read_bytes()
                    assert pcm == (prepared / (scenario + '.pcm')).read_bytes() and len(pcm) == 76800 and any(pcm)
                else:
                    refused += 1
                    if status == 2:
                        assert not destination.exists()
                    elif status == 1:
                        report = read(destination / 'result.json')
                        assert report == control['report'] and not report['passed'] and report['game'] is None and not (destination / 'audio.pcm').exists()
                        suffix = control['name'].split('-', 1)[1]
                        error = {'wrong-mode': f'cooked physics kernel {opposite} does not match compiled kernel {mode}', 'late-failure': 'script tick failed: Error: authored iOS late failure', 'checksum-failure': 'native package startup checksum mismatch'}[suffix]
                        assert report['error'] == error
                        if suffix == 'wrong-mode':
                            assert files(Path(request['package'])) == files(foreign / 'runtime/data')
            assert positive == 6 and refused == 16
        else:
            assert result['execution'] == 'unsigned_device_build_and_link_only' and result['controls'] == []
        records.append({'mode': mode, 'platform': platform, 'target': target, 'export_receipt_sha256': sha(exported / 'export.json'), 'caller_receipt_sha256': sha(caller / 'result.json'), 'positive_processes': positive, 'exact_refusals': refused, 'raw_command_streams_rehashed': streams, 'archive_bytes': receipt['archive']['bytes'], 'caller_bytes': {key: Path(value['path']).stat().st_size for key, value in result['callers'].items()}, 'object_platform': receipt['symbols']['platform'], 'runtime_files': len(result['runtime_before'])})

fallback = []
for item in FREEZE['files']:
    current = ROOT / item['path']
    if current.exists() and sha(current) == item['sha256']:
        continue
    original = subprocess.check_output(['git', 'show', FREEZE['git_head'] + ':' + item['path']], cwd=ROOT)
    assert hashlib.sha256(original).hexdigest() == item['sha256'], item['path']
    fallback.append(item['path'])
original = read(OUT / 'commands.json')
continuation = read(OUT / 'continuation-commands-001.json')
assert [row['exit_code'] for row in original] == [0, 0, 1]
assert 'Output already exists; choose a new directory' in Path(original[2]['log']).read_text()
for command in original + continuation:
    assert sha(command['log']) == command['sha256']
commands = original[:2] + continuation
assert len(commands) == 8 and all(row['exit_code'] == 0 for row in commands)
terminal = read(OUT / 'terminal.json')
assert terminal['passed'] and terminal['changed'] == [] and terminal['commands'] == commands
assert terminal['prior_pause_refusal'] == original[2]
summary = {'passed': True, 'kind': 'producer re-audit; independent final review separate', 'frozen_inputs': len(FREEZE['files']), 'immutable_git_fallbacks': fallback, 'shipping_commands': len(commands), 'records': records}
(OUT / 'producer-reaudit.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
