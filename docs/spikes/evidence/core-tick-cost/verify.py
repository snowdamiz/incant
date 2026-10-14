#!/usr/bin/env python3
"""Replay retained diagnostic assertions without compiling or measuring."""
import argparse
import base64
import gzip
import hashlib
import json
import math
import statistics
import tomllib
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true', help='also hash the retained executable')
    parser.add_argument('--originals', action='store_true', help='rehash all original files')
    args = parser.parse_args()
    directory = Path(__file__).resolve().parent
    manifest = json.loads((directory / 'manifest.json').read_text())
    archive = (directory / 'records.json.gz').read_bytes()
    assert sha(archive) == manifest['archive_sha256']
    values = json.loads(gzip.decompress(archive))
    records = {}
    for row in manifest['records']:
        if row['external']:
            if args.external or args.originals:
                data = Path(row['original']).read_bytes()
            else:
                continue
        else:
            data = base64.b64decode(values[row['sha256']], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256'], row
        if args.originals:
            assert Path(row['original']).read_bytes() == data, row
        records[row['name']] = data

    def read(name):
        return json.loads(records[name])

    root = directory.parents[3]
    for row in manifest['current_sources']:
        assert sha((root / row['path']).read_bytes()) == row['sha256'], row
    result = read('run-003/result.json')
    assert result['passed'] and result['source_unchanged'] and result['prepared_unchanged']
    assert [v['name'] for v in result['commands']] == [
        'node-version', 'rustc-version', 'prepare', 'lock', 'build', 'clippy', 'diagnostic']
    for command in result['commands']:
        assert command['exit_code'] == 0
        for stream in ['stdout', 'stderr']:
            assert sha(records[f"run-003/{command['name']}.{stream}"]) == command[stream]['sha256']
    assert read('run-001/build.json')['exit_code'] == 101
    assert read('run-002/lock.json')['exit_code'] == 0
    key = lambda row: (row['name'], row['version'], row.get('source'), row.get('checksum'))
    engine = {key(v) for v in tomllib.loads(records['source-003/files/Cargo.lock'].decode())['package'] if v.get('source')}
    diagnostic = {key(v) for v in tomllib.loads(records['prepared-003/Cargo.lock'].decode())['package'] if v.get('source')}
    assert diagnostic <= engine
    raw = [json.loads(line) for line in records['run-003/diagnostic.stdout'].splitlines()]
    assert len(raw) == 21
    assert {(v['variant']) for v in raw if v['kind'] == 'negative-refused'} == {'omitted', 'nanY', 'nanZ'}
    loops = [v for v in raw if v['kind'] == 'loop']
    assert [v['variant'] for v in loops] == ['baseline', 'count', 'length', 'cursor', 'soa', 'locals']
    identities = [v for v in raw if v['kind'] == 'identity']
    assert {(v['phase'], v['foreign'], v['method']) for v in identities} == {
        (phase, foreign, method) for phase in range(3) for foreign in [False, True]
        for method in ['direct', 'query']}
    computed = []
    for value in raw:
        row = {key: item for key, item in value.items() if key != 'ns'}
        assert row['diagnostic_only']
        if 'ns' in value:
            samples = value['ns']
            assert len(samples) == (500 if value['kind'] == 'loop' else 300)
            assert all(type(item) is int and item > 0 for item in samples)
            row.update(samples=len(samples), mean_ns=statistics.mean(samples),
                       median_ns=statistics.median(samples), minimum_ns=min(samples),
                       maximum_ns=max(samples), p95_ns=sorted(samples)[math.ceil(.95 * len(samples)) - 1])
        computed.append(row)
    assert read('run-003/summary.json') == {
        'diagnostic_only': True, 'all_samples_included': True, 'rows': computed}
    binary = read('run-003/binary.json')
    assert binary['mode'] == '0o755'
    external = next(v for v in manifest['records'] if v['name'] == 'run-003/incant-core-tick-diagnostic')
    assert binary['sha256'] == external['sha256'] and binary['bytes'] == external['bytes']
    print(json.dumps({'passed': True, 'records': len(manifest['records']),
                      'embedded_payloads': len(values), 'negative_controls': 3,
                      'loop_samples': 3000, 'identity_samples': 3600,
                      'external_verified': args.external or args.originals,
                      'scope': 'diagnostic replay only; not a native shipping performance gate'}))


if __name__ == '__main__':
    main()
