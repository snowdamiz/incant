#!/usr/bin/env python3
"""Read-only historical replay. Never executes archived code or the engine."""
import argparse
from collections import Counter
import copy
import gzip
import hashlib
import io
import json
import math
from pathlib import Path, PurePosixPath
import subprocess

MIB = 1024 * 1024
HERE = Path(__file__).resolve().parent
SOURCE = "b0d37fb8af69f5ff74658011a525f9a37af77d3e"
PCM = "ccabf2c68b6388ac554ac3efb5609ddf6b4a45d1a3f316891d5f344094e99ee3"


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "Duplicate JSON key: " + key)
        result[key] = value
    return result


def reject_constant(value):
    raise ValueError("Nonfinite JSON: " + value)


def parse(data):
    return json.loads(data, object_pairs_hook=unique, parse_constant=reject_constant)


def bounded(path, limit):
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    require(len(data) <= limit, "Read bound: " + str(path))
    return data


def file_identity(path, row):
    require(path.is_file() and path.stat().st_size == row['bytes'], "External size: " + str(path))
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(MIB), b''):
            h.update(chunk)
    require(h.hexdigest() == row['sha256'], "External hash: " + str(path))


def load():
    m = parse(bounded(HERE/'manifest.json', 2*MIB))
    require(m['format'] == 'incant-independent-deadline-evidence-v1' and m['source'] == SOURCE, 'Manifest identity')
    compressed = bounded(HERE/'records.json.gz', 16*MIB)
    require(digest(compressed) == m['archive']['sha256'] and len(compressed) == m['archive']['bytes'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed), mode='rb') as stream:
        raw = stream.read(128*MIB + 1)
    require(len(raw) <= 128*MIB and digest(raw) == m['archive']['uncompressed_sha256'], 'Expanded archive identity/bound')
    records = parse(raw)
    require(type(records) is dict and set(records) == set(m['records']) and len(records) <= 1000, 'Closed record inventory')
    for name, text in records.items():
        p = PurePosixPath(name)
        require(not p.is_absolute() and '..' not in p.parts and type(text) is str, 'Record name/type')
        b = text.encode('utf-8'); row = m['records'][name]
        require(len(b) == row['bytes'] and digest(b) == row['sha256'], 'Record hash: ' + name)
    return m, records


def interval_replay(sidecar, expected):
    """Adjacent drawable rows only. All break outcomes stay in the inventory."""
    require(sidecar['version'] in (2, 4) and sidecar['requested_host_steps'] == 2156, 'Presentation contract')
    h = sidecar['host']; attempts = h['attempts']; rows = sidecar['rows']
    by_id = {a['id']: a for a in attempts}; segments = {s['id']: s for s in h['segments']}
    require(len(by_id) == len(attempts) and len(segments) == len(h['segments']), 'Duplicate owner identity')
    require(list(by_id) == list(range(1, len(attempts)+1)), 'Nonconsecutive attempts')
    require(not h['exhausted'] and h['omitted_attempts'] == 0 and h['timing_errors'] == 0, 'Partial host observation')
    require(sidecar['finalization']['accepted_host_steps'] == 2156 and sidecar['finalization']['returned_report'] is True
            and sidecar['finalization']['recording_completed'] is True and sidecar['finalization']['assertions_passed'] is True, 'Incomplete finalization')
    require(sidecar['coverage'] == expected['coverage'], 'Coverage differs')
    counts = Counter(r['outcome'] for r in rows)
    require(counts == {'presented':expected['coverage']['presented'], 'reported_zero':expected['coverage']['zero']}, 'Unreported outcome')
    require(len({r['attempt'] for r in rows}) == len(rows), 'Reused drawable owner')
    drawable_owners = {r['attempt'] for r in rows}
    roles = Counter(a['role'] for a in attempts)
    require(dict(roles) == expected['roles'], 'Role inventory')
    for row in rows:
        a = by_id[row['attempt']]; s = segments[row['segment']]
        require(row['requested_host_step'] == a['requested_host_step'] and s['session_generation'] == a['session_generation']
                and s['surface_generation'] == row['surface_generation'] and a['role'] == s['role'], 'Drawable owner mismatch')
        require(a['status'] in ('refresh_completed', 'host_committed'), 'Incomplete drawable attempt')
        t = a['host_timing']
        require(t['began_ns'] <= t['reserved_ns'] <= t['completed_ns'], 'Draw CPU stage order')
        if row['outcome'] == 'presented':
            t = row['presented_seconds']; require(type(t) in (int,float) and math.isfinite(t) and t > 0, 'Invalid positive callback')
        else:
            require(row['presented_seconds'] is None, 'Zero callback mislabeled')
    intervals = []; broken = 0; sim_counts = Counter(); steps = Counter()
    for before, after in zip(rows, rows[1:]):
        a = by_id[before['attempt']]; b = by_id[after['attempt']]
        sa = segments[before['segment']]; sb = segments[after['segment']]
        owners = (sa['surface_generation'], sa['session_generation']) == (sb['surface_generation'], sb['session_generation'])
        boundary = sa['id'] == sb['id'] or (sb['id'] == sa['id']+1 and sb['boundary_bits'] == 512)
        between = attempts[a['id']:b['id']-1]
        complete = all(x['role'] == 'simulation' and x['status'] == 'admitted' and x['host_timing']['finished_ns'] is not None for x in between)
        positive = before['outcome'] == after['outcome'] == 'presented'
        increasing = positive and after['presented_seconds'] > before['presented_seconds']
        if not (owners and boundary and complete and increasing):
            broken += 1
            continue
        intervals.append((after['presented_seconds'] - before['presented_seconds'])*1000)
        sim_counts[str(len(between))] += 1
        steps[str(b['requested_host_step'] - a['requested_host_step'])] += 1
    ordered = sorted(intervals); stats = expected['positive_adjacent_intervals']
    require(len(intervals) == stats['count'] and broken == len(expected['breaks']), 'Interval/break counts')
    require(math.isclose(ordered[math.ceil(.95*len(ordered))-1], stats['p95_ms'], abs_tol=1e-9)
            and math.isclose(max(ordered), stats['max_ms'], abs_tol=1e-9), 'Interval ranks')
    require(sum(v > 20 for v in intervals) == len(expected['over20ms']), 'Tail exclusion')
    require(dict(sim_counts) == expected['positive_pair_simulation_count_groups'] and dict(steps) == expected['accepted_step_delta_groups'], 'Adjacent simulation ownership')
    scheduled = [a for a in attempts if a['host_timing']['schedule'] is not None]
    batches = {}; lateness = []; missing = 0
    for a in scheduled:
        t = a['host_timing']; s = t['schedule']; key = (a['session_generation'], s['pulse_ns'], s['deadline_ns'])
        require(t['began_ns'] <= t['completed_ns'] <= t['finished_ns'], 'CPU stage order')
        if key not in batches:
            batches[key] = s
            if s['deadline_ns'] is None:
                missing += 1
            else:
                require(s['pulse_ns'] >= s['deadline_ns'], 'Future simulation admission')
                lateness.append((s['pulse_ns']-s['deadline_ns'])/1e6)
        require(s['due'] == 1 and s['ordinal'] == 0 and s['discarded_ticks'] == 0, 'Changed actual scheduling')
    require(len(batches) == 2156 and missing == expected['wake_lateness']['missing_deadline'], 'Schedule batch inventory')
    require(math.isclose(sorted(lateness)[math.ceil(.95*len(lateness))-1], expected['wake_lateness']['p95_ms'], abs_tol=1e-9), 'Wake rank')
    if sidecar['version'] == 4:
        simulation = [a for a in attempts if a['role'] == 'simulation']
        require([a['requested_host_step'] for a in simulation] == list(range(1,2157)), 'Simulation step coverage')
        for a in simulation:
            require(a['status'] == 'admitted' and a['host_timing']['reserved_ns'] is None
                    and a['host_timing']['acquisitions'] == [None,None] and a['id'] not in drawable_owners, 'Simulation acquired drawable')
        callbacks = []
        for a in attempts:
            if a['role'] != 'committed':
                continue
            d = a['display']
            if d['reason'] == 'pulse':
                require(type(d['callback_sequence']) is int and d['callback_arrived_ns'] <= a['host_timing']['began_ns'], 'Callback admission')
                callbacks.append(d['callback_sequence'])
            else:
                require(d['reason'] == 'stop' and a['requested_host_step'] == 2156 and d['callback_sequence'] is None, 'Boundary flush falsely owns callback')
        require(callbacks == sorted(set(callbacks)), 'Reused or backward display pulse')
    return {'rows':len(rows), 'positive':counts['presented'], 'zero':counts['reported_zero'], 'valid_adjacent':len(intervals),
            'p95_ms':stats['p95_ms'], 'max_ms':stats['max_ms'], 'over_20ms':sum(v > 20 for v in intervals)}


def replay(m, r):
    J = lambda p: parse(r[p])
    for generation in ('source-001', 'source-002'):
        f = J(generation+'/freeze.json')
        require(len(f['files']) == 28, 'Source inventory changed')
        for row in f['files']:
            b = r[generation+'/files/'+row['path']].encode()
            require(len(b) == row['bytes'] and digest(b) == row['sha256'], 'Frozen source mismatch')
    require([s['exit'] for s in J('cpu-002/result.json')['stages']] == [0,0,101], 'Initial lint failure erased')
    require([s['exit'] for s in J('cpu-003/result.json')['stages']] == [0,0], 'Successor lints')
    ship = J('shipping-001/metadata/result.json'); build = J('shipping-001/package/diagnostic-build.json')
    require(ship['exit'] == 0 and ship['source'] == SOURCE and not ship['scratch_retained'], 'Shipping closure')
    calls = J('shipping-001/metadata/managed-calls.json')
    require(len(calls) == 56 and all(c['exit_code'] == 0 and c['native_process_returned'] and c['cleanup_confirmed'] for c in calls), 'Shipping child cleanup')
    require(build['source'] == SOURCE and build['player_sha256'] == m['player_sha256'] and build['diagnostic_only']
            and not build['eligible_for_distribution'] and not build['eligible_for_performance_gate'], 'Package interpretation')
    baseline = J('historical/full-report.json'); report = J('functional-001/artifacts/stdout.json')
    require(report == baseline and report['pcm_sha256'] == PCM, 'Full CPU gameplay parity')
    restart = J('lifecycle-cpu-001/artifacts/stdout.json'); restart_result = J('lifecycle-cpu-001/result.json')
    require(restart['session'] == restart_result['session'] and restart['session']['generation'] == 2
            and restart['session']['host_tick'] == 2216 and restart['session']['simulation_tick'] == 59, 'CPU restart identity')
    old_checks = report['observations']['checks']; new_checks = restart['observations']['checks']
    require(len(old_checks) == 8 and len(new_checks) == 14 and all(c['passed'] for c in new_checks)
            and [c for c in new_checks if c['tick'] == 2156] == old_checks, 'Restart checks changed')
    original = J('historical/input.json'); derived = J('replay-preparation-001/restart-input-001/input.json')
    require(original['ticks'] == 2156 and derived['ticks'] == 2216 and derived['frames'][:len(original['frames'])] == original['frames'], 'Original route altered')
    require(restart_result['pcm']['original_prefix_sha256'] == PCM and restart_result['pcm']['original_prefix_bytes'] == 13798400, 'PCM prefix identity')
    summaries = {}
    for expected in J('paired-independent-review-001/result.json')['arms']:
        arm = expected['arm']; prefix = 'window-'+arm+'-001/'
        result = J(prefix+'result.json'); actual = J(prefix+'stdout.json')
        require(result['exit_code'] == 0 and result['child_cleanup_confirmed'] is True, 'Native pair failed')
        require(actual['observations'] == report['observations'] and actual['host_steps'] == actual['tick'] == 2156, 'Native logical parity')
        require(actual['ui']['viewport'] == [640.0,480.0], 'Matched viewport')
        states = [parse(x) for x in r[prefix+'window-state.jsonl'].splitlines()]
        require(states[-1]['kind'] == 'final' and max(x['focus_lost_events'] for x in states if x['kind']=='snapshot') == 0, 'Pair focus interruption')
        summaries[arm] = interval_replay(J(prefix+'presentations.json'), expected)
    fail = 'window-lifecycle-focus-resize-001/'
    f = J(fail+'result.json'); child = J(fail+'child-result.json')
    require(f['exit_code'] == 1 and f['status'] == 'failed' and f['child_cleanup_confirmed'] is True
            and child['exit_code'] != 0 and 'deadline' in child['error'].lower(), 'Focus timeout relabeled')
    require(all(r[fail+n] == '' for n in ('stdout.json','presentations.json','compilations.json','stderr.log')), 'Failed final output changed')
    states = [parse(x) for x in r[fail+'window-state.jsonl'].splitlines()]; last = states[-1]
    require(last['kind'] == 'snapshot' and last['state']['host_steps'] == last['state']['world_tick'] == 2151
            and last['focus_lost_events'] == 2 and last['state']['simulation_suspended'] is True, 'Focus partial scope')
    require({tuple(x['state']['physical_size']) for x in states if x['kind']=='snapshot'} == {(1280,960)}, 'Unexpected resize')
    require(m['native_lifecycle']['restart'] == 'not executed' and not m['adopted'], 'Lifecycle/adoption overclaim')
    absence = J(fail+'root-absence-001.json')
    require(absence['terminal_exit'] == 1 and {x['pid'] for x in absence['owned']} == {14159,14169}
            and all(x['pid_absent'] and x['group_absent'] for x in absence['owned']), 'Failed focus ownership closure')
    for path in ('cpu-002/result.json','cpu-003/result.json'):
        require(J(path)['all_groups_absent'] is True and all(x['cleanup_confirmed'] for x in J(path)['owned']), 'CPU ownership closure')
    return summaries


def git_source(repo, records):
    count = 0
    # Read only exact Git objects, not the current checkout or arbitrary archived commands.
    for generation in ('source-001', 'source-002'):
        frozen = parse(records[generation+'/freeze.json'])
        source = frozen['source']
        require(len(source) == 40 and all(c in '0123456789abcdef' for c in source), 'Git source SHA')
        for row in frozen['files']:
            p = PurePosixPath(row['path'])
            require(not p.is_absolute() and '..' not in p.parts and ':' not in str(p), 'Git path')
            done = subprocess.run(['git','-C',str(repo),'cat-file','blob',source+':'+str(p)], capture_output=True, timeout=30, check=True)
            require(len(done.stdout) == row['bytes'] and digest(done.stdout) == row['sha256'], 'Git source mismatch: '+str(p))
            count += 1
    return count


def self_test(records):
    raw = parse(records['window-independent-001/presentations.json'])
    expected = parse(records['paired-independent-review-001/result.json'])['arms'][1]
    cases = [
        ('duplicate attempt', lambda d: d['host']['attempts'][1].update(id=1)),
        ('zero filtered into positive', lambda d: d['rows'][0].update(outcome='presented', presented_seconds=1.0)),
        ('cross boundary interval', lambda d: d['host']['segments'][2].update(boundary_bits=16)),
        ('invented callback owner', lambda d: d['host']['attempts'][3]['display'].update(callback_sequence=None)),
        ('future simulation', lambda d: d['host']['attempts'][2]['host_timing']['schedule'].update(deadline_ns=2**63)),
        ('partial stage', lambda d: d['host']['attempts'][2]['host_timing'].update(finished_ns=None)),
        ('unrecorded dropped row', lambda d: d['coverage'].update(missing=1)),
    ]
    for label, mutate in cases:
        candidate = copy.deepcopy(raw); mutate(candidate)
        try:
            interval_replay(candidate, expected)
        except (ValueError, TypeError):
            continue
        raise ValueError('Negative control accepted: '+label)
    for malformed in ('{"x":1,"x":2}', '{"x":NaN}'):
        try:
            parse(malformed)
        except ValueError:
            continue
        raise ValueError('Malformed JSON accepted')
    return len(cases)+2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', type=Path)
    args = parser.parse_args()
    manifest, records = load(); summary = replay(manifest, records)
    if args.originals:
        for row in manifest['records'].values():
            file_identity(Path(row['original']), row)
    if args.binaries:
        for row in manifest['external_artifacts'].values():
            file_identity(Path(row['path']), row)
        pcm = Path(manifest['external_artifacts']['restart-PCM']['path'])
        with pcm.open('rb') as stream:
            require(digest(stream.read(13798400)) == PCM, 'Actual restart PCM prefix')
    controls = self_test(records) if args.self_test else 0
    sources = git_source(args.git_source, records) if args.git_source else 0
    print(json.dumps({'passed':True,'records':len(records),'negative_controls':controls,'source_objects_rehashed':sources,'originals_rehashed':args.originals,
        'binaries_PCM_rehashed':args.binaries,'paired_positive_adjacent_subsets':summary,
        'focus_resize':'failed timeout; incomplete at step 2151; no resize','native_restart':'unexecuted',
        'continuous_display_or_performance_acceptance':False,'adopted':False}, sort_keys=True))


if __name__ == '__main__':
    main()
