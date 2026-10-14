#!/usr/bin/env python3
"""Read retained records only; never launch the player or execute archived code."""
import argparse
import base64
from collections import Counter
import gzip
import hashlib
import json
import math
from pathlib import Path
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
    cpu = read('player/qualify-004/result.json')
    require(cpu['outcome'] == 'passed' and cpu['source'] == manifest['qualified_source'], 'CPU identity')
    summaries = []
    for row in cpu['commands']:
        require(row['exit_code'] == 0 and row['managed_returned_after_cleanup'], 'CPU result/cleanup')
        for stream in ('stdout', 'stderr'):
            key = f"player/qualify-004/{row['name']}.{stream}"
            require(sha(records[key]) == row[stream + '_sha256'], 'CPU raw log binding')
        if 'test_summary' in row:
            summaries.append(row['test_summary']['passed'])
    require(summaries == [105, 87], 'Actual on/off test counts')
    plan = read('player/live-packet-002/build-plan.json')
    build = read('player/private-build-002/diagnostic-build.json')
    attempt = read('player/private-build-002/attempt.json')
    source = read('player/live-packet-002/source-manifest.json')
    require(build['source'] == plan['source'] == manifest['source'] == source['source'], 'Producing source')
    require(len(source['files']) == 4507 and attempt['admission']['authored_files'] == 74, 'Input inventory')
    require(build['plan_sha256'] == attempt['plan_sha256'] == sha(records['player/live-packet-002/build-plan.json']), 'Build plan binding')
    require(build['source_manifest']['sha256'] == sha(records['player/live-packet-002/source-manifest.json']), 'Source manifest')
    require(build['diagnostic_only'] and not build['eligible_for_distribution']
            and not build['eligible_for_performance_gate'] and build['window_launches'] == 0,
            'Private build scope')
    require(attempt['status'] == 'complete' and not attempt['scratch_retained']
            and build['build_storage_removed_before_startup'], 'Build/storage completion')
    children = read('player/private-build-002/children.json')
    require(len(children) == 22 and all(c['exit_code'] == 0 and c['managed_returned_after_cleanup'] for c in children), '22 managed children')
    for name, digest in build['artifacts'].items():
        key = 'player/private-build-002/' + name
        if key in records:
            require(sha(records[key]) == digest, 'Actual build artifact: ' + name)
    additions = {'incant_game_player': 'native-presentations', 'wgpu-hal': 'diagnostic-presentations',
                 'objc2-metal': 'objc2-core-foundation'}
    require(set(build['graph_delta']) == set(additions), 'Exactly three added feature recipients')
    for package, feature in additions.items():
        delta = build['graph_delta'][package]
        require(set(delta['after']) == set(delta['before']) | {feature}, 'Feature delta')
    units = []
    for line in records['player/private-build-002/cargo.jsonl'].splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get('reason') != 'compiler-artifact':
            continue
        name = item['target']['name'].replace('_', '-')
        package = {'incant-game-player': 'incant_game_player'}.get(name, name)
        if package in additions:
            units.append({'package': package, 'kind': item['target']['kind'][0], 'features': item['features']})
    expected = build['observer_compiler_units']['target_units']
    require(sorted(units, key=lambda r: (r['package'], r['kind'])) == expected, 'Actual target compiler feature rows')
    for row in build['codegen'].values():
        require(row['opt-level'] == '3' and row['codegen-units'] == '1' and row['panic'] == 'abort', 'Shipping codegen')
    require(build['codegen']['incant_game_player']['lto'] == 'fat' and build['physics_mode'] == 'fast', 'Fat LTO/fast physics')
    baseline = read('baseline/export.json')
    require(build['startup'] == baseline['startup'] and build['startup']['tick'] == 1, 'Full offline startup parity')
    require(build['data'] == baseline['data'], 'Exact baseline cooked-data inventory')
    before, after = read('baseline/project-build.json'), read('player/private-build-002/project-build.json')
    require(before['compiler'] == after['compiler'], 'Compiler payload identity')
    old_script, new_script = before['script'].copy(), after['script'].copy()
    require(old_script.pop('build_id') != new_script.pop('build_id') and old_script == new_script,
            'Honest changed build identity, unchanged script payload')
    require(len(build['data']['files']) == 28 and build['data']['bytes'] == 35365370, 'Cooked payload inventory')
    require(build['player_sha256'] == manifest['player_sha256'], 'Copied binary identity')
    # Preserve the original failed setup and the failed first live attempt.
    require(read('player/private-build-001/attempt.json')['status'] == 'failed', 'Build001 failure retained')
    first, second = read('player/window-001/result.json'), read('player/window-002/result.json')
    require(first['status'] == second['status'] == 'failed' and first['exit_code'] == second['exit_code'] == 1,
            'Both live attempts remain failed')
    require('TimeoutExpired' in first['error'] and not records['player/window-001/presentations.json'], 'Timeout with no timestamps')
    require(second['child_exit'] == 1 and second['managed_returned_after_cleanup'], 'Second child failure and cleanup')
    for number, result in [('001', first), ('002', second)]:
        prefix = f'player/window-{number}/'
        require(result['plan_sha256'] == sha(records[f'player/live-controller-{number}/plan.json']), 'Actual live plan')
        require(result['runner_sha256'] == sha(records[f'player/live-controller-{number}/run.py']), 'Actual live runner')
        for name, artifact in result['artifacts'].items():
            require(sha(records[prefix + name]) == artifact['sha256']
                    and len(records[prefix + name]) == artifact['bytes'], 'Live raw artifact')
    require(records['player/window-002/stderr.log'] == b'Error: Audio(Backend("A buffer underrun or overrun occurred."))\n', 'Exact audio failure')
    sidecar = read('player/window-002/presentations.json')
    require(sidecar['format'] == 'incant-native-presentations' and sidecar['version'] == 1
            and sidecar['diagnostic_only'] and not sidecar['display_gate'], 'Raw callback schema')
    rows, host, final = sidecar['rows'], sidecar['host'], sidecar['finalization']
    require(sidecar['requested_host_steps'] == 2156 and sidecar['capture_capacity'] == 4096
            and final['accepted_host_steps'] == 1444 and not final['returned_report']
            and final['recording_completed'] is None and final['stage'] == 'window_finalized', 'Partial finalization')
    require(final['window_error'] == 'audio backend: A buffer underrun or overrun occurred.'
            and final['pending_at_last_drain_scan'] == 2, 'Error/drain retained')
    attempts, segments = host['attempts'], host['segments']
    require(len(rows) == len(attempts) == 1445 and len(segments) == 4, 'Bounded actual rows')
    require([a['id'] for a in attempts] == list(range(1, 1446)), 'Attempt identities')
    require(attempts[0]['status'] == 'refresh_completed'
            and all(a['status'] == 'host_committed' for a in attempts[1:]), 'Refresh/host commit distinction')
    outcomes = Counter(); positive_intervals = 0; equal_intervals = 0; previous = None; ids = {}
    for row in rows:
        a, s = attempts[row['attempt'] - 1], segments[row['segment'] - 1]
        require(row['requested_host_step'] == a['requested_host_step']
                and row['surface_generation'] == s['surface_generation']
                and a['session_generation'] == s['session_generation'] and a['role'] == s['role'], 'Callback correlation')
        require(row['drawable_id'] > ids.get(row['surface_generation'], -1), 'Drawable order')
        ids[row['surface_generation']] = row['drawable_id']; outcomes[row['outcome']] += 1
        t = row['presented_seconds']
        if row['outcome'] == 'presented':
            require(isinstance(t, (int, float)) and math.isfinite(t) and t > 0, 'Actual positive timestamp')
        else:
            require(t is None, 'Never infer a missing/zero timestamp')
        if previous and previous['segment'] == row['segment'] and previous['outcome'] == row['outcome'] == 'presented':
            d = t - previous['presented_seconds']; require(d >= 0, 'No timestamp reversal')
            positive_intervals += d > 0; equal_intervals += d == 0
        previous = row
    require(outcomes == {'presented': 1415, 'reported_zero': 28, 'requested_missing_callback': 2}, 'All callback outcomes')
    require(positive_intervals == 1407 and equal_intervals == 1, 'Do not hide equal timestamp or gaps')
    require(sidecar['coverage'] == {'abandoned': 0, 'acquired': 0, 'discarded': 0, 'invalid': 0,
            'missing': 2, 'overflow': 0, 'presented': 1415, 'protocol_errors': 0, 'rows': 1445,
            'segments': 4, 'zero': 28}, 'Exact coverage')
    require(not host['exhausted'] and host['omitted_attempts'] == 0
            and host['boundary_counts'][1] == 6, 'No metadata loss; focus changes retained')
    partial = read('player/window-002/coverage-partial.json')
    require(partial['failed_attempt'] and partial['coverage_limited_or_inconclusive']
            and not partial['performance_acceptance'] and len(partial['observed_positive_intervals']) == 1407,
            'Partial analysis does not upgrade outcome')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', type=Path, help='Read exact producing Git objects from this checkout')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    archive = HERE / 'records.json.gz'
    require(archive.stat().st_size == manifest['archive_bytes'] and file_sha(archive) == manifest['archive_sha256'], 'Archive')
    require(0 < manifest['expanded_bytes'] <= 128 * 1024 * 1024, 'Expanded archive bound')
    with gzip.open(archive, 'rb') as stream:
        raw = stream.read(manifest['expanded_bytes'] + 1)
    require(len(raw) == manifest['expanded_bytes'], 'Exact expanded bytes')
    blobs = {k: base64.b64decode(v, validate=True) for k, v in json.loads(raw).items()}
    require(set(blobs) == {r['sha256'] for r in manifest['records']}, 'Exact content inventory')
    records = {r['name']: blobs[r['sha256']] for r in manifest['records']}
    require(len(records) == len(manifest['records']), 'Unique record names')
    for row in manifest['records']:
        require(sha(records[row['name']]) == row['sha256'] and len(records[row['name']]) == row['bytes'], 'Record hash')
        if args.originals:
            require(Path(row['original']).read_bytes() == records[row['name']], 'Original record identity')
    if args.binaries:
        for row in manifest['external_artifacts']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'] and file_sha(path) == row['sha256'], 'External artifact')
    if args.git_source:
        for row in manifest['source_files']:
            result = subprocess.run(['git', 'show', manifest['qualified_source'] + ':' + row['path']],
                                    cwd=args.git_source, capture_output=True, timeout=10, check=True)
            require(sha(result.stdout) == row['sha256'], 'Immutable qualified Git source')
    replay(records, manifest)
    print(f"PASS: {len(records)} records; CPU/build qualified; live002 FAILED after1444 steps, "
          "1415 positive/28 zero/2 missing callbacks. No complete replay or performance claim.")


if __name__ == '__main__':
    main()
