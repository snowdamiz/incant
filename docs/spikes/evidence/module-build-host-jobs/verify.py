#!/usr/bin/env python3
"""Bounded data-only replay of this one host-job qualification. Executes no archive code."""
import argparse
import copy
import gzip
import hashlib
import io
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
WORKER = 'backend::tests::actual_shared_bridge_build_and_cancellation'
ACTUAL = {WORKER, *('process::tests::' + name for name in (
    'actual_terminal_and_cancel_close_owned_groups',
    'actual_exited_bridge_cannot_leave_pipe_holding_descendant',
    'actual_observer_failure_retains_owner_without_blocking_wait',
    'actual_crashed_bridge_still_requires_delegated_child_closure'))}


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def bounded(path, cap):
    with path.open('rb') as stream:
        data = stream.read(cap + 1)
    require(len(data) <= cap, 'bounded file')
    return data


def load():
    manifest = json.loads(bounded(HERE / 'manifest.json', 256 * 1024))
    require(manifest['format'] == 'module-build-host-evidence-v1', 'manifest format')
    require(len(manifest['external_artifacts']) <= 16 and all(
        0 < r['bytes'] <= 128 * 1024 * 1024 for r in manifest['external_artifacts']), 'artifact inventory/bounds')
    identity = manifest['archive']
    require(identity['file'] == 'records.json.gz', 'fixed archive name')
    packed = bounded(HERE / identity['file'], 2 * 1024 * 1024)
    require(len(packed) == identity['bytes'] and digest(packed) == identity['sha256'], 'packed identity')
    # Consume the exact verified snapshot; never reopen a replaceable pathname.
    with gzip.GzipFile(fileobj=io.BytesIO(packed), mode='rb') as stream:
        raw = stream.read(16 * 1024 * 1024 + 1)
    require(len(raw) <= 16 * 1024 * 1024 and len(raw) == identity['raw_bytes']
            and digest(raw) == identity['raw_sha256'], 'expanded identity/bound')
    corpus = json.loads(raw)
    require(corpus['format'] == 'module-build-host-raw-records-v1', 'record format')
    rows = corpus['records']
    require(len(rows) == identity['records'] and len(rows) <= 256, 'record count')
    for name, row in rows.items():
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'record name')
        data = row['text'].encode('utf-8')
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'record identity')
    return manifest, rows


def validate(manifest, rows):
    def record(name):
        return json.loads(rows[name]['text'])
    results = [record(f'cpu-{n:03}/result.json') for n in range(1, 4)]
    require([r['passed'] for r in results] == [False, False, True], 'preserved failed attempts')
    require([[s['exit'] for s in r['steps']] for r in results]
            == [[0, 0, 101], [0, 0, 0, 0, 0, 0, 0, 101], [0, 0, 0]], 'actual stage exits')
    require('Err` value: Budget' in rows['cpu-001/002.stdout.log']['text'], 'original budget refusal')
    require(rows['cpu-002/007.stdout.log']['text'].count('clippy::collapsible_if') >= 2, 'original lint refusal')
    stages = {s['name']: s for s in results[1]['steps']}
    for name, count in [('service-default', 13), ('agent-adapter', 2), ('actual-owner-and-worker', 5)]:
        require(len(stages[name]['tests']) == count and len(set(stages[name]['tests'])) == count, 'selected test count')
    require(set(stages['actual-owner-and-worker']['tests']) == ACTUAL, 'actual lifecycle/worker inventory')
    require(WORKER not in stages['default-ignored-inventory']['ignored_inventory']
            and WORKER in stages['feature-ignored-inventory']['ignored_inventory'], 'explicit worker feature gate')
    require(rows['cpu-003/002.stdout.log']['text'].strip()
            == 'Fresh headless schemas match every published file byte for byte.', 'full schema comparison')
    for n, result in enumerate(results, 1):
        close = record(f'cpu-{n:03}/closeout.json')
        require(close['fresh_all_absent'] is True and all(
            r['pid_absent'] is True and r['group_absent'] is True for r in result['absence']), 'fresh process absence')
        require(close['actual_terminal_exit'] == (0 if n == 3 else 1), 'actual tool terminal')
        owner = record(f'cpu-{n:03}/owner.json')
        require(all(r['finished'] and r['cleanup_confirmed'] for r in owner['calls'])
                and all(r['cleanup_confirmed'] for r in owner['owned']), 'managed closeout')
    require(abs(sum(r['seconds'] for r in results) - manifest['episode_seconds_including_failures']) < 1e-6, 'cumulative time')
    require(results[1]['source'] == manifest['producer_service']
            and results[2]['source'] == manifest['producer_editor_and_schemas'], 'producer identities')


def self_test(manifest, rows):
    controls = 0
    for name, change in [
        ('cpu-001/result.json', lambda r: r.update(passed=True)),
        ('cpu-002/result.json', lambda r: r['steps'][5].update(tests=[])),
        ('cpu-002/result.json', lambda r: r['steps'][1]['ignored_inventory'].append(WORKER)),
        ('cpu-003/result.json', lambda r: r['absence'][0].update(pid_absent=False)),
        ('cpu-003/closeout.json', lambda r: r.update(actual_terminal_exit=1)),
        ('cpu-002/owner.json', lambda r: r['owned'][0].update(cleanup_confirmed=False)),
    ]:
        altered = copy.deepcopy(rows)
        value = json.loads(altered[name]['text'])
        change(value)
        altered[name]['text'] = json.dumps(value)
        try:
            validate(manifest, altered)
        except ValueError:
            controls += 1
        else:
            raise ValueError('negative control accepted: ' + name)
    return controls


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--artifacts', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    manifest, rows = load()
    validate(manifest, rows)
    if args.originals:
        for row in rows.values():
            require(bounded(Path(row['original']), row['bytes']) == row['text'].encode('utf-8'), 'original bytes')
    if args.artifacts:
        for row in manifest['external_artifacts']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'], 'external artifact size')
            with path.open('rb') as stream:
                remaining = row['bytes']
                value = hashlib.sha256()
                while remaining:
                    chunk = stream.read(min(remaining, 1024 * 1024))
                    require(chunk, 'external artifact shortened')
                    remaining -= len(chunk)
                    value.update(chunk)
                require(stream.read(1) == b'' and value.hexdigest() == row['sha256'], 'external artifact identity')
    print(json.dumps({'passed': True, 'records': len(rows), 'artifacts': len(manifest['external_artifacts']),
                      'negative_controls': self_test(manifest, rows) if args.self_test else 0}))


if __name__ == '__main__':
    main()
