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
    require(manifest['format'] == 'incant-native-window-eligibility-evidence-v1', 'Manifest format')
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
    for key in ('full_native_route_verified', 'diagnostic_window_run', 'performance_gate_passed', 'filesystem_pixel_archive'):
        require(manifest['scope'][key] is False, 'Open boundary '+key)
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
    print(f"PASS {len(data)} records; CPU90+113/lint2; planarCPU781/201/221; two90s timeouts retained; diagnostic window unrun")


if __name__ == '__main__':
    main()
