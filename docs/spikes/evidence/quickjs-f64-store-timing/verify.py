#!/usr/bin/env python3
"""Recompute the descriptive comparison from all raw samples; never run players."""
import argparse
import base64
import gzip
import hashlib
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    raw = (HERE / 'records.json.gz').read_bytes()
    assert sha(raw) == manifest['archive_sha256']
    archive = json.loads(gzip.decompress(raw))
    payloads = {h: base64.b64decode(v, validate=True) for h, v in archive['payloads'].items()}
    assert all(sha(v) == h for h, v in payloads.items())
    rows = {r['name']: r for r in archive['records']}
    assert len(rows) == len(archive['records']) == manifest['records']
    assert len(payloads) == manifest['payloads']
    for row in rows.values():
        assert len(payloads[row['sha256']]) == row['bytes']
        if args.external:
            data = Path(row['original']).read_bytes()
            assert len(data) == row['bytes'] and sha(data) == row['sha256']
    def data(name):
        return payloads[rows[name]['sha256']]
    def load(name):
        return json.loads(data(name))
    plan, quiet, result, terminal, reaped = [load(n + '.json') for n in ['plan', 'quiet', 'result', 'terminal', 'reaped']]
    assert sha(data('plan.json')) == manifest['plan_sha256'] == quiet['plan_sha256']
    assert sha(data('measure.py')) == manifest['runner_sha256'] == quiet['runner_sha256']
    assert quiet['explicit_root_grant'] and len(quiet['acknowledgments']) == 4
    assert plan['order'] == ['B0', 'C0', 'C1', 'B1', 'B2', 'C2']
    assert plan['warmup_ticks'] == 0 and plan['measured_ticks'] == 500 and plan['retain_first_tick']
    assert plan['variants']['B']['binary']['sha256'] != plan['variants']['C']['binary']['sha256']
    assert result['all_six_functionally_valid'] and result['all_500_samples_per_run_retained']
    assert result['gate_claim'] is None and not result['retuning_or_reruns']
    assert terminal['attempts'] == 6 and terminal['elapsed_seconds'] < plan['quiet_window_max_seconds']
    assert reaped['supervisor_absent'] and reaped['matching_player_processes'] == []
    assert reaped['terminal_sha256'] == sha(data('terminal.json'))
    assert reaped['result_sha256'] == sha(data('result.json'))
    if args.external:
        bindings = [plan[k] for k in ['source_freeze', 'functional_manifest', 'functional_result', 'native_process']]
        bindings += plan['callback_files']
        bindings += [v for variant in plan['variants'].values() for v in variant.values() if isinstance(v, dict)]
        for row in bindings:
            value = Path(row['path']).read_bytes()
            assert len(value) == row['bytes'] and sha(value) == row['sha256'], row['path']
    summaries = {}
    for expected_name, outcome in zip(plan['order'], result['outcomes'], strict=True):
        assert outcome['name'] == expected_name and outcome['pass'] and outcome['exit_code'] == 0
        assert outcome['managed_call_returned_after_cleanup']
        assert outcome['binary_sha256'] == plan['variants'][expected_name[0]]['binary']['sha256']
        for stream in ['stdout', 'stderr']:
            assert sha(data('runs/' + expected_name + '.' + stream)) == outcome[stream]['sha256']
        assert not data('runs/' + expected_name + '.stderr')
        observed = [json.loads(v) for v in data('runs/' + expected_name + '.stdout').splitlines()]
        reference = [json.loads(v) for v in data('functional-' + expected_name[0] + '.stdout').splitlines()]
        assert len(observed) == len(reference) == 5 and observed[:4] == reference[:4]
        actual, expected = dict(observed[4]), dict(reference[4])
        samples = actual.pop('ns')
        assert expected.pop('ns') == [] and actual == expected
        assert len(samples) == 500 and all(type(n) is int and n > 0 for n in samples)
        ordered = sorted(samples)
        summary = {'count': 500, 'total_ns': sum(samples), 'mean_ns': sum(samples) / 500,
                   'median_ns': statistics.median(samples), 'p95_nearest_rank_ns': ordered[474],
                   'minimum_ns': ordered[0], 'maximum_ns': ordered[-1], 'all_functional_checks': True}
        assert summary == outcome['summary']
        summaries[expected_name] = summary
    for index, pair in enumerate(result['comparisons']):
        assert pair['pair'] == index and pair['baseline'] == summaries[f'B{index}'] and pair['candidate'] == summaries[f'C{index}']
        for metric, value in pair['candidate_over_baseline'].items():
            assert value == pair['candidate'][metric] / pair['baseline'][metric]
    assert len(result['comparisons']) == 3
    print(json.dumps({'pass': True, 'records': len(rows), 'payloads': len(payloads),
                      'all_samples_recomputed': 3000, 'no_adoption_or_shipping_claim': True,
                      'external': args.external}))


if __name__ == '__main__':
    main()
