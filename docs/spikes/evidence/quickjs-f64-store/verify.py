#!/usr/bin/env python3
"""Rehash and replay retained assertions; never execute the native experiment."""
import argparse
import base64
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--git-source', action='store_true')
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
        if row['embedded']:
            assert len(payloads[row['sha256']]) == row['bytes']
        if args.external:
            data = Path(row['original']).read_bytes()
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['original']
    def data(name):
        return payloads[rows[name]['sha256']]
    def load(name):
        return json.loads(data(name))
    for generation in range(1, 6):
        prefix = f'source-{generation:03}'
        freeze = load(prefix + '/freeze.json')
        for row in freeze['records']:
            copied = data(prefix + '/copies/' + row['name'])
            assert len(copied) == row['bytes'] and sha(copied) == row['sha256']
            if args.git_source and not row['name'].startswith('registry/'):
                blob = subprocess.check_output(['git', 'show', freeze['source'] + ':' + row['name']], cwd=ROOT)
                assert sha(blob) == row['sha256'], row['name']
    for attempt in range(1, 5):
        prefix = f'execution-{attempt:03}'
        for command in load(prefix + '/checks.json'):
            for stream in ['stdout', 'stderr']:
                name = prefix + '/' + Path(command[stream]).name
                assert sha(data(name)) == command[stream + '_sha256']
            assert command.get('owned_process_group_cleaned'), command['name']
        assert load(prefix + '/terminal.json')['all_native_process_calls_returned_after_cleanup']
    assert load('execution-001/checks.json')[-1]['exit_code'] == 1
    assert load('execution-002/checks.json')[-1]['exit_code'] == 101
    old = load('execution-003/binaries.json')
    assert len({r['sha256'] for r in old.values()}) == 1
    assert load('execution-003/checks.json')[-1]['exit_code'] == 0
    for attempt in range(1, 4):
        assert load(f'execution-{attempt:03}/failure.json')['exception']
        assert not load(f'execution-{attempt:03}/terminal.json')['success']
    prefix = 'execution-004/'
    final = load(prefix + 'result.json')
    assert final['pass'] and final['timing_samples'] == 0 and final['source'] == manifest['source']
    checks = load(prefix + 'checks.json')
    assert all(r['exit_code'] == (1 if r['name'] == 'functional-wrong-store' else 0) for r in checks)
    actual = []
    for variant in ['baseline', 'candidate']:
        result = [json.loads(line) for line in data(prefix + 'functional-' + variant + '.stdout').splitlines()]
        assert len(result) == 5 and len(result[0]['cases']) == 58 and result[0]['owned_buffer_detach']
        names = [r['name'] for r in result[0]['cases']]
        assert len(set(names)) == 58
        assert sum(n.startswith('numeric-resize-') for n in names) == 4
        assert [r['name'] for r in result[1:4]] == ['omitted', 'nanY', 'nanZ']
        assert all(r['refused'] for r in result[1:4])
        s = result[4]
        assert s['ticks'] == 500 and s['rows'] == 9999 and s['every_tick_verified'] and s['transitions'] == 3 and s['ns'] == []
        assert data(prefix + 'functional-' + variant + '.stderr') == b''
        actual.append(result)
    assert actual[0][:4] == actual[1][:4]
    assert b'numeric store/return differs' in data(prefix + 'functional-wrong-store.stderr')
    assert not data(prefix + 'functional-wrong-store.stdout')
    binaries = load(prefix + 'binaries.json')
    assert len({v['sha256'] for v in binaries.values()}) == 3
    locks = set()
    for variant in ['baseline', 'candidate', 'wrong-store']:
        native = data(f'prepared-004/{variant}/rquickjs-sys/quickjs/quickjs.c')
        assert native == data(prefix + 'compiled-native/' + variant + '/quickjs.c')
        binary = rows[prefix + 'binaries/' + variant]
        assert binary['sha256'] == binaries[variant]['sha256'] and binary['mode'] & 0o111
        locks.add(rows[f'prepared-004/{variant}/Cargo.lock']['sha256'])
    assert len(locks) == 1
    patch = data('source-005/copies/tools/probes/quickjs-f64-store/numeric-store.patch')
    assert patch and b'JS_IsNumber(val)' in patch
    assert b'/* deliberate omitted store */' in data('prepared-004/wrong-store/rquickjs-sys/quickjs/quickjs.c')
    print(json.dumps({'pass': True, 'records': len(rows), 'payloads': len(payloads),
                      'functional_cases_per_variant': 58, 'ticks_per_variant': 500,
                      'no_timing_or_production_acceptance': True, 'external': args.external,
                      'git_source': args.git_source}))


if __name__ == '__main__':
    main()
