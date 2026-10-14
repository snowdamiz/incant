#!/usr/bin/env python3
"""Read-only portable result audit; never runs a player, compiler or benchmark."""
import argparse
import base64
import gzip
import hashlib
import io
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
ORDER = ['D0', 'M0', 'M1', 'D1', 'D2', 'M2']
TICKS = 2156


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def integer(value):
    assert type(value) is int and 0 <= value < 2**64
    return value


def interval(report, name):
    profile = report['profile']; histogram = profile['frame_wall']
    assert type(profile['version']) is int and profile['version'] == 3
    assert type(histogram['version']) is int and histogram['version'] == 1
    assert histogram['unit'] == 'ns' and histogram['bucket_width_ns'] == 10000
    assert histogram['max_ns'] == 100000000 and histogram['saturated'] is False
    assert histogram['attempted_frames'] == histogram['successful_frames'] == TICKS
    assert all(histogram[key] == 0 for key in ['failed_frames', 'invalid_frames', 'active_frames'])
    phases = [row for row in histogram['phases'] if row['name'] == name]
    assert len(phases) == 1
    row = phases[0]
    assert row['samples'] == TICKS and all(row[key] == 0 for key in ['missing_frames', 'invalid_frames', 'overflow_samples'])
    counts = [integer(value) for value in row['counts']]
    assert len(counts) == 10001 and sum(counts) == TICKS
    rank = (TICKS * 95 + 99) // 100
    cumulative = 0
    for index, count in enumerate(counts):
        cumulative += count
        if cumulative >= rank:
            result = {'lower': 0 if index == 0 else (index - 1) * 10000 + 1, 'upper': index * 10000}
            assert result == {key: row['p95_' + key + '_ns'] for key in ['lower', 'upper']}
            return result
    raise AssertionError('Missing quantile')


def instrument(report, mode):
    profile = report['profile']; heap = profile['quickjs_heap']
    assert type(profile['version']) is int and profile['version'] == 3
    assert type(heap['version']) is int and heap['version'] == 1
    assert heap['mode'] == mode and heap['sampling'] == 'script_tick_end_per_vm'
    assert heap['samples'] == TICKS and heap['invalid_samples'] == 0
    assert heap['detailed_samples'] == (TICKS if mode == 'detailed' else 0)
    assert heap['latest']['limit_bytes'] == 32 * 1024**2
    for name in ['allocated_bytes', 'allocation_count', 'limit_bytes']:
        integer(heap['latest'][name])
    for name in ['sampled_peak_allocated_bytes', 'sampled_peak_allocation_count']:
        integer(heap[name])
    if mode == 'maintained':
        assert heap['latest']['estimated_used_bytes'] is heap['sampled_peak_estimated_used_bytes'] is None
    else:
        integer(heap['latest']['estimated_used_bytes']); integer(heap['sampled_peak_estimated_used_bytes'])
    scripts = profile['scripts']
    assert len(scripts) == 2 and [row['slot'] for row in scripts] == [0, 1]
    assert [row['source'] for row in scripts] == [[1, 102, 114, 192] + [0] * 11 + [index] for index in (1, 2)]
    for row in scripts:
        totals = row['totals']
        assert totals['calls'] == TICKS and totals['unwound'] == totals['unavailable_cpu_samples'] == 0
        integer(totals['wall_ns']); integer(totals['thread_cpu_ns'])
    return {'profile_version': 3, 'quickjs_heap_version': 1,
            'quickjs_heap_mode': mode, 'quickjs_heap_sampling': heap['sampling']}


def metrics(report):
    profile = report['profile']
    assert report['tick'] == profile['completed_frames'] == TICKS
    assert profile['failed_frames'] == profile['dropped_source_samples'] == 0
    assert profile['tracy_process_transport'] is False
    allocation = profile['rust_allocations']
    assert allocation['installed'] is True and allocation['saturated'] is False
    peak = integer(allocation['process_peak_requested_bytes'])
    return {'player.frame.p95_wall_ns': interval(report, 'player.frame'),
            'script.tick.p95_wall_ns': interval(report, 'script.tick'),
            'rust.process_peak_requested_bytes': {'lower': peak, 'upper': peak}}


def compare(current, baseline):
    assert current['match'] == baseline['match']
    result = {}
    for key, observed in current['metrics'].items():
        previous = baseline['metrics'][key]
        result[key] = 'pass' if observed['upper'] * 100 <= previous['lower'] * 105 else 'regression' if observed['lower'] * 100 > previous['upper'] * 105 else 'inconclusive'
    script = current['metrics']['script.tick.p95_wall_ns']
    budget = 'pass' if script['upper'] <= 2000000 else 'fail' if script['lower'] > 2000000 else 'inconclusive'
    return {'regression_limit_percent': 5, 'metrics': result, 'script_2ms_p95': budget,
            'passed': all(value == 'pass' for value in result.values()) and budget == 'pass', 'phase_gate_passed': False}


def core(report, observed, reference):
    assert report['tick'] == TICKS and report['frames'] == 1724800
    assert report['observations']['passed'] is True and report['logs']['dropped'] == 0
    assert report['render_input']['gpu_submitted'] is False
    checks = report['observations']['checks']
    assert len(checks) == len(observed['checks']) == 8
    for actual, expected in zip(sorted(checks, key=lambda row: row['name']), sorted(observed['checks'], key=lambda row: row['name'])):
        assert {key: actual[key] for key in ['name', 'tick', 'expect']} == expected
        assert actual['passed'] is True
        value, want = actual['actual'], expected['expect']
        if want['type'] == 'vector':
            assert len(value) == 3 and all(math.isfinite(a) and abs(a - b) <= 1e-9 for a, b in zip(value, want['value']))
        else:
            assert value == want['value']
    logs = report['logs']['entries']
    pickups = [row for row in logs if row['message'].startswith('core.pickup ')]
    assert [row['tick'] for row in pickups] == [127, 508, 1266, 1842]
    assert [row['message'].split()[1] for row in pickups] == reference['samples']
    assert [(row['tick'], row['message']) for row in logs if row['message'].startswith('core.damage ')] == [(664, 'core.damage health=2')]
    assert [(row['tick'], row['message']) for row in logs if row['message'].startswith('core.won ')] == [(2066, 'core.won tick=2066 health=2 pickups=4')]
    assert not any(row['message'].startswith('core.lost ') for row in logs)
    assert all(report['observations']['entities'][owner] is None for owner in reference['samples'])
    hud = reference['hud']
    for role in ['won', 'lost', 'prompt']:
        assert report['observations']['entities'][hud[role]]['parent'] == hud['inactive' if role == 'lost' else 'result']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify-only', action='store_true')
    parser.add_argument('--external', action='store_true', help='Also rehash every retained original, including PCM, player and assets')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_bytes())
    assert digest(Path(__file__).read_bytes()) == manifest['verifier_sha256']
    raw = (HERE / 'artifacts.json.gz').read_bytes()
    assert digest(raw) == manifest['archive_sha256']
    with gzip.GzipFile(fileobj=io.BytesIO(raw)) as stream:
        decoded = stream.read(64 * 1024**2 + 1)
    assert len(decoded) <= 64 * 1024**2
    payloads = {sha: base64.b64decode(value, validate=True) for sha, value in json.loads(decoded)['payloads'].items()}
    assert all(digest(value) == sha for sha, value in payloads.items())
    rows = {row['path']: row for row in manifest['records']}
    assert len(rows) == len(manifest['records'])
    assert set(payloads) == {row['sha256'] for row in rows.values() if row['embedded']}
    for row in rows.values():
        if row['embedded']:
            assert len(payloads[row['sha256']]) == row['size']
        if args.external:
            original = Path(row['original'])
            assert original.stat().st_size == row['size'] and digest(original.read_bytes()) == row['sha256']
    def read(path):
        return payloads[rows[path]['sha256']]
    def data(path):
        return json.loads(read(path))
    summary, sequence = data('summary.json'), data('timing-001/sequence.json')
    assert summary['execution_source_sha'] == manifest['execution_source_sha'] == '43b185ef03e0ac377eac50a1183781aef5ab212c'
    assert summary['execution_base_sha'] == '0b1f739c6617f0f654380ae7ad7910219e789b0c'
    assert sequence['order'] == ORDER and sequence['anchors'] == {'maintained': 'M0', 'detailed': 'D0'}
    assert sequence['status'] == 'complete' and summary['phase_gate_passed'] is False
    expected = data('reference/functional-unprofiled/stdout.json')
    observed, reference = data('functional-001/observe.json'), data('functional-001/core-prepared.json')['receipt']
    for mode in ['unprofiled', 'detailed', 'maintained']:
        report = data('functional-001/' + mode + '/stdout.json')
        assert {key: value for key, value in report.items() if key != 'profile'} == expected
        assert report['pcm_sha256'] == rows['functional-001/' + mode + '/output.pcm']['sha256'] == expected['pcm_sha256']
        core(report, observed, reference)
        if mode != 'unprofiled':
            instrument(report, mode)
    measurements = {}
    for label in ORDER:
        prefix = 'timing-001/' + label + '/'
        report, measured, command = data(prefix + 'stdout.json'), data(prefix + 'measurement.json'), data(prefix + 'command.json')
        mode = 'detailed' if label.startswith('D') else 'maintained'
        identity = instrument(report, mode)
        assert identity == measured['match']['profile_instrumentation']
        assert measured['profile_report_sha256'] == rows[prefix + 'stdout.json']['sha256'] == command['stdout.json_sha256']
        assert metrics(report) == measured['metrics']
        assert measured['identities'] == sequence['identities']
        assert measured['match']['machine'] == summary['machine']
        assert command['exit'] == 0 and command['requested_heap_mode'] == mode and command['timeout_seconds'] == 45
        assert '--profile' in command['argv'] and ('--profile-heap-detail' in command['argv']) == (mode == 'detailed')
        assert measured['phase_gate_passed'] is False and not read(prefix + 'stderr.log')
        assert {key: value for key, value in report.items() if key != 'profile'} == expected
        assert report['pcm_sha256'] == rows[prefix + 'output.pcm']['sha256'] == expected['pcm_sha256']
        core(report, observed, reference)
        attempt = next(row for row in sequence['attempts'] if row['label'] == label)
        assert attempt['status'] == 'complete' and attempt['script_2ms_p95'] == 'fail'
        record = next(row for row in summary['records'] if row['label'] == label)
        assert record['script_p95_ns'] == measured['metrics']['script.tick.p95_wall_ns']
        for context, source in zip(record['callback_means'], report['profile']['scripts']):
            assert context['wall_mean_ns'] == [source['totals']['wall_ns'], TICKS]
            assert context['thread_cpu_mean_ns'] == [source['totals']['thread_cpu_ns'], TICKS]
        if label not in ['M0', 'D0']:
            anchor = 'D0' if mode == 'detailed' else 'M0'
            actual = compare(measured, measurements[anchor])
            assert actual == attempt['same_mode_stability']
            assert all(value == 'pass' for value in actual['metrics'].values()) and actual['passed'] is False
        measurements[label] = measured
    for index in range(3):
        detailed, maintained = [measurements[mode + str(index)]['metrics']['script.tick.p95_wall_ns'] for mode in ['D', 'M']]
        assert sequence['descriptive_detailed_minus_maintained'][str(index)] == {
            'lower': detailed['lower'] - maintained['upper'], 'upper': detailed['upper'] - maintained['lower']}
    print(json.dumps({'verified': True, 'functional_processes': 3, 'timing_processes': 6,
                      'same_mode_per_metric_5percent': 'pass', 'all_2ms_budgets': 'fail',
                      'phase_gate_passed': False, 'external_rehashed': args.external}))


if __name__ == '__main__':
    main()
