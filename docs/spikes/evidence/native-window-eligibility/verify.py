#!/usr/bin/env python3
"""Offline receipt replay; no build, player, device, CUA or project mutation."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE/'manifest.json').read_text())
    require(manifest['format'] == 'incant-native-window-eligibility-evidence-v2', 'Manifest format')
    packed = (HERE/'records.json.gz').read_bytes()
    require(sha(packed) == manifest['archive_sha256'], 'Archive hash')
    with gzip.open(HERE/'records.json.gz', 'rb') as stream:
        unpacked = stream.read(32*1024**2+1)
    require(len(unpacked) <= 32*1024**2, 'Finite archive limit')
    archived = json.loads(unpacked)
    data = {}
    for row, expected in zip(archived['records'], manifest['records'], strict=True):
        name = row['name']
        path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and name not in data, 'Unique relative record')
        require({k: row[k] for k in ('name', 'original', 'bytes', 'sha256')} == expected, 'Record inventory')
        raw = base64.b64decode(row['base64'], validate=True)
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Record bytes')
        if args.originals:
            require(Path(row['original']).read_bytes() == raw, 'Retained original '+name)
        data[name] = raw
    read = lambda name: json.loads(data[name])
    cpu = read('eligibility-cpu-001/cpu-source001-results.json')
    launch = read('eligibility-cpu-001/cpu-launch-001.json')
    proposal = read('eligibility-cpu-proposal-001.json')
    require(cpu['source_sha'] == launch['source_sha'] == proposal['source_sha'] == manifest['eligibility_source'], 'CPU source')
    require(launch['proposal_sha256'] == sha(data['eligibility-cpu-proposal-001.json'])
            and launch['supervisor_sha256'] == sha(data['eligibility-cpu-supervisor-001.py']), 'CPU protocol binding')
    require(proposal['freeze_sha256'] == sha(data['eligibility-source-001/freeze.json']), 'CPU frozen source binding')
    frozen = read('eligibility-source-001/freeze.json')
    require(frozen['source_sha'] == manifest['eligibility_source'] and frozen['clean'] is True, 'Producing source freeze')
    for row in frozen['records']:
        original = Path(row['copy'])
        name = 'eligibility-source-001/files/'+row['path']
        require(len(data[name]) == row['bytes'] and sha(data[name]) == row['sha256'], 'Copied frozen source '+row['path'])
    require(all(not v for v in launch['ambient_override_values'].values()), 'Ambient refusal')
    require(all(r['exit_code'] == 0 for r in launch['identities'].values()), 'Actual tool/host probes')
    require(len(cpu['results']) == 4, 'Four CPU stages')
    for index, row in enumerate(cpu['results']):
        require(row['exit_code'] == 0 and row['reaped'] is True and row['argv'] == proposal['commands'][index]['argv'], 'CPU command/result/reaping')
        raw = data['eligibility-cpu-001/'+Path(row['log']).name]
        require(sha(raw) == row['log_sha256'], 'CPU raw log binding')
        expected = {0: 90, 2: 113}.get(index)
        if expected:
            require(re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed', raw) == [(str(expected).encode(), b'0')], 'Executed CPU totals')
            require(len(re.findall(rb'^test [^\n]+state_log[^\n]+ \.\.\. ok$', raw, re.M)) == 8, 'Eight actual diagnostic controls')
        else:
            require('--all-targets' in row['argv'] and row['argv'][-2:] == ['-D', 'warnings'], 'Strict all-target lint')
    require(len(cpu['copied_artifacts']) == 2 and cpu['copied_artifacts'] == manifest['CPU_test_artifacts'], 'Copied test identities')
    for receipt, log in [('python-source005-result.json', 'python-source005.raw.log'),
                         ('python-eligibility-001.json', 'python-eligibility-001.raw.log')]:
        result = read(receipt)
        require(result['exit_code'] == 0 and result['log_sha256'] == sha(data[log])
                and re.search(rb'Ran 10 tests\b', data[log]) and b'\nOK\n' in data[log], 'Ten Python controls '+receipt)
    author = read('author-001/authoring/result.json')
    require(author['passed'] is True and author['undo_redo_reopen'] is True
            and [t['commands'] for t in author['transactions']] == [3, 2, 13, 6], 'Real command authoring/history')
    failed = read('export-001/result.json')
    require(failed['passed'] is False and 'Only retained output files' in failed['error']
            and failed['managed_calls_finished'] and failed['remaining_private_entries'] == [], 'Original supervisor refusal retained')
    exported = read('export-002/result.json')
    package = read('shipping-002/export.json')
    require(exported['passed'] is True and exported['authoring_unchanged'] and exported['remaining_private_entries'] == []
            and exported['export_receipt_sha256'] == sha(data['shipping-002/export.json']), 'Fresh successful export')
    require(exported['source_sha'] == manifest['planar_source'] and exported['player_sha256'] == package['player_sha256']
            == manifest['historical_shipping_player_sha256'], 'Historical source/package identity')
    require(package['mode'] == 'window' and package['build_storage_removed_before_startup'] is True
            and package['startup']['tick'] == 1 and package['startup']['render_input']['gpu_submitted'] is False, 'Inherent CPU startup only')
    baseline = read('baseline-001/result.json')
    require(baseline['passed'] is True and baseline['owned_children_reaped'] and baseline['player_sha256'] == package['player_sha256'], 'Same-package CPU baseline')
    routes = [('keyboard-win', 781, 'Sailing home with 3 pearls'), ('urchin-loss', 201, 'Stung by a sea urchin'), ('shore-and-cliff', 221, 'Pearls 0 of 3')]
    checks, pcm_bytes = 0, 0
    binary_inputs = {row['path']: row for row in manifest['retained_binary_inputs']}
    baseline_closeout = read('baseline-001/closeout.json')
    for name, ticks, status in routes:
        report = read('baseline-001/'+name+'.json')
        require(report == read('baseline-001/'+name+'.stdout.raw.log'), 'Raw CPU report')
        require(report['tick'] == ticks and report['frames'] == ticks*800, 'Exact fixed CPU/audio clocks')
        observations = report['observations']
        require(observations['passed'] is True and all(r['passed'] is True for r in observations['checks']), 'Actual observation checks')
        checks += len(observations['checks'])
        pcm_bytes += ticks*800*2*4
        retained = next(r for r in baseline_closeout['routes'] if r['name'] == name)
        pcm = next(row for path, row in binary_inputs.items() if path.endswith('/baseline-001/'+name+'.pcm'))
        require(retained['pcm_sha256'] == report['pcm_sha256'] == pcm['sha256']
                and retained['pcm_bytes'] == pcm['bytes'] == ticks*800*2*4, 'Raw PCM/report identity')
        require(report['render_input']['gpu_submitted'] is False
                and report['render_input']['ui']['sources']['01PSHD00000000000000000002'] == status, 'Canonical CPU HUD and no GPU')
    require((checks, pcm_bytes) == (11, 7699200), 'Eleven checks and exact PCM byte total')
    for suffix, pid in [('001', 85471), ('002', 87531)]:
        prefix = 'window-keyboard-win-'+suffix+'/'
        result = read(prefix+'result.json')
        closeout = read(prefix+'closeout.json')
        ready = read(prefix+'readiness.json')
        require(result['passed'] is False and 'TimeoutExpired' in result['error'] and ', 90)' in result['error'], 'Both unchanged actual timeouts')
        require(closeout['owned_pid_and_pgid'] == pid and closeout['owned_group_absent'] is True
                and closeout['bundle_processes_at_closeout'] == [] and closeout['all_bound_inputs_unchanged'], 'Actual cleanup/post-identities')
        require(ready['monitor_joined'] is True and ready['records'][0]['pid'] == pid, 'Owned live readiness')
        require(not data[prefix+'keyboard-win.stdout.raw.log'] and not data[prefix+'keyboard-win.stderr.raw.log'], 'No fabricated final report')
    require(read('window-keyboard-win-002/closeout.json')['no_CUA_or_activation_requested'] is True, 'No-CUA successor distinction')
    cua = read('window-keyboard-win-001/root-cua-observation.json')
    require(cua['owned_pid'] == 85471 and cua['screenshots']['count'] == 4
            and 'no filesystem PNG' in cua['scope'], 'Actual prior tool pixels remain separate')
    for key in ('native_loss_and_shore_routes_verified', 'performance_gate_passed', 'filesystem_pixel_archive'):
        require(manifest['scope'][key] is False, 'Open boundary '+key)
    require(manifest['scope']['native_win_route_verified'] is True
            and manifest['scope']['diagnostic_window_run'] is True, 'Actual diagnostic win scope')
    exported = read('export-003/result.json')
    fresh = read('shipping-003/export.json')
    closeout = read('export-003/closeout.json')
    require(exported['passed'] is True and exported['source_sha'] == manifest['eligibility_source']
            and exported['authoring_unchanged'] and exported['remaining_private_entries'] == [], 'Fresh diagnostic-source export')
    require(exported['player_sha256'] == fresh['player_sha256'] == manifest['shipping_player_sha256']
            and exported['export_receipt_sha256'] == sha(data['shipping-003/export.json']), 'Fresh player identity')
    require(closeout['all_managed_calls_finished'] and closeout['initial_owned_processes_and_cargo_group_absent']
            and closeout['frozen_source_rows_verified'] == 33 and closeout['authoring_files_verified'] == 28
            and closeout['remaining_private_entries'] == [], 'Fresh export postchecks/cleanup')
    require(fresh['build_storage_removed_before_startup'] and fresh['startup']['tick'] == 1
            and fresh['startup']['render_input']['gpu_submitted'] is False, 'Fresh CPU one-tick startup only')
    require(package['data']['files'] == fresh['data']['files'], 'All runtime-data hashes unchanged')
    baseline2 = read('baseline-002/result.json')
    require(baseline2['passed'] and baseline2['owned_children_reaped']
            and baseline2['player_sha256'] == fresh['player_sha256'], 'New same-package CPU baseline')
    for name, ticks, _ in routes:
        report = read('baseline-002/'+name+'.json')
        require(report == read('baseline-002/'+name+'.stdout.raw.log') == read('baseline-001/'+name+'.json'), 'Historical logical parity')
        pcm = next(row for path, row in binary_inputs.items() if path.endswith('/baseline-002/'+name+'.pcm'))
        require(pcm['bytes'] == ticks*800*2*4 and pcm['sha256'] == report['pcm_sha256'], 'New PCM/report identity')
    require(all(r['reaped'] is True and r['exit_code'] == 0 for r in read('baseline-002/processes.json')), 'Three CPU children reaped')
    controls = read('python-widget-contract-002.json')
    require(controls['source_sha'] == manifest['runner_source'] and controls['exit_code'] == 0
            and controls['log_sha256'] == sha(data['python-widget-contract-002.raw.log'])
            and b'Ran 11 tests' in data['python-widget-contract-002.raw.log']
            and b'\nOK\n' in data['python-widget-contract-002.raw.log'], 'Eleven final runner controls')
    repair = read('widget-contract-source-001/freeze.json')
    require(repair['source_sha'] == manifest['runner_source'] and repair['player_producing_source'] == manifest['eligibility_source']
            and repair['tool_only_delta_paths'] == ['tools/probes/planar-window.py', 'tools/tests/test_planar_window.py'], 'Tool-only successor')
    for row in repair['records']:
        if row['path'].startswith('tools/') and row['path'] in repair['tool_only_delta_paths']:
            require(sha(data['widget-contract-source-001/files/'+row['path']]) == row['sha256'], 'Final tool source')
    replay = read('widget-contract-replay-002.json')
    require(replay['passed'] and replay['kind'].startswith('Retained raw report replay only')
            and replay['source_sha'] == manifest['runner_source'], 'Replay kept distinct from execution')
    reports = []
    logs = []
    for suffix, pid, passed in [('001', 22070, False), ('002', 25657, True)]:
        prefix = 'window-eligibility-keyboard-win-'+suffix+'/'
        outcome = read(prefix+'result.json')
        native = read(prefix+'keyboard-win.stdout.raw.log')
        ended = read(prefix+'closeout.json')
        ledger = read(prefix+'processes.json')
        ready = read(prefix+'readiness.json')
        require(outcome['passed'] is passed and ended['owned_pid_and_pgid'] == pid
                and ended['owned_group_absent'] and ended['bundle_processes_at_closeout'] == []
                and ended['all_bound_inputs_unchanged'], 'Actual result/cleanup '+suffix)
        require(len(ledger) == 1 and ledger[0]['exit_code'] == 0 and ledger[0]['reaped']
                and ledger[0]['logs'][0]['sha256'] == sha(data[prefix+'keyboard-win.stdout.raw.log'])
                and ready['monitor_joined'] and not ready['errors'] and ready['records'][0]['pid'] == pid, 'Native ownership '+suffix)
        require(native['mode'] == 'native-window' and native['backend'] == 'Metal' and native['on_screen'] is True
                and type(native['host_steps']) is int and native['host_steps'] == native['tick'] == 781
                and native['accepted_frames'] == 782 and type(native['ui']['widgets']) is int and native['ui']['widgets'] == 2
                and native['ui']['glyph_quads'] == 22 and native['observations'] == read('baseline-002/keyboard-win.json')['observations'], 'Actual native route/submission/parity '+suffix)
        recording = native['input_recording']
        require(recording['completed'] and recording['native_focus_and_surface_required']
                and recording['physical_devices'] is False and recording['accepted_host_steps'] == recording['requested_host_steps'] == 781
                and recording['sha256'] == '3bee29c5cdf91b7bf634bc10d8d56c285b112f13f2822dff09c43dea0a3165e8', 'Recorded physical input '+suffix)
        raw = data[prefix+'eligibility.jsonl']
        state = [json.loads(line) for line in raw.splitlines()]
        require(len(raw) == outcome['window_state_log']['bytes'] and sha(raw) == outcome['window_state_log']['sha256']
                and len(raw) <= 512*1024 and len(state) <= 512 and state[0]['format'] == 'incant-window-state'
                and state[-1]['kind'] == 'final' and state[-1]['host_steps'] == 781 and state[-1]['returned_report'] is True, 'Actual bounded completed log '+suffix)
        require(not any(r['kind'] == 'truncated' for r in state), 'No hidden truncation')
        if passed:
            require(native == read(prefix+'keyboard-win.json') and outcome['owned_children_reaped'], 'Fresh repaired runner success')
        else:
            require('Native scene/HUD submissions' in outcome['error'], 'Original exact widget refusal preserved')
        reports.append(native); logs.append(state)
    first = [r for r in logs[0] if r['kind'] == 'snapshot']
    require(len(logs[0]) == 16 and first[0]['state']['host_steps'] == 0 and not first[0]['state']['cached_focused']
            and first[-1]['focus_gained_events'] == 1 and first[-1]['focus_lost_events'] == 0
            and logs[0][-1]['elapsed_us'] == 14151943, 'Autonomous first diagnostic trajectory')
    second = [r for r in logs[1] if r['kind'] == 'snapshot']
    paused = [r for r in second if r['state']['host_steps'] == 772 and not r['state']['cached_focused']]
    require(len(logs[1]) == 49 and paused and all(r['state']['simulation_suspended'] for r in paused)
            and second[-1]['focus_gained_events'] == 2 and second[-1]['focus_lost_events'] == 1
            and second[-1]['eligible_by_cached_flags'] is True and logs[1][-1]['elapsed_us'] == 45608917
            and reports[1]['surface_skipped'] == 1 and second[-1]['surface_deferrals'] == 1, 'Actual pause/normal activation/completion trajectory')
    cua = read('window-eligibility-keyboard-win-002/root-cua-observation.json')
    require(cua['owned_pid'] == 25657 and cua['exec_session'] == 38831 and cua['exec_exit'] == 0
            and cua['pid_and_group_absent'] and cua['captures']['count'] == 3
            and cua['result_sha256'] == sha(data['window-eligibility-keyboard-win-002/result.json'])
            and cua['diagnostic_sha256'] == sha(data['window-eligibility-keyboard-win-002/eligibility.jsonl']), 'Scoped actual root tool images')
    for row in manifest['git_input_inventory']:
        if args.source_root:
            require(sha((args.source_root/row['path']).read_bytes()) == row['sha256'], 'Integrated source bytes '+row['path'])
        if args.git_source:
            raw = subprocess.check_output(['git', 'show', args.git_source+':'+row['path']], cwd=args.source_root or Path.cwd(), timeout=10)
            require(sha(raw) == row['sha256'], 'Git source bytes '+row['path'])
    if args.binaries:
        for row in manifest['retained_binary_inputs']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'], 'Retained binary size')
            h = hashlib.sha256()
            with path.open('rb') as stream:
                for block in iter(lambda: stream.read(1024*1024), b''):
                    h.update(block)
            require(h.hexdigest() == row['sha256'], 'Retained binary/PCM identity')
    print(f"PASS {len(data)} records; CPU90+113/lint2; freshCPU781/201/221 exactPCM; original2timeouts/widgetrefusal retained; final native781/782frames; scoped3toolimages")


if __name__ == '__main__':
    main()
