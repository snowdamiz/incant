"""Read-only replay of retained navigation-cache shipping evidence; never launch players."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ORDER = ('B0', 'C0', 'C1', 'B1', 'B2', 'C2')
PAIRS = (('C0', 'B0'), ('C1', 'B1'), ('C2', 'B2'))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def histogram(row):
    assert row['samples'] == 2156
    assert all(row[k] == 0 for k in ('invalid_frames', 'missing_frames', 'overflow_samples'))
    assert len(row['counts']) == 10001 and sum(row['counts']) == 2156
    assert all(type(v) is int and v >= 0 for v in row['counts'])
    rank = (2156 * 95 + 99) // 100
    cumulative = 0
    for index, count in enumerate(row['counts']):
        cumulative += count
        if cumulative >= rank:
            bounds = {'lower': 0 if index == 0 else (index - 1) * 10000 + 1,
                      'upper': index * 10000}
            assert bounds == {'lower': row['p95_lower_ns'], 'upper': row['p95_upper_ns']}
            return bounds
    raise AssertionError('Missing rank')


def replay(data):
    def j(name):
        return json.loads(data['run/' + name])
    plan, result = j('timing-plan.json'), j('timing-001/result.json')
    assert result['status'] == 'complete' and result['order'] == list(ORDER)
    assert result['plan_sha256'] == digest(data['run/timing-plan.json'])
    assert result['runner_sha256'] == digest(data['run/run-paired.py'])
    assert result['grant_sha256'] == digest(data['run/timing-grant.json'])
    assert [a['label'] for a in result['attempts']] == list(ORDER)
    assert all(a['status'] == 'complete' for a in result['attempts'])
    assert plan['pairs'] == [list(p) for p in PAIRS] and plan['retries'] == 0
    assert plan['ticks'] == 2156 and plan['warmup_exclusion'] == 0
    assert plan['script_budget_ns'] == 2000000 and plan['regression_limit_percent'] == 5
    plain = j('functional-001/unprofiled/stdout.json')
    functional = j('functional-001/result.json')
    assert functional['status'] == 'complete'
    assert [a['mode'] for a in functional['attempts']] == ['unprofiled', 'detailed', 'maintained']
    for attempt in functional['attempts']:
        assert attempt['exit_code'] == 0 and attempt['logical_parity'] and attempt['exact_pcm_parity']
        raw = j('functional-001/' + attempt['mode'] + '/stdout.json')
        assert {k: v for k, v in raw.items() if k != 'profile'} == plain
        if attempt['mode'] != 'unprofiled':
            assert raw['profile']['version'] == 3
            assert raw['profile']['quickjs_heap']['mode'] == attempt['mode']
    assert plain == json.loads(data['baseline/unprofiled.json'])
    values, observations, identity = {}, {}, None
    for label in ORDER:
        prefix = 'timing-001/' + label + '/'
        raw, measure, command = (j(prefix + n) for n in ('stdout.json', 'measurement.json', 'command.json'))
        assert command['exit'] == 0 and command['timeout_seconds'] == 45
        assert not data['run/' + prefix + 'stderr.log']
        assert measure['profile_report_sha256'] == digest(data['run/' + prefix + 'stdout.json'])
        assert measure['identities'] == plan['identities'][label[0]]
        assert measure['match']['inputs'] == plan['input_hashes']
        if identity is None:
            identity = measure['match']
        assert measure['match'] == identity
        assert {k: v for k, v in raw.items() if k != 'profile'} == plain
        p = raw['profile']
        assert p['version'] == 3 and p['completed_frames'] == 2156
        assert p['failed_frames'] == p['dropped_source_samples'] == 0
        heap = p['quickjs_heap']
        assert heap['mode'] == 'maintained' and heap['samples'] == 2156
        assert heap['detailed_samples'] == heap['invalid_samples'] == 0
        assert all(s['totals']['calls'] == 2156 and s['totals']['unwound'] == 0 for s in p['scripts'])
        frame = p['frame_wall']
        assert frame['bucket_width_ns'] == 10000 and frame['max_ns'] == 100000000
        assert frame['successful_frames'] == frame['attempted_frames'] == 2156
        assert not frame['saturated'] and frame['invalid_frames'] == frame['failed_frames'] == frame['active_frames'] == 0
        rows = {r['name']: r for r in frame['phases']}
        m = {name + '.p95_wall_ns': histogram(rows[name]) for name in ('player.frame', 'script.tick')}
        a = p['rust_allocations']
        assert a['installed'] and not a['saturated'] and a['failures'] == 0
        peak = a['process_peak_requested_bytes']
        m['rust.process_peak_requested_bytes'] = {'lower': peak, 'upper': peak}
        assert m == measure['metrics']
        values[label] = m
        zones = {z['name']: z['totals'] for z in p['zones']}
        observations[label] = {'metrics': m, 'rust_allocations': a,
            'frame_max_ns': zones['player.frame']['max_wall_ns'], 'script_max_ns': zones['script.tick']['max_wall_ns'],
            'load_average_before': command['load_average_before'], 'load_average_after': command['load_average_after']}
    pairs = {}
    for candidate, baseline in PAIRS:
        observed = {}
        for metric, cur in values[candidate].items():
            old = values[baseline][metric]
            verdict = 'pass' if cur['upper'] * 100 <= old['lower'] * 105 else (
                'regression' if cur['lower'] * 100 > old['upper'] * 105 else 'inconclusive')
            assert result['pair_results'][candidate + '/' + baseline]['metrics'][metric] == verdict
            observed[metric] = {'verdict_5percent': verdict,
                'delta_lower': cur['lower'] - old['upper'], 'delta_upper': cur['upper'] - old['lower'],
                'reduction_percent_lower': 100 * (1 - cur['upper'] / old['lower']),
                'reduction_percent_upper': 100 * (1 - cur['lower'] / old['upper'])}
        assert values[candidate]['script.tick.p95_wall_ns']['lower'] > 2000000
        assert result['candidate_2ms'][candidate] == 'fail'
        pairs[candidate + '/' + baseline] = observed
    return {'runs': observations, 'pairs': pairs, 'machine': identity['machine'],
            'candidate_2ms': result['candidate_2ms'], 'adopted': False, 'phase_gate_passed': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', type=Path)
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    compressed = (HERE / 'records.json.gz').read_bytes()
    assert digest(compressed) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(compressed))
    payload = {key: value.encode() for key, value in archive['payloads'].items()}
    assert all(digest(value) == key for key, value in payload.items())
    data = {}
    for row in archive['records']:
        content = payload[row['sha256']]
        assert len(content) == row['bytes'] and row['name'] not in data
        data[row['name']] = content
        if args.originals:
            assert Path(row['original']).read_bytes() == content, row['original']
    if args.binaries:
        for row in manifest['external_binary_files']:
            path = Path(row['path'])
            assert path.stat().st_size == row['bytes']
            with path.open('rb') as source:
                h = hashlib.file_digest(source, 'sha256').hexdigest()
            assert h == row['sha256'], str(path)
    if args.git_source:
        source = json.loads(data['run/source-inputs.json'])
        paths = list(source['files'])
        request = ''.join(source['source_commit'] + ':' + path + '\n' for path in paths).encode()
        run = subprocess.run(['git', '-C', str(args.git_source), 'cat-file', '--batch'], input=request, capture_output=True, check=True)
        cursor = 0
        for path in paths:
            end = run.stdout.index(b'\n', cursor)
            header = run.stdout[cursor:end].split(); assert header[1] == b'blob'
            size = int(header[2]); cursor = end + 1
            assert digest(run.stdout[cursor:cursor + size]) == source['files'][path], path
            cursor += size + 1
        assert cursor == len(run.stdout)
    analysis = replay(data)
    assert analysis == json.loads(data['run/analysis.json'])
    print(json.dumps({'status': 'verified', 'records': len(data), 'payloads': len(payload), 'runs': 6,
                      'candidate_2ms': analysis['candidate_2ms'], 'adopted': False, 'phase_gate_passed': False}))

if __name__ == '__main__':
    main()
