"""Read-only producer re-audit of retained iOS static-Wasm shipping evidence."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT = Path.home()/'.codex/incant-evidence/authored-ios-wasm-shipping-001'
FREEZE = json.loads((OUT / 'continuation-freeze-002.json').read_text())
ORIGINAL = json.loads((OUT / 'freeze.json').read_text())
ROOT = Path('/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant')
sys.path.insert(0, str(ROOT / 'tools'))
import ios_archive
import ios_wasm
import runpy
audio = runpy.run_path(str(ROOT / "tools/probes/native-wasm-audio-export.py"))
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
    prepared = OUT.parent / 'authored-ios-wasm-preflight-001' / mode
    declared = read(prepared / 'result.json')
    assert declared['passed'] and declared['physics_mode'] == mode and declared['public_import_select_mode_undo_redo_reopen']
    assert not (prepared / 'authoring').exists()
    calls = read(prepared / 'public-commands.json')
    assert [row['method'] for row in calls] == ['project.read', 'command.execute', 'project.read', 'history.undo', 'project.read', 'history.redo', 'project.read', 'project.save', 'project.read', 'project.read']
    assert calls[0]['result']['project'] == calls[4]['result']['project']
    assert calls[2]['result']['project'] == calls[6]['result']['project']
    assert calls[8]['result'] == calls[9]['result']
    assert calls[1]['params']['commands'][-2] == {'op': 'set_physics_mode', 'mode': mode}
    assert calls[2]['result']['project']['scenes'][declared['scene']]['program'] == {'kind': 'wasm', 'module': declared['program_asset']['id']}
    native_runs = [read(prepared / f'development-{i}.json') for i in range(2)]
    assert native_runs[0] == native_runs[1]
    oracle = read(prepared / 'oracle.json')
    assert len(oracle) == 96 and all(len(row['values']) == 186 for row in oracle)
    for i, native in enumerate(native_runs):
        assert native['passed'] and not native['script_source_parsing'] and native['cpu_render_input'] and not native['gpu_submitted']
        audio['check_logs'](native['logs'], oracle, lambda ok, reason: None if ok else (_ for _ in ()).throw(AssertionError(reason)))
        assert (prepared / f'development-{i}.pcm').read_bytes() == (prepared / 'oracle.pcm').read_bytes()
    preflight_commands = read(prepared / 'commands.json')
    assert len(preflight_commands) == 14 and [r['exit_code'] for r in preflight_commands] == [0]*8 + [1]*6
    for row in preflight_commands:
        for stream in ('stdout', 'stderr'):
            assert sha(prepared / row[stream]['path']) == row[stream]['sha256']
    for row, (name, _, _) in zip(preflight_commands[-6:], audio['ACTIONS']):
        assert not (prepared / row['stdout']['path']).read_bytes()
        assert (prepared / row['stderr']['path']).read_text().strip() == audio['DEV_ERRORS'][name]
        assert not (prepared / ('refused-' + name + '.pcm')).exists()
    for path, expected in declared['source_deleted'].items():
        assert not (prepared / path).exists()
    for path, expected in declared['binaries'].items():
        assert sha(path) == expected
    for platform in ('simulator', 'device'):
        label = mode + '-' + platform
        exported = OUT / (label + '-archive')
        caller = OUT / (label + '-callers' + ('-002' if label == 'fast-simulator' else ''))
        receipt = read(exported / 'export.json')
        result = read(caller / 'result.json')
        target = 'aarch64-apple-ios-sim' if platform == 'simulator' else 'aarch64-apple-ios'
        assert receipt['target'] == result['target'] == target and receipt['physics_mode'] == result['physics_mode'] == mode
        assert receipt['physics_variant'] == mode and result['passed'] and receipt['private_build_storage_removed']
        assert sha(exported / 'export.json') == result['export_sha256']
        producer = read(exported / 'project-build.json')
        ios_archive.producer_receipt(producer, target, declared['authoring_unchanged']['game.json'], mode, receipt['program'])
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
        assert ios_archive.verify_codegen(log, target, True) == receipt['codegen']
        actual_physics = ios_archive.kernel_provenance(log, target, mode)
        messages = [json.loads(line) for line in (exported / 'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
        actual_physics['compiled_target'] = validate_artifacts(messages, mode, target_dir=target_dir, target=target, profile='shipping')
        assert actual_physics == receipt['physics']
        assert receipt['program']['fingerprint'] == declared['program_asset']['sha256']
        assert sha(exported / 'runtime/data/world.scene') == declared['oracle_scene_sha256']
        compiled = receipt['static_module']['compiled']
        c_sources = compiled['sources']
        facade = next(Path(path) for path in c_sources if path.endswith('/m0-facade.c'))
        c_prepared = {'bundle': str(facade.parent), 'compiler': compiled['compiler']}
        assert ios_wasm.audit_build((exported / 'cargo.jsonl').read_text(), log, target, compiled['sdkroot'], c_prepared, ROOT) == compiled
        assert not facade.parent.exists()
        assert read(exported / 'wasm-translation.json') == receipt['static_module']['translation']
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
            assert all(name in names for name in ('incant_game_run_v1', 'incant_m0_update', 'wasm2c_m0_instantiate'))
            assert not any(name in names for name in ios_archive.PARSERS)
        positive = refused = 0
        if platform == 'simulator':
            assert result['execution'] == 'actual_ios_simulator_cpu'
            udid = result['simulator']['udid']
            assert [row['argv'] for row in result['commands'][-2:]] == [['xcrun', 'simctl', 'shutdown', udid], ['xcrun', 'simctl', 'delete', udid]]
            assert all(row['exit_code'] == 0 for row in result['commands'][-2:])
            assert [row['expected_status'] for row in result['controls']] == ([0, 0, 0, 1, 3] + [1]*6 + [2]*4 + [1]) * 2
            opposite = 'fast' if mode == 'deterministic' else 'deterministic'
            foreign = OUT / (opposite + '-simulator-archive')
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
                    native = read(prepared / 'development-0.json')
                    oracle = read(prepared / 'oracle.json')
                    report = read(destination / 'result.json')
                    assert report == control['report']
                    assert report['passed'] and report['error'] is None and report['os'] == 'ios' and report['architecture'] == 'aarch64'
                    assert report['panic'] == 'abort' and not report['debug_assertions'] and report['execution'] == 'offline_cpu'
                    game = report['game']
                    assert game['logs'] == native['logs']
                    audio['check_logs'](game['logs'], oracle, lambda ok, reason: None if ok else (_ for _ in ()).throw(AssertionError(reason)))
                    assert len(game['logs']['entries']) == 576 and game['logs']['dropped'] == 0
                    for key in ('tick', 'frames', 'observations', 'pcm_sha256'):
                        assert game[key] == native[key]
                    assert game['observations']['passed'] and len(game['observations']['checks']) == 248
                    pcm = (destination / 'audio.pcm').read_bytes()
                    assert pcm == (prepared / 'oracle.pcm').read_bytes() and len(pcm) == 614400 and any(pcm)
                else:
                    refused += 1
                    if status == 2:
                        assert not destination.exists()
                    elif status == 1:
                        report = read(destination / 'result.json')
                        assert report == control['report'] and not report['passed'] and report['game'] is None and not (destination / 'audio.pcm').exists()
                        suffix = control['name'].split('-', 1)[1]
                        error = {'wrong-mode': f'cooked physics kernel {opposite} does not match compiled kernel {mode}', **{name: 'static Wasm trapped (code 1)' for name, _, _ in audio['ACTIONS'][:3]}, 'late-owner': 'runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist', 'bad-role': 'invalid scene data: audio control requires final source', 'seek-clip': 'invalid audio setting: pan, rate or start time outside supported range', 'checksum-failure': 'native package startup checksum mismatch'}[suffix]
                        assert report['error'] == error
                        if suffix == 'wrong-mode':
                            assert files(Path(request['package'])) == files(foreign / 'runtime/data')
            assert positive == 6 and refused == 26
        else:
            assert result['execution'] == 'unsigned_device_build_and_link_only' and result['controls'] == []
        records.append({'mode': mode, 'platform': platform, 'target': target, 'export_receipt_sha256': sha(exported / 'export.json'), 'caller_receipt_sha256': sha(caller / 'result.json'), 'positive_processes': positive, 'exact_refusals': refused, 'raw_command_streams_rehashed': streams, 'archive_bytes': receipt['archive']['bytes'], 'caller_bytes': {key: Path(value['path']).stat().st_size for key, value in result['callers'].items()}, 'object_platform': receipt['symbols']['platform'], 'runtime_files': len(result['runtime_before'])})

fallback = []
for path, expected in FREEZE['files'].items():
    current = ROOT / path
    if current.exists() and sha(current) == expected:
        continue
    original = subprocess.check_output(['git', 'show', FREEZE['commit'] + ':' + path], cwd=ROOT)
    assert hashlib.sha256(original).hexdigest() == expected, path
    fallback.append(path)
original_commands = read(OUT / 'commands.json')
assert len(original_commands) == 1 and original_commands[0]['exit_code'] == 1
assert 'Missing actual target static-module C compilation' in Path(original_commands[0]['stderr']['path']).read_text()
archive_commands = read(OUT / 'continuation-commands-002.json')
first_callers = read(OUT / 'caller-commands-003.json')
retry_callers = read(OUT / 'caller-commands-004.json')
assert [row['exit_code'] for row in first_callers] == [0, 0, 1]
assert len(retry_callers) == 2
caller_commands = first_callers[:2] + retry_callers
assert len(archive_commands) == 3 and len(caller_commands) == 4
assert all(row['exit_code'] == 0 for row in archive_commands + caller_commands)
commands = original_commands + archive_commands + first_callers + retry_callers
for row in commands:
    for stream in ('stdout', 'stderr'):
        assert sha(row[stream]['path']) == row[stream]['sha256']
archive_terminal = read(OUT / 'continuation-terminal-002.json')
caller_terminal = read(OUT / 'caller-terminal-004.json')
assert archive_terminal['passed'] and archive_terminal['unchanged_inputs'] == len(FREEZE['files']) and archive_terminal['commands'] == archive_commands
failed = read(caller_terminal['retained_failed_attempt']['path'])
assert sha(caller_terminal['retained_failed_attempt']['path']) == caller_terminal['retained_failed_attempt']['sha256']
assert not failed['passed'] and failed['commands'][-3]['exit_code'] == 124
assert 'Domain is tearing down' in Path(failed['commands'][-3]['stderr']).read_text()
assert failed['commands'][-2]['exit_code'] == 149 and failed['commands'][-1]['exit_code'] == 0
raw_commands(failed['commands'])
assert caller_terminal['passed'] and caller_terminal['inputs_unchanged'] == len(FREEZE['files']) and caller_terminal['commands'] == caller_commands
recovery = read(OUT / 'deterministic-simulator-archive/export.json')['reused_compiler_output']
assert recovery['original_compiler_binary_sha256_recorded'] is False
assert sha(recovery['receipt']) == recovery['receipt_sha256']
assert recovery['archive_sha256_before_strip'] == '6af80f4730bf036105140067884c0bbfeed1e79055d2fa31e6a6632e6663b800'
assert recovery['source_commit'] == ORIGINAL['commit']
recovery_terminal = read(OUT / 'recovery-terminal-001.json')
assert recovery_terminal['exit_code'] == 0 and recovery_terminal['terminal_reaped'] is True
assert recovery_terminal['result']['passed'] and recovery_terminal['result']['reuse_not_fresh_compile']
assert sha(recovery_terminal['combined_log']['path']) == recovery_terminal['combined_log']['sha256']
assert sha(recovery_terminal['argv'][1]) == recovery_terminal['source_sha256']
assert not (OUT / 'completed-compiler-preserved').exists()
original_fallback = []
for path, expected in ORIGINAL['files'].items():
    if (ROOT / path).exists() and sha(ROOT / path) == expected:
        continue
    original = subprocess.check_output(['git', 'show', ORIGINAL['commit'] + ':' + path], cwd=ROOT)
    assert hashlib.sha256(original).hexdigest() == expected, path
    original_fallback.append(path)
observer_delta = [path for path, value in ORIGINAL['files'].items() if FREEZE['files'].get(path) != value]
assert set(observer_delta) == set(recovery['observer_only_changed_paths'])
assert recovery['unchanged_compiled_inputs'] == len(ORIGINAL['files']) - len(observer_delta)
assert not (OUT / 'trusted-tools').exists() and not any((OUT / 'private-builds').iterdir())
env = read(OUT / 'process-environment.json')
assert not any(key.startswith('INCANT_') or key == 'PYTHONOPTIMIZE' for key in env)
summary = {'passed': True, 'kind': 'producer re-audit; independent final review separate', 'frozen_inputs': len(FREEZE['files']), 'immutable_git_fallbacks': fallback,
           'original_build_frozen_inputs': len(ORIGINAL['files']), 'original_build_git_fallbacks': original_fallback, 'observer_only_changes_between_frozen_builds': observer_delta,
           'shipping_commands': len(commands), 'original_observer_refusals': 1, 'failed_infrastructure_caller_suites': 1, 'fresh_remaining_archives': 3, 'caller_suites': 4,
           'reused_compiler_output': recovery, 'records': records}
(OUT / 'producer-reaudit.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
