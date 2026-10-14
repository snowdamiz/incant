#!/usr/bin/env python3
"""Replay retained ordered-query shipping evidence. No player/build execution."""
import argparse
import base64
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ORDER = ('B0', 'C0', 'C1', 'B1', 'B2', 'C2')
PAIRS = (('C0', 'B0'), ('C1', 'B1'), ('C2', 'B2'))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def need(condition, message):
    if not condition:
        raise ValueError(message)


def interval(row, ticks):
    need(row['samples'] == ticks and all(row[k] == 0 for k in
         ('missing_frames', 'invalid_frames', 'overflow_samples')), 'Incomplete phase')
    counts = row['counts']
    need(len(counts) == 10001 and all(type(n) is int and n >= 0 for n in counts)
         and sum(counts) == ticks, 'Histogram coverage')
    rank = (95 * ticks + 99) // 100
    count = 0
    for index, value in enumerate(counts):
        count += value
        if count >= rank:
            bounds = {'lower': 0, 'upper': 0} if index == 0 else {
                'lower': (index - 1) * 10000 + 1, 'upper': index * 10000}
            need(row['p95_lower_ns'] == bounds['lower'] and
                 row['p95_upper_ns'] == bounds['upper'], 'Reported p95 disagrees')
            return bounds
    raise ValueError('Empty histogram')


def budget(v):
    return 'pass' if v['upper'] <= 2000000 else 'fail' if v['lower'] > 2000000 else 'inconclusive'


def compare(c, b):
    verdicts = {}
    for key, v in c.items():
        p = b[key]
        verdicts[key] = ('pass' if v['upper'] * 100 <= p['lower'] * 105 else
                         'regression' if v['lower'] * 100 > p['upper'] * 105 else 'inconclusive')
    within = budget(c['script.tick.p95_wall_ns'])
    return {'regression_limit_percent': 5, 'metrics': verdicts, 'script_2ms_p95': within,
            'passed': within == 'pass' and all(x == 'pass' for x in verdicts.values()),
            'phase_gate_passed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='Also verify retained original text and PCM payloads')
    parser.add_argument('--git-source', action='store_true', help='Verify frozen source blobs against immutable Git objects')
    parser.add_argument('--binaries', action='store_true', help='Also rehash retained players and cooked-data trees')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_bytes())
    need(digest(Path(__file__).read_bytes()) == manifest['verifier_sha256'], 'Verifier changed')
    packed = (HERE / 'records.json.gz').read_bytes()
    need(digest(packed) == manifest['archive_sha256'], 'Archive changed')
    with gzip.GzipFile(fileobj=__import__('io').BytesIO(packed)) as source:
        raw = source.read(128 * 1024**2 + 1)
    need(len(raw) <= 128 * 1024**2, 'Archive size limit')
    archive = json.loads(raw)
    payloads = {sha: base64.b64decode(value, validate=True) for sha, value in archive.items()}
    need(all(digest(raw) == sha for sha, raw in payloads.items()), 'Payload digest')
    records = {r['key']: r for r in manifest['records']}
    need(len(records) == len(manifest['records']), 'Duplicate records')
    origins = {r['origin']: r for r in manifest['records'] if r['origin'] is not None}
    for row in records.values():
        raw = payloads[row['sha256']]
        need(len(raw) == row['bytes'], 'Record size')
        if args.originals and row['origin'] is not None:
            need(Path(row['origin']).read_bytes() == raw, 'Original record changed: ' + row['origin'])
    if args.originals or args.binaries:
        for row in manifest['external_payloads']:
            if (row['kind'] == 'pcm' and args.originals) or (row['kind'] != 'pcm' and args.binaries):
                path = Path(row['path'])
                need(path.stat().st_size == row['bytes'] and digest(path.read_bytes()) == row['sha256'], 'External payload changed')
    def get(key):
        return payloads[records[key]['sha256']]
    def obj(key):
        return json.loads(get(key))
    source = obj('run/source-001/freeze.json')
    need(source['source_sha'] == manifest['source'] and source['baseline_source_sha'] == manifest['baseline'], 'Source commit correspondence')
    if args.git_source:
        specs = [(manifest['source'], r['path'], r['sha256']) for r in source['files']]
        specs += [(r['commit'], r['path'], r['sha256']) for r in manifest['source_objects']]
        commands = ''.join(commit + ':' + path + '\n' for commit, path, _ in specs).encode()
        output = subprocess.check_output(['git', 'cat-file', '--batch'], input=commands, cwd=HERE)
        offset = 0
        for _, path, expected_sha in specs:
            end = output.index(b'\n', offset)
            header = output[offset:end].split()
            need(len(header) == 3 and header[1] == b'blob', 'Missing source object: ' + path)
            size = int(header[2]); start = end + 1
            need(digest(output[start:start + size]) == expected_sha, 'Source object differs: ' + path)
            offset = start + size + 1
        need(offset == len(output), 'Unexpected source object output')
    plan = obj('run/timing-plan.json')
    for arm, prefix in [('B', 'baseline/'), ('C', 'run/')]:
        exported = obj(prefix + 'package/export.json')
        built = obj(prefix + 'package/project-build.json')
        identity = plan['identities'][arm]
        need(digest(get(prefix + 'package/export.json')) == identity['export_sha256'] and
             digest(get(prefix + 'package/build.log')) == identity['build_log_sha256'] and
             exported['player_sha256'] == identity['player_sha256'] and exported['data']['files'] == identity['data'], 'Export binding')
        need(exported['build_storage_removed_before_startup'] and exported['target'] == 'aarch64-apple-darwin' and
             exported['mode'] == 'headless' and exported['physics_mode'] == 'fast' and not exported['startup_source_parsing'], 'Export configuration')
        need(built['script']['source_sha256'] == identity['script_source_sha256'] and built['tick_rate'] == 60, 'Script producer binding')
    need(obj('baseline/package/project-build.json')['script'] == obj('run/package/project-build.json')['script'], 'Compiled script ABI changed')
    need(obj('baseline/package/project-build.json')['compiler'] == obj('run/package/project-build.json')['compiler'], 'Source compiler output changed')
    need(obj('baseline/package/export.json')['codegen'] == obj('run/package/export.json')['codegen'], 'Shipping codegen changed')
    result = obj('run/timing-001/result.json')
    need(result['status'] == 'complete' and result['order'] == list(ORDER) and
         result['pairs'] == [list(p) for p in PAIRS] and result['retries'] == 0, 'Sequence scope')
    need(result['plan_sha256'] == digest(get('run/timing-plan.json')) and
         result['runner_sha256'] == digest(get('run/run-paired.py')), 'Execution source binding')
    need(result['grant_sha256'] == digest(get('run/root-quiet-grant.json')), 'Grant binding')
    grant = obj('run/root-quiet-grant.json')
    need(grant['authorized'] and grant['quiet'] and grant['plan_sha256'] == result['plan_sha256'] and
         grant['runner_sha256'] == result['runner_sha256'], 'Grant mismatch')
    release = obj('run/root-quiet-release.json')
    need(release['exit'] == 0 and release['complete'] and not release['live_owned_processes'] and
         release['result_sha256'] == digest(get('run/timing-001/result.json')), 'Release mismatch')
    external = {r['path']: r for r in manifest['external_payloads']}
    for path, sha in plan['frozen_files'].items():
        row = origins.get(path, external.get(path))
        need(row is not None and row['sha256'] == sha, 'Frozen dependency missing/changed')
    expected = obj('run/functional-001/unprofiled/stdout.json')
    need('profile' not in expected and expected['tick'] == 2156 and expected['observations']['passed'], 'Plain reference')
    need(expected == obj('baseline/functional-001/unprofiled/stdout.json'), 'Historical logical reference mismatch')
    external = {r['path']: r for r in manifest['external_payloads']}
    stats = {}; match = None; source_ids = None; last_end = 0
    for label in ORDER:
        prefix = 'run/timing-001/' + label + '/'
        report = obj(prefix + 'stdout.json'); measured = obj(prefix + 'measurement.json')
        command = obj(prefix + 'command.json')
        need(command['exit'] == 0 and command['requested_heap_mode'] == 'maintained'
             and command['timeout_seconds'] == 45 and command['started_ns'] >= last_end, 'Process ordering/exit')
        last_end = command['ended_ns']
        need(command['stdout.json_sha256'] == digest(get(prefix + 'stdout.json')) and
             get(prefix + 'stderr.log') == b'', 'Raw command output')
        argv = command['argv']
        need(argv[0] == plan['packages'][label[0]] + '/player/incant_game_player' and
             argv[argv.index('--ticks') + 1] == '2156' and '--profile' in argv and
             '--profile-heap-detail' not in argv, 'Player/mode argv')
        need({k: v for k, v in report.items() if k != 'profile'} == expected, 'Full logical parity')
        pcm = external[command['argv'][command['argv'].index('--output') + 1]]
        need(pcm['sha256'] == expected['pcm_sha256'] and pcm['bytes'] == 13798400, 'PCM correspondence')
        need(digest(get(prefix + 'input.json')) == plan['input_hashes']['input'] and
             digest(get(prefix + 'observe.json')) == plan['input_hashes']['observe'], 'Input binding')
        need(measured['identities'] == plan['identities'][label[0]], 'Package identity')
        need(measured['profile_report_sha256'] == digest(get(prefix + 'stdout.json')), 'Measurement raw identity')
        if match is None:
            match = measured['match']
        need(measured['match'] == match, 'Machine/workload/compiler/mode match')
        p = report['profile']; heap = p['quickjs_heap']; frame = p['frame_wall']
        need(p['version'] == 3 and p['completed_frames'] == 2156 and p['failed_frames'] == 0 and
             p['dropped_source_samples'] == 0 and p['tracy_process_transport'] is False, 'Profile coverage')
        need(heap['version'] == 1 and heap['mode'] == 'maintained' and heap['sampling'] == 'script_tick_end_per_vm'
             and heap['samples'] == 2156 and heap['detailed_samples'] == 0 and heap['invalid_samples'] == 0
             and heap['latest']['limit_bytes'] == 32 * 1024**2, 'Heap mode coverage')
        need(frame['version'] == 1 and frame['unit'] == 'ns' and frame['bucket_width_ns'] == 10000
             and frame['max_ns'] == 100000000 and frame['attempted_frames'] == frame['successful_frames'] == 2156
             and all(frame[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
             and frame['saturated'] is False, 'Frame histogram format')
        phases = {r['name']: r for r in frame['phases']}
        need(len(phases) == len(frame['phases']), 'Duplicate phase')
        metrics = {phase + '.p95_wall_ns': interval(phases[phase], 2156)
                   for phase in ('player.frame', 'script.tick')}
        allocations = p['rust_allocations']
        need(allocations['installed'] and not allocations['saturated'], 'Allocation observation unavailable')
        peak = allocations['process_peak_requested_bytes']
        metrics['rust.process_peak_requested_bytes'] = {'lower': peak, 'upper': peak}
        need(metrics == measured['metrics'], 'Metric replay')
        ids = [r['source'] for r in p['scripts']]
        need(len(ids) == 2 and ids == sorted(ids) and ids[0] != ids[1], 'Source groups')
        if source_ids is None:
            source_ids = ids
        need(ids == source_ids and all(r['totals']['calls'] == 2156 and
             r['totals']['unwound'] == r['totals']['unavailable_cpu_samples'] == 0 for r in p['scripts']), 'Source coverage')
        attempt = result['attempts'][ORDER.index(label)]
        need(attempt['label'] == label and attempt['status'] == 'complete' and
             attempt['measurement_sha256'] == digest(get(prefix + 'measurement.json')) and
             attempt['script_2ms_p95'] == budget(metrics['script.tick.p95_wall_ns']), 'Sequence receipt mismatch')
        stats[label] = metrics
    differences = {}
    for c, b in PAIRS:
        need(compare(stats[c], stats[b]) == result['pair_results'][c + '/' + b], 'Pair verdict mismatch')
        differences[c + '/' + b] = {name: {'lower': value['lower'] - stats[b][name]['upper'],
                                         'upper': value['upper'] - stats[b][name]['lower']}
                                    for name, value in stats[c].items()}
    need(result['candidate_2ms'] == {c: budget(stats[c]['script.tick.p95_wall_ns']) for c, _ in PAIRS}, '2ms verdict mismatch')
    script_improved = sum(stats[c]['script.tick.p95_wall_ns']['upper'] < stats[b]['script.tick.p95_wall_ns']['lower'] for c, b in PAIRS)
    script_worsened = sum(stats[c]['script.tick.p95_wall_ns']['lower'] > stats[b]['script.tick.p95_wall_ns']['upper'] for c, b in PAIRS)
    frame_worsened = sum(stats[c]['player.frame.p95_wall_ns']['lower'] > stats[b]['player.frame.p95_wall_ns']['upper'] for c, b in PAIRS)
    need((script_improved, script_worsened, frame_worsened) == (1, 2, 3), 'Conclusion no longer fits raw pairs')
    print(json.dumps({'status': 'verified', 'records': len(records), 'unique_payloads': len(payloads),
                      'machine': match['machine'], 'metrics': stats, 'candidate_minus_baseline': differences,
                      'candidate_2ms': result['candidate_2ms'], 'measured_improvement_established': False,
                      'recommendation': 'Do not adopt this query change on performance grounds',
                      'limitations': ['Single fixed six-process sequence, three predeclared pairs; no retry or significance claim',
                                      'Portable replay validates PCM correspondence hashes; --originals rehashes actual retained PCM',
                                      'Native executables/cooked data remain external; --binaries rehashes them',
                                      'All frame p95 pairs increased; ScriptTick improved in one pair and worsened in two; 5% nonregression is not speedup']}, indent=2))


if __name__ == '__main__':
    main()
