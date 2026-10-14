#!/usr/bin/env python3
"""Replay retained data only; never run the player or archived scripts."""
import argparse
import base64
from collections import Counter
import gzip
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def replay(records, manifest):
    read = lambda name: json.loads(records[name])
    cpu = read('current/cpu-001/result.json')
    require(cpu['outcome'] == 'passed' and cpu['source'] == manifest['source'], 'CPU source/result')
    for row in cpu['commands']:
        require(row['exit_code'] == 0 and row['managed_returned_after_cleanup'], 'CPU cleanup')
        for kind in ('stdout', 'stderr'):
            require(sha(records[f"current/cpu-001/{row['name']}.{kind}"]) == row[kind + '_sha256'], 'CPU log')
    counts = []
    for name in ('player-observer-tests', 'player-normal-tests', 'audio-policy-tests'):
        raw = records[f'current/cpu-001/{name}.stdout'].decode()
        results = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw)
        require(results and all(failed == ignored == '0' for _, failed, ignored in results), 'Actual CPU assertions')
        counts.append(sum(int(passed) for passed, _, _ in results))
    require(counts == [105, 87, 39], 'Actual CPU counts')
    build = read('current/private-build-001/diagnostic-build.json')
    attempt = read('current/private-build-001/attempt.json')
    source = read('current/live-packet-001/source-manifest.json')
    plan = read('current/live-packet-001/build-plan.json')
    require(build['source'] == plan['source'] == source['source'] == manifest['source'], 'Producing source')
    require(len(source['files']) == 4514 and attempt['admission']['authored_files'] == 74, 'Input inventory')
    require(build['plan_sha256'] == attempt['plan_sha256'] == sha(records['current/live-packet-001/build-plan.json']), 'Build plan')
    require(build['source_manifest']['sha256'] == sha(records['current/live-packet-001/source-manifest.json']), 'Source inventory binding')
    require(attempt['status'] == 'complete' and not attempt['scratch_retained']
            and build['build_storage_removed_before_startup'], 'Scratch cleanup before startup')
    children = read('current/private-build-001/children.json')
    require(len(children) == 22 and all(c['exit_code'] == 0 and c['managed_returned_after_cleanup'] for c in children), 'Managed children')
    require(build['diagnostic_only'] and not build['eligible_for_distribution']
            and not build['eligible_for_performance_gate'] and build['window_launches'] == 0, 'Build scope')
    artifacts = {r['path']: r for r in manifest['external_artifacts']}
    for name, digest in build['artifacts'].items():
        key = 'current/private-build-001/' + name
        if key in records:
            require(sha(records[key]) == digest, 'Retained build artifact')
        else:
            path = str(Path(manifest['evidence_root']) / 'private-build-001' / name)
            require(artifacts[path]['sha256'] == digest, 'External build artifact binding')
    additions = {'incant_game_player': 'native-presentations', 'wgpu-hal': 'diagnostic-presentations',
                 'objc2-metal': 'objc2-core-foundation'}
    require(set(build['graph_delta']) == set(additions), 'Exact three feature recipients')
    for package, feature in additions.items():
        delta = build['graph_delta'][package]
        require(set(delta['after']) == set(delta['before']) | {feature}, 'Feature addition')
    units = []
    for line in records['current/private-build-001/cargo.jsonl'].splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get('reason') == 'compiler-artifact':
            name = item['target']['name'].replace('_', '-')
            package = {'incant-game-player': 'incant_game_player'}.get(name, name)
            if package in additions:
                units.append({'package': package, 'kind': item['target']['kind'][0], 'features': item['features']})
    require(sorted(units, key=lambda r: (r['package'], r['kind'])) == build['observer_compiler_units']['target_units'], 'Actual compiler recipients')
    require(build['physics_mode'] == 'fast' and build['codegen']['incant_game_player']['lto'] == 'fat', 'Shipping selections')
    for row in build['codegen'].values():
        require(row['opt-level'] == '3' and row['codegen-units'] == '1' and row['panic'] == 'abort', 'Codegen')
    baseline = read('baseline/export.json')
    require(build['startup'] == baseline['startup'] and build['startup']['tick'] == 1, 'Offline startup parity')
    require(build['data'] == baseline['data'] and len(build['data']['files']) == 28, 'Cooked data parity')
    before, after = read('baseline/project-build.json'), read('current/private-build-001/project-build.json')
    require(before['compiler'] == after['compiler'], 'Compiler payload parity')
    a, b = before['script'].copy(), after['script'].copy()
    require(a.pop('build_id') != b.pop('build_id') and a == b, 'Honest new identity, unchanged script payload')
    require(build['player_sha256'] == manifest['player_sha256'], 'Player identity')
    first, second = read('historical/window-001/result.json'), read('historical/window-002/result.json')
    require(first['status'] == second['status'] == 'failed' and 'TimeoutExpired' in first['error'], 'Prior failures preserved')
    require(records['historical/window-002/stderr.log'] == b'Error: Audio(Backend("A buffer underrun or overrun occurred."))\n', 'Actual historical Xrun failure')
    require(read('historical/window-002/presentations.json')['finalization']['accepted_host_steps'] == 1444, 'Historical incomplete replay')
    result = read('current/window-001/result.json')
    require(result['status'] == 'complete' and result['child_exit'] == result['exit_code'] == 0
            and result['managed_returned_after_cleanup'] and not result['performance_acceptance'], 'Live completion')
    require(result['plan_sha256'] == sha(records['current/live-controller-001/plan.json'])
            and result['runner_sha256'] == sha(records['current/live-controller-001/run.py']), 'Executed live source')
    live_plan = read('current/live-controller-001/plan.json')
    require(live_plan['build_identity'] == sha(records['current/private-build-001/diagnostic-build.json'])
            and live_plan['player_sha256'] == manifest['player_sha256'], 'Live build identity')
    for name, artifact in result['artifacts'].items():
        raw = records['current/window-001/' + name]
        require(sha(raw) == artifact['sha256'] and len(raw) == artifact['bytes'], 'Live artifact')
    report = read('current/window-001/stdout.json')
    require(report['mode'] == 'native-window' and report['backend'] == 'Metal'
            and report['adapter'] == 'Apple M5 Pro' and report['on_screen'], 'Actual device route')
    require(report['tick'] == report['host_steps'] == 2156 and report['accepted_frames'] == 2157, 'Complete host replay')
    require(report['profile']['completed_frames'] == 2156 and report['profile']['failed_frames'] == 0, 'Profile coverage')
    require(report['observations'] == read('baseline/gameplay.json')['observations']
            and report['observations']['passed'] and len(report['observations']['checks']) == 8
            and len(report['observations']['entities']) == 17, 'Full gameplay parity')
    recording = report['input_recording']
    require(recording['completed'] and recording['accepted_host_steps'] == 2156
            and not recording['physical_devices'] and recording['native_focus_and_surface_required'], 'Replay isolation')
    require(recording['sha256'] == sha(records['baseline/input.json']), 'Recording identity')
    require(report['audio_device'] == result['audio_device'] == {
        'scope': 'current-session-owner', 'glitch_free_output_verified': False,
        'diagnostics': {'xrun_notifications': 0, 'poll_limit_hits': 0, 'counters_saturated': False}}, 'Raw current-owner audio observation')
    sidecar = read('current/window-001/presentations.json')
    rows, host, final = sidecar['rows'], sidecar['host'], sidecar['finalization']
    attempts, segments = host['attempts'], host['segments']
    require(sidecar['diagnostic_only'] and not sidecar['display_gate'] and sidecar['capture_capacity'] == 4096, 'Observer bounds/scope')
    require(len(rows) == len(attempts) == 2157 and len(segments) == 6, 'Actual bounded rows')
    require([a['id'] for a in attempts] == list(range(1, 2158)), 'Attempt identities')
    require(attempts[0]['status'] == 'refresh_completed' and all(a['status'] == 'host_committed' for a in attempts[1:]), 'Distinct refresh/commit roles')
    outcomes = Counter(); positive = equal = 0; previous = None; ids = {}
    for row in rows:
        a, s = attempts[row['attempt'] - 1], segments[row['segment'] - 1]
        require(row['requested_host_step'] == a['requested_host_step']
                and row['surface_generation'] == s['surface_generation']
                and a['session_generation'] == s['session_generation'] and a['role'] == s['role'], 'Raw callback correlation')
        require(row['drawable_id'] > ids.get(row['surface_generation'], -1), 'Drawable order')
        ids[row['surface_generation']] = row['drawable_id']; outcomes[row['outcome']] += 1
        t = row['presented_seconds']
        if row['outcome'] == 'presented':
            require(isinstance(t, (int, float)) and math.isfinite(t) and t > 0, 'Positive timestamp')
        else:
            require(t is None, 'No inferred zero/missing timestamp')
        if previous and previous['segment'] == row['segment'] and previous['outcome'] == row['outcome'] == 'presented':
            delta = t - previous['presented_seconds']; require(delta >= 0, 'No timestamp reversal')
            positive += delta > 0; equal += delta == 0
        previous = row
    require(outcomes == {'presented': 2149, 'reported_zero': 8} and positive == 2142 and equal == 1, 'All raw outcomes and equality preserved')
    require(sidecar['coverage'] == {'abandoned': 0, 'acquired': 0, 'discarded': 0, 'invalid': 0,
        'missing': 0, 'overflow': 0, 'presented': 2149, 'protocol_errors': 0, 'rows': 2157, 'segments': 6, 'zero': 8}, 'Exact coverage')
    require(not host['exhausted'] and host['omitted_attempts'] == 0 and host['boundary_counts'][1] == 9, 'No metadata loss, focus retained')
    require(final['stage'] == 'window_finalized' and final['accepted_host_steps'] == 2156
            and final['recording_completed'] and final['returned_report'] and final['assertions_passed']
            and final['pending_at_last_drain_scan'] == 0 and final['window_error'] is None
            and final['unbind_error'] is None, 'Bounded finalization')
    analysis = read('current/window-001/coverage.json')
    require(analysis['coverage_limited_or_inconclusive'] and not analysis['performance_acceptance']
            and len(analysis['observed_positive_intervals']) == positive
            and analysis['timestamp_order_issues'] == [{'before': 1, 'after': 2, 'seconds': 0.0}], 'No filtered timing acceptance')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    archive = HERE / 'records.json.gz'
    require(archive.stat().st_size == manifest['archive_bytes'] and file_sha(archive) == manifest['archive_sha256'], 'Archive identity')
    require(0 < manifest['expanded_bytes'] <= 128 * 1024 * 1024, 'Expanded bound')
    with gzip.open(archive, 'rb') as stream:
        raw = stream.read(manifest['expanded_bytes'] + 1)
    require(len(raw) == manifest['expanded_bytes'], 'Exact expansion')
    blobs = {k: base64.b64decode(v, validate=True) for k, v in json.loads(raw).items()}
    require(set(blobs) == {r['sha256'] for r in manifest['records']}, 'Content inventory')
    records = {r['name']: blobs[r['sha256']] for r in manifest['records']}
    require(len(records) == len(manifest['records']), 'Unique names')
    for row in manifest['records']:
        require(sha(records[row['name']]) == row['sha256'] and len(records[row['name']]) == row['bytes'], 'Record identity')
        if args.originals:
            require(Path(row['original']).read_bytes() == records[row['name']], 'Original identity')
    if args.binaries:
        for row in manifest['external_artifacts']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'] and file_sha(path) == row['sha256'], 'External artifact')
    if args.git_source:
        for row in manifest['source_files']:
            result = subprocess.run(['git', 'show', manifest['source'] + ':' + row['path']],
                                    cwd=args.git_source, capture_output=True, timeout=10, check=True)
            require(sha(result.stdout) == row['sha256'], 'Producing Git source')
    replay(records, manifest)
    print(f"PASS: {len(records)} records; 2156-step gameplay parity; 2149 positive/8 zero/0 missing callbacks. "
          "One equal adjacent timestamp retained; no display-p95 or audio-quality claim.")


if __name__ == '__main__':
    main()
