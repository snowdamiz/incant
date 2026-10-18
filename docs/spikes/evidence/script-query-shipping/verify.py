#!/usr/bin/env python3
"""Replay bounded data; never execute an archived runner, compiler or player."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
TICKS = 2156
MIB = 1024**2
CAPABILITIES = ['character_motion', 'trigger_events', 'navigation_path', 'locale', 'localize', 'other']
REGIONS = ['call', 'parse', 'execute', 'materialize']


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def read(path, cap):
    with Path(path).open('rb') as stream:
        raw = stream.read(cap + 1)
    require(len(raw) <= cap, 'Input bound: ' + str(path))
    return raw


def integer(value):
    require(type(value) is int and 0 <= value < 2**64, 'Invalid unsigned observation')
    return value


def vector(value, size):
    require(isinstance(value, list) and len(value) == size, 'Observation shape')
    return [integer(v) for v in value]


def check_external(row):
    raw = read(row['path'], row['bytes'])
    require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'External identity: ' + row['path'])


def p95(profile):
    frame = profile['frame_wall']
    require(frame['version'] == 1 and frame['unit'] == 'ns'
            and frame['attempted_frames'] == frame['successful_frames'] == TICKS
            and all(frame[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
            and frame['saturated'] is False, 'Frame histogram coverage')
    hist = [p for p in frame['phases'] if p['name'] == 'script.tick']
    require(len(hist) == 1, 'Unique ScriptTick histogram')
    h = hist[0]
    counts = vector(h['counts'], frame['max_ns'] // frame['bucket_width_ns'] + 1)
    require(sum(counts) == h['samples'] == TICKS
            and all(h[k] == 0 for k in ('invalid_frames', 'missing_frames', 'overflow_samples')),
            'Histogram samples')
    rank, seen = (TICKS * 95 + 99) // 100, 0
    for index, n in enumerate(counts):
        seen += n
        if seen >= rank:
            bounds = {'lower': 0 if index == 0 else (index - 1) * frame['bucket_width_ns'] + 1,
                      'upper': index * frame['bucket_width_ns']}
            require(bounds == {'lower': h['p95_lower_ns'], 'upper': h['p95_upper_ns']}, 'Histogram p95')
            return bounds, counts
    raise ValueError('Missing histogram rank')


def check_queries(report, analysis, expected_sources):
    p = report['profile']
    require(p['version'] == 4 and p['completed_frames'] == TICKS
            and p['failed_frames'] == p['dropped_source_samples'] == 0
            and p['tracy_process_transport'] is False, 'Diagnostic report contract')
    d = p['script_phase_diagnostic']
    require(d['schema'] == 5 and d['complete'] is True and d['diagnostic_only'] is True
            and d['eligible_for_performance_gate'] is False and d['attempted'] == TICKS
            and d['open'] == 0 and d['max_rows'] == 4096
            and 0 < d['raw_row_bytes'] <= 2048
            and d['raw_storage_bound_bytes'] == 4096 * d['raw_row_bytes'], 'Query coverage/bound')
    require(d['query_capabilities'] == CAPABILITIES and d['query_regions'] == REGIONS
            and 'navigation_regions' not in d and 'budget_check_regions' not in d,
            'Unknown/combined schema')
    require([s['source'] for s in p['scripts']] == expected_sources
            and [s['slot'] for s in p['scripts']] == [0, 1], 'Source/slot identity')
    rows = d['rows']
    require(len(rows) == TICKS, 'Complete raw rows')
    bounds, hist = p95(p)
    bins = [0] * len(hist)
    for index, r in enumerate(rows, 1):
        require(r['frame'] == r['ordinal'] == index and r['valid'] is True
                and r['success'] is True and r['unwound'] is False
                and r['whole_tick_calls'] == 1, 'Invalid/reordered raw row')
        source = vector(r['source_ns'], 64)
        require(vector(r['source_calls'], 64) == [1, 1] + [0] * 62
                and source[2:] == [0] * 62, 'Callback coverage')
        phases = vector(r['regions_ns'], 14)
        nested = vector(r['numeric_ns'], 3)
        tick = integer(r['whole_tick_ns'])
        require(vector(r['numeric_calls'], 3) == [1, 1, 1]
                and sum(phases) == r['trace_ns'] >= tick
                and sum(source) <= phases[7] and nested[0] <= phases[9]
                and sum(nested[1:]) <= phases[11], 'Phase containment')
        width = p['frame_wall']['bucket_width_ns']
        bucket = (tick + width - 1) // width
        require(bucket < len(bins), 'Whole tick overflow')
        bins[bucket] += 1
        q = r['queries']
        require(set(q) == {'inclusive_ns', 'calls', 'failed', 'source_masks', 'recursive_calls'}
                and vector(q['recursive_calls'], 6) == [0] * 6, 'Unknown/recursive query')
        require(all(len(q[k]) == 6 for k in ('inclusive_ns', 'calls', 'failed')), 'Category count')
        masks = vector(q['source_masks'], 6)
        for ns, calls, failed, mask in zip(q['inclusive_ns'], q['calls'], q['failed'], masks, strict=True):
            ns, calls, failed = vector(ns, 4), vector(calls, 4), vector(failed, 4)
            require(calls == [calls[0]] * 4 and failed == [0] * 4
                    and bool(mask) == bool(calls[0]) and mask & ~3 == 0,
                    'Query stage/source coverage')
            require(sum(ns[1:]) <= ns[0] <= sum(source[s] for s in (0, 1) if mask & (1 << s))
                    and all(n or t == 0 for n, t in zip(calls, ns)), 'Query containment')
        require(sum(ns[0] for ns in q['inclusive_ns']) <= sum(source), 'Queries exceed callbacks')
    require(bins == hist and bounds == analysis['instrumented_whole_p95_interval_ns'], 'Raw histogram replay')
    whole = [z for z in p['zones'] if z['name'] == 'script.tick']
    require(len(whole) == 1 and whole[0]['totals']['wall_ns'] == sum(r['whole_tick_ns'] for r in rows),
            'Whole timer/raw correspondence')
    tail = sorted(rows, key=lambda r: (-r['whole_tick_ns'], r['frame']))[:108]
    require(analysis['tail_rows'] == tail, 'Same whole-frame tail, no marginal selection')
    for key, selected in [('all_frames', rows), ('same_frame_tail', tail)]:
        a, q = analysis[key], analysis['queries'][key]
        frames = [r['frame'] for r in selected]
        require(a['frames'] == q['frames'] == frames and a['count'] == q['count'] == len(selected), 'Frame identity')
        require(a['whole_tick_total_ns'] == sum(r['whole_tick_ns'] for r in selected)
                and a['trace_total_ns'] == sum(r['trace_ns'] for r in selected), 'Whole totals')
        for field, row_field, size in [('region_totals_ns', 'regions_ns', 14), ('numeric_nested_totals_ns', 'numeric_ns', 3)]:
            require(a[field] == [sum(r[row_field][i] for r in selected) for i in range(size)], 'Phase totals')
        require(a['callback_nested_totals_ns'] == {str(s): sum(r['source_ns'][s] for r in selected) for s in (0, 1)}, 'Source totals')
        for field in ('inclusive_ns', 'calls', 'failed'):
            require(q[field] == [[sum(r['queries'][field][c][s] for r in selected) for s in range(4)] for c in range(6)], 'Query totals')
        masks = [0] * 6
        for r in selected:
            for c, mask in enumerate(r['queries']['source_masks']):
                masks[c] |= mask
        require(q['source_masks'] == masks == [1, 1, 1, 1, 1, 0], 'Actual gameplay-only attribution')
        require(q['call_residual_ns'] == [n[0] - sum(n[1:]) for n in q['inclusive_ns']]
                and q['callback_residual_ns'] == sum(sum(r['source_ns']) for r in selected) - sum(n[0] for n in q['inclusive_ns']), 'Residual arithmetic')
    return bounds


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ('originals', 'binaries', 'pcm', 'compiler-inputs', 'build-logs'):
        parser.add_argument('--' + flag, action='store_true')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    m = json.loads(read(HERE / 'manifest.json', MIB))
    require(m['format'] == 'script-query-shipping-evidence-v1', 'Manifest version')
    compressed = read(HERE / 'records.json.gz', 4 * MIB)
    require(len(compressed) == m['archive']['bytes'] and sha(compressed) == m['archive']['sha256'], 'Archive hash')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        raw = stream.read(24 * MIB + 1)
    require(len(raw) <= 24 * MIB and len(raw) == m['archive']['uncompressed_bytes'], 'Expansion bound')
    encoded = json.loads(raw)
    require(len(m['records']) == len({r['name'] for r in m['records']})
            and set(encoded) == {r['name'] for r in m['records']}, 'Exact record inventory')
    records = {}
    for row in m['records']:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record hash')
        if args.originals:
            require(read(row['original'], row['bytes']) == data, 'External original: ' + row['name'])
        records[row['name']] = data
    value = lambda name: json.loads(records[name])
    build = value('paired-build-002/result.json')
    plan = value('paired-build-packet-002/plan.json')
    require(build['source'] == plan['source_commit'] == m['producer']
            and build['status'] == 'complete' and build['fresh_builds'] == 2
            and build['offline_startup_ticks'] == 1 and build['full_route_children'] == 0
            and build['scratch_empty'] and build['bytecode_hardening_source_map_symbols_equal'], 'Fresh build result')
    require(build['plan']['sha256'] == sha(records['paired-build-packet-002/plan.json']), 'Build plan binding')
    for prefix, count in [('paired-build-002', 28), ('paired-run-001', 18)]:
        owner, closed, outer = [value(prefix + '/' + n + '.json') for n in ('owner', 'closeout', 'outer-closeout')]
        require(owner['passed'] and len(owner['owned']) == count
                and all(c['exit'] == 0 and c['cleanup_confirmed'] for c in owner['owned'])
                and closed['status'] == outer['status'] == 'complete'
                and closed['fresh_owned_pid_group_absence'] and outer['fresh_owned_pid_group_absence']
                and outer['actual_exec_exit'] == 0 and not outer.get('fresh_matches', outer.get('fresh_ps_rows_matching')), 'Terminal ownership')
        require(closed['owner']['sha256'] == sha(records[prefix + '/owner.json'])
                and outer['closeout']['sha256'] == sha(records[prefix + '/closeout.json']), 'Closure binding')
    failed = value('paired-build-001/owner.json')
    require(not failed['passed'] and failed['peaks']['rss'] > 8 * 1024**3
            and value('paired-build-001/outer-closeout.json')['actual_exec_exit'] == 1, 'Original failure retained')
    closed = value('paired-build-002/closeout.json')
    outer = value('paired-run-001/outer-closeout.json')
    # The two durations sample monotonic time separately; the later cumulative
    # sample includes serialization preparation after the first sample.
    require(closed['prior_attempt_seconds'] + closed['seconds_including_cleanup']
            <= closed['cumulative_seconds_including_cleanup'] <= 5400
            and abs(outer['all_failed_build_successful_build_and_pair_seconds'] - closed['cumulative_seconds_including_cleanup'] - outer['pair_stage_seconds_including_cleanup']) < 1e-6
            and outer['accounting_note'], 'Explicit carry-account correction')
    ordinary = value('paired-build-002/ordinary/export.json')
    query = value('paired-build-002/query/diagnostic-build.json')
    require(ordinary['data'] == query['data'] and ordinary['codegen'] == query['codegen']
            and ordinary['physics_mode'] == query['physics']['mode'] == 'fast'
            and query['eligible_for_performance_gate'] is False and query['player_launches'] == 0
            and query['source']['commit'] == m['producer'], 'Matched package/recipe')
    require(query['source']['files'] == 4887
            and query['source']['manifest_sha256'] == sha(records['paired-build-packet-002/source.json'])
            and query['node_version_observed'] == 'v22.23.3', 'Producing source/compiler binding')
    required_delta = {
        'incant_game_player': ['diagnostic-script-phases', 'diagnostic-script-queries'],
        'incant_runtime': ['diagnostic-script-phases'],
        'incant_profile': ['diagnostic-script-phases', 'diagnostic-script-queries'],
        'incant_script': ['diagnostic-script-phases', 'diagnostic-script-queries'],
    }
    require(set(query['graph_delta']) == set(required_delta), 'Exact target feature recipients')
    for name, added in required_delta.items():
        delta = query['graph_delta'][name]
        require(len(delta['before']) == len(delta['after']) == 1
                and sorted(delta['before'][0] + added) == delta['after'][0], 'Feature delta')
    for flags in query['codegen'].values():
        require(flags['opt-level'] == '3' and flags['panic'] == 'abort'
                and flags['codegen-units'] == '1', 'Shipping code generation')
    require(query['codegen']['incant_game_player']['lto'] == 'fat', 'Shipping LTO')
    require(not {'incant_doc', 'incant_cmd', 'incant_agent'} & set(query['normal_dependencies']),
            'No authoring dependencies in runtime')
    for name in ('bytecode.bin', 'hardening.bin', 'behavior.js.map', 'script-symbols.json'):
        require(m['compiled_payloads']['ordinary/' + name]['sha256'] == m['compiled_payloads']['query/' + name]['sha256'], 'Recorded compiler payload equality')
    inputs = value('paired-shipping-proposal-001/inputs.json')
    expected = value('historical/functional_report.json')
    reports = [value('paired-run-001/' + arm + '/stdout.json') for arm in ('ordinary', 'query')]
    expected_sources = [s['source'] for s in value('historical/source_identity_report.json')['profile']['scripts']]
    for arm, report in zip(('ordinary', 'query'), reports):
        require({k: v for k, v in report.items() if k != 'profile'} == expected
                and expected['tick'] == TICKS and expected['observations']['passed'], 'Full gameplay equality')
        receipt = value('paired-run-001/' + arm + '/result.json')
        require(receipt['status'] == 'complete' and receipt['exit'] == 0
                and receipt['terminal_reaped'] and receipt['full_logic_and_PCM_equal']
                and records['paired-run-001/' + arm + '/stderr.log'] == b'', 'Actual child result')
        for row in receipt['outputs']:
            name = row['path'].split('/paired-run-001/')[-1]
            if 'paired-run-001/' + name in records:
                require(sha(records['paired-run-001/' + name]) == row['sha256'], 'Actual child raw hash')
    require(all(p['sha256'] == expected['pcm_sha256'] == inputs['functional_pcm']['sha256']
                and p['bytes'] == 13798400 for p in m['PCM']), 'Recorded complete PCM equality')
    require(reports[0]['profile']['version'] == 3 and 'script_phase_diagnostic' not in reports[0]['profile'], 'Ordinary diagnostic remains off')
    normal_p95, _ = p95(reports[0]['profile'])
    analysis = value('paired-run-001/query/analysis.json')
    query_p95 = check_queries(reports[1], analysis, expected_sources)
    receipt = value('paired-run-001/attribution-receipt.json')
    require(receipt['script_tick_p95_wall_ns'] == {'ordinary_histogram_interval': normal_p95, 'query_instrumented_histogram_interval': query_p95}
            and receipt['eligible_for_performance_gate'] is False and receipt['optimization_adopted'] is False, 'Scoped p95 observation')
    for key in ('all_frames', 'same_frame_tail'):
        q, a, r = analysis['queries'][key], analysis[key], receipt['attribution'][key]
        require(r['whole_tick_ns'] == a['whole_tick_total_ns'] and r['frames'] == q['count']
                and r['query_inclusive_ns'] == {name: sum(t[i] for t in q['inclusive_ns']) for i, name in enumerate(REGIONS)}
                and r['all_callback_residual_ns'] == q['callback_residual_ns'], 'Conclusion arithmetic')
    if args.binaries:
        for row in m['binaries']:
            check_external(row)
    if args.pcm:
        for row in m['PCM']:
            check_external(row)
    if args.build_logs:
        for row in m['external_large_build_logs']:
            check_external(row)
    if args.compiler_inputs:
        for row in value('paired-build-packet-002/compiler-inputs.json')['records']:
            check_external(row)
    if args.source_root:
        root = args.source_root.resolve()
        for name, digest in value('paired-build-packet-002/source.json')['files'].items():
            path = (root / name).resolve()
            require(path.is_relative_to(root) and sha(read(path, 64 * MIB)) == digest, 'Producer source input: ' + name)
    print(f'PASS: {len(records)} records; two fresh builds, failed001 retained; two complete2156 routes/recorded PCM identity; schema5 raw/all108-tail attribution; all closure records. No gate or optimization claim. Optional originals={args.originals}, binaries={args.binaries}, pcm={args.pcm}, compiler={args.compiler_inputs}, source={bool(args.source_root)}, build_logs={args.build_logs}.')


if __name__ == '__main__':
    main()
