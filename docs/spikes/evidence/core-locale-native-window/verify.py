#!/usr/bin/env python3
"""Offline evidence replay; never starts an engine, compiler, CUA or GPU."""
import argparse
import base64
import gzip
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import re
import struct
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(manifest['format'] == 'incant-core-locale-native-window-evidence-v1', 'Manifest format')
    packed = (HERE / 'records.json.gz').read_bytes()
    require(sha(packed) == manifest['archive_sha256'], 'Archive hash')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(16 * 1024**2 + 1)
    require(len(raw) <= 16 * 1024**2, 'Finite archive')
    archive = json.loads(raw)
    data = {}
    for row, expected in zip(archive['records'], manifest['records'], strict=True):
        name = row['name']; path = PurePosixPath(name)
        require(not path.is_absolute() and '..' not in path.parts and name not in data, 'Unique relative record')
        require({k: row[k] for k in ('name', 'original', 'bytes', 'sha256')} == expected, 'Record inventory')
        value = base64.b64decode(row['base64'], validate=True)
        require(len(value) == row['bytes'] and sha(value) == row['sha256'], 'Record bytes ' + name)
        if args.originals:
            require(Path(row['original']).read_bytes() == value, 'Retained original ' + name)
        data[name] = value
    read = lambda name: json.loads(data[name])
    for version, source in [('001', manifest['producer_source']), ('002', manifest['runner_source'])]:
        frozen = read('source-' + version + '/freeze.json')
        require(frozen['source_sha'] == source, 'Frozen source identity')
        for row in frozen['records']:
            stored = 'source-' + version + '/files/' + row['path']
            if stored in data:
                require(sha(data[stored]) == row['sha256'], 'Archived source bytes')
            if args.originals:
                require(sha(Path(row['copy']).read_bytes()) == row['sha256'], 'Retained frozen source ' + row['path'])
    tests = read('python-tests-003.json')
    require(tests['exit_code'] == 0 and tests['raw_log_sha256'] == sha(data['python-tests-003.raw.log'])
            and re.search(rb'Ran 18 tests\b', data['python-tests-003.raw.log']) and b'\nOK\n' in data['python-tests-003.raw.log'], 'Eighteen pure controls')
    for name in ('historical-schema-replay-failure-001', 'python-schema-order-failure-001'):
        require(read(name + '.json')['exit_code'] != 0 and b'Exact successful observation plan' in data[name + '.raw.log'], 'Original schema-order refusals retained')
    author = read('ar/author-001/authoring/receipt.json')
    require(author['locale'] == 'ar' and author['state'] == 'playing' and author['revision'] == 32
            and author['rpc_calls'] == 46 and author['undo_redo_reopen'] is True and author['validation']['valid'] is True, 'Actual Arabic public authoring/history')
    require(author['binary_sha256'] == manifest['historical_headless_sha256'], 'Historical authoring executable identity')
    exported = read('ar/export-001/result.json'); package = read('ar/shipping-001/export.json')
    require(exported['passed'] is True and exported['source_sha'] == manifest['producer_source']
            and exported['remaining_private_entries'] == [] and exported['authoring_unchanged'] is True, 'Successful retired export')
    require(exported['export_receipt_sha256'] == sha(data['ar/shipping-001/export.json'])
            and package['player_sha256'] == exported['player_sha256'] == manifest['player_sha256'], 'Player/export binding')
    require(package['mode'] == 'window' and package['target'] == 'aarch64-apple-darwin'
            and package['physics_mode'] == 'deterministic' and package['startup_source_parsing'] is False
            and package['build_storage_removed_before_startup'] is True and package['startup']['tick'] == 1
            and package['startup']['render_input']['gpu_submitted'] is False and 'profiling' not in package, 'Ordinary source-free shipping startup')
    require(exported['node'] == 'v22.23.3' and exported['sdk_version'] == '27.0'
            and exported['compiler'].startswith('rustc 1.99.0 '), 'Actual selected tools')
    calls = read('ar/export-001/managed-calls.json')
    require(len(calls) == 18 and all(r['managed_call_finished'] is True and r['exit_code'] == 0 for r in calls), 'Eighteen owned export calls')
    external_logs = {row['path']: row for row in manifest['external_logs']}
    for call in calls:
        for stream in ('stdout', 'stderr'):
            captured = call[stream]
            if 'path' not in captured:
                require(captured == {'merged_into': 'stdout'}, 'Preserved merged stream')
                continue
            name = 'ar/export-001/' + Path(captured['path']).name
            if name in data:
                require(sha(data[name]) == captured['sha256'] and len(data[name]) == captured['bytes'], 'Raw managed output binding')
            else:
                retained = external_logs[captured['path']]
                require(all(retained[k] == captured[k] for k in ('bytes', 'sha256')), 'Large raw output identity')
    baseline = read('ar/baseline-001/result.json')
    require(baseline['passed'] is True and baseline['managed_calls_finished'] is True
            and baseline['player_sha256'] == manifest['player_sha256'], 'Same-package CPU baseline')
    recordings = read('ar/recordings-001/recordings.json')
    require(recordings['locale'] == 'ar' and recordings['project_sha256'] == author['project_sha256'] == package['project_sha256']
            and recordings['locale_settings'] == {'locale': 'ar', 'fallbacks': [], 'pseudo': False}, 'Arabic whole-scene locale identity')
    reports = {}
    for row, (name, host, tick) in zip(recordings['cases'], [('lose-short', 900, 900), ('win', 2156, 2156), ('lose-restart-win', 3056, 2156)], strict=True):
        require((row['name'], row['host_steps'], row['simulation_tick']) == (name, host, tick), 'Unchanged route clocks')
        report = read('ar/baseline-001/' + name + '.json'); reports[name] = report
        require(report == read('ar/baseline-001/' + name + '.stdout.raw.log')
                and integer(report['tick']) and report['tick'] == tick
                and integer(report['frames']) and report['frames'] == host * 800, 'Raw actual CPU report/PCM clocks')
        observed = report['observations']; plan = read('ar/recordings-001/' + name + '-observe.json')
        require(observed['passed'] is True and set(observed['entities']) == set(plan['entities']), 'Complete observed entities')
        require(len(observed['checks']) == len(plan['checks']) > 0, 'No discarded checks')
        for actual, expected in zip(observed['checks'], sorted(plan['checks'], key=lambda c: (c['tick'], c['name'])), strict=True):
            require(all(actual[k] == expected[k] for k in ('name', 'tick', 'expect')) and actual['passed'] is True
                    and actual['actual'] is expected['expect']['value'], 'Exact ordered actual observation outcome')
        require(report['logs']['dropped'] == 0 and report['render_input']['mode'] == 'cpu-only'
                and report['render_input']['gpu_submitted'] is False, 'Actual CPU scene input')
        ui = report['render_input']['ui']
        require(ui['widgets'] == 13 and integer(ui['glyphs'], 1) and ui['sources'] == row['labels'], 'Exact canonical localized HUD')
        pcm = next(v for v in manifest['retained_inputs'] if v['path'].endswith('/ar/baseline-001/' + name + '.pcm'))
        require(pcm['bytes'] == host * 6400 and pcm['sha256'] == report['pcm_sha256'], 'PCM/report identity')
    reset = reports['lose-restart-win']; win = reports['win']
    require(reset['session'] == {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156,
            'restarts': [{'host_step': 900, 'completed_generation': 1, 'simulation_tick': 900, 'output_frames': '720000'}]}, 'Whole-session restart')
    require(all(reset[k] == win[k] for k in ('logs', 'render_input', 'flipbooks', 'sources'))
            and reset['observations']['entities'] == win['observations']['entities'], 'Fresh winning generation parity')
    for suffix in ('001', '002'):
        prefix = 'ar/window-' + suffix + '/'
        result = read(prefix + 'result.json'); ledger = read(prefix + 'processes.json'); ready = read(prefix + 'owned-live.json')
        closeout = read(prefix + 'closeout.json')
        require(result['managed_calls_finished'] is True and ledger[0]['managed_call_finished'] is True
                and ready['pid'] == ready['pgid'] == closeout['owned_pid'], 'Owned native child identity/cleanup')
        events = [json.loads(line) for line in data[prefix + 'eligibility.jsonl'].splitlines()]
        require(events[0]['format'] == 'incant-window-state' and events[0]['max_records'] == 512
                and len(events) <= 512 and len(data[prefix + 'eligibility.jsonl']) <= 512 * 1024, 'Bounded durable eligibility log')
        if suffix == '001':
            native = read(prefix + 'lose-restart-win.json')
            require(result['passed'] is True and native == read(prefix + 'lose-restart-win.stdout.raw.log')
                    and native['mode'] == 'native-window' and native['backend'] == 'Metal' and native['on_screen'] is True, 'Actual complete native window')
            require(native['host_steps'] == 3056 and native['tick'] == 2156 and native['observations'] == reset['observations'], 'Same-package native/CPU parity')
            require(native['session'] == {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156, 'accepted_frames': 3058}
                    and native['accepted_frames'] == 2157 and native['presentation_refreshes'] == 2, 'Replacement vs cumulative frame counters')
            require(native['input_recording']['completed'] is True and native['input_recording']['sha256'] == recordings['cases'][2]['recording_sha256']
                    and native['input_recording']['native_focus_and_surface_required'] is True, 'Actual physical replay')
            require(native['ui']['widgets'] == 7 and native['ui']['glyph_quads'] == 82 and native['ui']['viewport'] == [320, 240]
                    and native['ui']['scale_factor'] == 2 and events[-1]['kind'] == 'final'
                    and events[-1]['host_steps'] == 3056, 'Actual narrow Retina UI/count contract')
            require(closeout['appearance_review'] is False and closeout['owned_pid_independently_absent'] is True, 'No first-window appearance acceptance')
        else:
            require(result['passed'] is False and 'TimeoutExpired' in result['error'] and ', 120)' in result['error'], 'Original visual timeout retained')
            last = events[-1]
            require(last['kind'] == 'snapshot' and last['state']['host_steps'] == 1133
                    and last['state']['generation'] == 2 and last['state']['world_tick'] == 233
                    and last['state']['cached_focused'] is False and last['state']['native_focused'] is False
                    and last['state']['simulation_suspended'] is True and last['surface_deferrals'] == 0, 'Retained focus-held partial state')
            require(not data[prefix + 'lose-restart-win.stdout.raw.log'] and not data[prefix + 'lose-restart-win.stderr.raw.log']
                    and closeout['final_report_present'] is False and closeout['owned_pid_and_bundle_processes_independently_absent'] is True, 'No fabricated final report, actual cleanup')
            visual = read(prefix + 'root-visual-review.json')
            require(visual['actual_screenshots'] == 5 and 'no saved PNG' in visual['screenshots_storage']
                    and closeout['RTL_health_order_accepted'] is False and closeout['full_visual_route_pass'] is False, 'Partial actual tool pixels, open appearance gates')
    cjk = read('zh-Hans/author-001/authoring/receipt.json')
    require(cjk['locale'] == 'zh-Hans' and cjk['undo_redo_reopen'] is True and cjk['validation']['valid'] is True
            and cjk['revision'] == 32 and cjk['rpc_calls'] == 46 and '按 R 或 Start' in cjk['localized_text']['zh-Hans']['hud.prompt'], 'Actual CJK authoring only')
    require(all(manifest['scope'][k] is False for k in ('complete_appearance', 'normal_size_visual_review', 'RTL_health_order',
            'audible_quality', 'performance_gate', 'CJK_export_or_window', 'pseudo_qualification')), 'Open scope preserved')
    if args.originals:
        for row in manifest['external_logs']:
            raw = Path(row['path']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Retained large raw compiler output')
    if args.binaries:
        for row in manifest['retained_inputs']:
            raw = Path(row['path']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Retained runtime/PCM identity')
        pcm = {name: Path(next(r['path'] for r in manifest['retained_inputs'] if r['path'].endswith('/ar/baseline-001/' + name + '.pcm'))).read_bytes()
               for name in reports}
        require(pcm['lose-restart-win'] == pcm['lose-short'] + pcm['win'] and not any(pcm['lose-short'][-60 * 6400:]), 'Exact actual PCM reset/settled loss cue')
        require(all(math.isfinite(v[0]) for raw in pcm.values() for v in struct.iter_unpack('<f', raw)), 'Finite actual PCM')
    if args.git_source:
        require(args.source_root is not None, 'Git source requires repository path')
        for row in read('source-002/freeze.json')['records']:
            ref = args.git_source if row['path'] in manifest['tool_paths'] else manifest['public_base']
            actual = subprocess.check_output(['git', 'show', ref + ':' + row['path']], cwd=args.source_root)
            require(sha(actual) == row['sha256'], 'Accepted tools/public base source ' + row['path'])
    print('PASS:', len(data), 'records;18 Python;Arabic author/export/3CPU/complete numeric window;retained visual timeout/5 partial tool images;CJK author only. No full appearance or performance gate.')


if __name__ == '__main__':
    main()
