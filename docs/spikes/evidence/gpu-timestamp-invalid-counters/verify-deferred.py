#!/usr/bin/env python3
"""Replay retained counter evidence without launching a compiler, engine or GPU."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import struct


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def verify(originals=False):
    raw = Path(__file__).with_name('deferred-resolve.json').read_bytes()
    require(len(raw) <= 512 * 1024, 'bounded portable packet')
    packet = json.loads(raw)
    require(packet['format'] == 'incant-deferred-query-resolution-evidence'
            and packet['version'] == 1, 'packet format')
    data = {}
    for row in packet['records']:
        content = base64.b64decode(row['base64'], validate=True)
        require(row['name'] not in data and len(content) == row['bytes']
                and sha(content) == row['sha256'], 'record identity')
        if originals:
            require(Path(row['original']).read_bytes() == content, 'retained original')
        data[row['name']] = content
    require(len(data) == 22, 'complete compact record inventory')
    read = lambda name: json.loads(data[name])
    close = read('root-closeout.json')
    result = read('gpu-result.json')
    timing = read('gpu-output/gpu-timing.json')
    report = read('gpu-output/report.json')
    require(close['result_sha256'] == sha(data['gpu-result.json'])
            and close['source_review_sha256'] == sha(data['independent-review.json'])
            and close['source'] == sha(data['area_probe.rs']), 'review/source/result bindings')
    require(result['exit'] == 0 and result['error'] is None and result['stage_passed']
            and result['owned_process_group_cleaned_and_reaped'], 'actual completed GPU child')
    require(result['proposal_sha256'] == sha(data['gpu-proposal.json'])
            and result['runner_sha256'] == sha(data['run-gpu.py'])
            and result['log_sha256'] == sha(data['gpu.log']), 'GPU invocation identities')
    outputs = {r['path']: r for r in result['outputs']}
    for name, content in data.items():
        if name in outputs:
            require(outputs[name]['sha256'] == sha(content)
                    and outputs[name]['bytes'] == len(content), 'raw output binding')
    for name in ('helper-compile-result.json', 'helper-cpu-result.json'):
        cpu = read(name)
        require(cpu['exit'] == 0 and cpu['error'] is None
                and cpu['owned_process_group_cleaned_and_reaped']
                and cpu['artifact_sha256'] == close['helper']
                and cpu['plan_sha256'] == sha(data['helper-cpu-plan.json'])
                and cpu['runner_sha256'] == sha(data['run-helper-cpu.py']), 'CPU qualification')
    ticks = list(struct.unpack('<4Q', data['gpu-output/timestamps.bin']))
    require(ticks == timing['raw_ticks'] and timing == close['timing'], 'raw counter bytes')
    require(all(0 < t < (1 << 64) - 1 for t in ticks)
            and ticks[1] >= ticks[0] and ticks[3] >= ticks[2], 'valid ordered pairs')
    require(timing['timestamp_period_ns'] == 1.0 and timing['invalid_reason'] is None
            and timing['resolve_after_sampled_completion'] is True
            and timing['nanoseconds'] == [ticks[1] - ticks[0], ticks[3] - ticks[2]], 'integer deltas')
    require((report['submissions'], report['maps'], report['passes'], report['dispatches'])
            == (2, 1, 2, 2) and report['backend'] == 'Metal'
            and report['adapter'] == 'Apple M5 Pro', 'actual unchanged sampled work')
    comparison = read('comparison.json')
    require(comparison['all_failures'] == []
            and sum(g['records'] for g in comparison['groups']) == close['exact_records'] == 59900
            and all(g['actual_sha256'] == g['expected_sha256']
                    and g['all_mismatched_indices'] == [] for g in comparison['groups']), 'exact numerical comparisons')
    prior = read('prior/gpu-result.json')
    old_outputs = {r['path']: r for r in prior['outputs']}
    require(read('prior/gpu-output/gpu-timing.json')['raw_ticks'][2:] == [0, 0]
            and close['retained_failure']['sha256'] == sha(data['prior/gpu-result.json']), 'original missing counters retained')
    require(outputs['gpu-output/readback.bin']['sha256']
            == old_outputs['gpu-output/readback.bin']['sha256'] == close['raw_numerical_sha256'], 'same complete numerical payload')
    if originals:
        source = Path(next(r['original'] for r in packet['records'] if r['name'] == 'gpu-result.json')).parent
        for row in result['outputs']:
            content = (source / row['path']).read_bytes()
            require(len(content) == row['bytes'] and sha(content) == row['sha256'], 'complete external output')
        require(sha((source / 'area_probe_frozen').read_bytes()) == close['helper'], 'retained executable')
    print(json.dumps({'passed': True, 'records': len(data), 'originals': originals,
                      'raw_ticks': ticks, 'nanoseconds': timing['nanoseconds'],
                      'numerical_payload_unchanged': True, 'performance_gate_passed': False}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    verify(parser.parse_args().originals)
