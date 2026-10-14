#!/usr/bin/env python3
"""Replay retained hashes/assertions; never execute the native diagnostic."""
import argparse
import base64
import gzip
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def apply_patch(originals, patch):
    """Exact unified patch replay against frozen original bytes, without fuzz."""
    lines = patch.decode().splitlines(keepends=True)
    result = {}
    i = 0
    while i < len(lines):
        assert lines[i].startswith('--- a/')
        name = lines[i][6:].strip()
        i += 1
        assert lines[i] == '+++ b/' + name + '\n'
        i += 1
        before = originals[name].decode().splitlines(keepends=True)
        after, cursor = [], 0
        while i < len(lines) and not lines[i].startswith('--- '):
            match = re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@.*\n', lines[i])
            assert match, lines[i]
            start = int(match[1]) - 1
            assert start >= cursor
            after.extend(before[cursor:start])
            cursor = start
            i += 1
            consumed = produced = 0
            while i < len(lines) and not lines[i].startswith(('@@ ', '--- ')):
                sign, text = lines[i][0], lines[i][1:]
                assert sign in ' +-'
                if sign in ' -':
                    assert before[cursor] == text
                    cursor += 1
                    consumed += 1
                if sign in ' +':
                    after.append(text)
                    produced += 1
                i += 1
            assert consumed == int(match[2] or 1) and produced == int(match[4] or 1)
        after.extend(before[cursor:])
        result[name] = ''.join(after).encode()
    return result


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
    originals = {r['original']: r for r in archive['records']}
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
    freeze = load('source-001/freeze.json')
    assert freeze['source_commit'] == manifest['source']
    for record in freeze['records']:
        copied = originals[record['copy']]
        assert copied['sha256'] == record['sha256'] and copied['bytes'] == record['bytes']
        path = record['path']
        marker = '/grid-navigation/incant/'
        if args.git_source and marker in path:
            relative = path.split(marker, 1)[1]
            blob = subprocess.check_output(['git', 'show', manifest['source'] + ':' + relative], cwd=ROOT)
            assert sha(blob) == record['sha256'], relative
    review = data('reviews/quickjs-malloc-state-source-independent-20261014.json')
    assert sha(review) == manifest['source_review_sha256']
    prep = load('prepared-001/manifest.json')
    for variant in ['getter', 'wrong-count']:
        for crate, inventory in prep['variants'][variant].items():
            for path, digest in inventory.items():
                assert rows[f'prepared-001/{variant}/{crate}/{path}']['sha256'] == digest
    for crate, patch in [('rquickjs-sys', 'sys.patch'), ('rquickjs-core', 'core.patch')]:
        prefix = f'source-001/files/registry/{crate}/'
        before = {name[len(prefix):]: data(name) for name in rows if name.startswith(prefix)}
        changed = apply_patch(before, data('source-001/files/owned/' + patch))
        assert len(changed) == 3
        for name, after in changed.items():
            assert after == data(f'prepared-001/getter/{crate}/{name}')
        for name, digest in prep['original_crates'][crate].items():
            if name not in changed:
                assert prep['variants']['getter'][crate][name] == digest
    native = data('prepared-001/getter/rquickjs-sys/quickjs/quickjs.c')
    wrong = data('prepared-001/wrong-count/rquickjs-sys/quickjs/quickjs.c')
    anchor = b'*allocation_count = rt->malloc_state.malloc_count;'
    assert native.count(anchor) == 1
    assert wrong == native.replace(anchor, b'*allocation_count = rt->malloc_state.malloc_count + 1;')
    for crate, inventory in prep['variants']['getter'].items():
        for name, digest in inventory.items():
            if (crate, name) != ('rquickjs-sys', 'quickjs/quickjs.c'):
                assert prep['variants']['wrong-count'][crate][name] == digest
    checks = load('execution-001/checks.json')
    assert len(checks) == manifest['commands'] == 15
    for command in checks:
        assert command['exit_code'] == (101 if command['name'] == 'functional-wrong-count' else 0)
        assert command['owned_process_group_cleaned']
        for stream in ['stdout', 'stderr']:
            assert sha(data('execution-001/' + Path(command[stream]).name)) == command[stream + '_sha256']
    assert b'count: first/before-context' in data('execution-001/functional-wrong-count.stderr')
    assert not data('execution-001/functional-wrong-count.stdout')
    assert not data('execution-001/functional-getter.stderr')
    actual = [json.loads(line) for line in data('execution-001/functional-getter.stdout').splitlines()]
    assert len(actual) == 33 and actual[-1]['kind'] == 'success' and actual[-1]['timing_samples'] == 0
    observed = {r['name']: r for r in actual[:-1]}
    assert len(observed) == 32
    for row in observed.values():
        assert row['kind'] == 'observation' and row['same_lock_exact_parity']
        assert row['continuous_peak'] is None
        assert row['limit_bytes'] == (0 if row['name'] == 'zero-means-unlimited' else 32 * 1024 * 1024)
    def allocated(name):
        return observed[name]['allocated_bytes']
    assert allocated('arena-populated') > allocated('arena-half-freed') > allocated('initial')
    assert allocated('cycles-unrooted') > allocated('cycles-collected')
    assert allocated('realloc-grown') > allocated('realloc-small')
    assert allocated('realloc-grown') > allocated('realloc-shrunk')
    assert allocated('external-attached') - allocated('external-before') < 1024 * 1024
    assert observed['external-attached']['full_scan_estimated_used_bytes'] - observed['external-before']['full_scan_estimated_used_bytes'] >= 1024 * 1024
    assert allocated('owned-attached') - allocated('owned-before') >= 1024 * 1024
    assert allocated('transient-during') > max(allocated('transient-before'), allocated('transient-after')) + 1024 * 1024
    for field in ['allocated_bytes', 'allocation_count', 'limit_bytes']:
        assert observed['independent-before'][field] == observed['independent-after'][field]
    bins = load('execution-001/binaries.json')
    assert len({r['sha256'] for r in bins.values()}) == 2
    root_lock = tomllib.loads(data('source-001/files/baseline/Cargo.lock').decode())['package']
    identities = {(r['name'], r['version'], r.get('source'), r.get('checksum')) for r in root_lock}
    lock_hashes = set()
    features = load('execution-001/features.json')
    for variant in ['getter', 'wrong-count']:
        assert data(f'execution-001/compiled-native/{variant}/quickjs.c') == data(f'prepared-001/{variant}/rquickjs-sys/quickjs/quickjs.c')
        binary = rows['execution-001/binaries/' + variant]
        assert binary['mode'] & 0o111 and binary['sha256'] == bins[variant]['sha256']
        lock = data(f'prepared-001/{variant}/Cargo.lock')
        lock_hashes.add(sha(lock))
        for row in tomllib.loads(lock.decode())['package']:
            if row['name'] in ['rquickjs-core', 'rquickjs-sys', 'incant-quickjs-malloc-state-probe']:
                assert 'source' not in row and 'checksum' not in row
            else:
                assert (row['name'], row['version'], row.get('source'), row.get('checksum')) in identities
        assert {r['name']: r['features'] for r in features[variant]} == {'rquickjs': ['default', 'std'], 'rquickjs-core': ['std'], 'rquickjs-sys': []}
    assert len(lock_hashes) == 1
    for record in load('execution-001/compiled-inputs.json'):
        assert originals[record['path']]['sha256'] == record['sha256']
    if args.external:
        for record in load('execution-001/tool-binaries.json'):
            assert sha(Path(record['path']).read_bytes()) == record['sha256']
    result = load('execution-001/result.json')
    assert result['pass'] and result['observations'] == 32 and result['timing_samples'] == 0
    assert load('execution-001/terminal.json')['all_native_process_calls_returned_after_cleanup']
    assert load('execution-001/reaped.json')['supervisor_absent']
    print(json.dumps({'pass': True, 'records': len(rows), 'payloads': len(payloads),
                      'observations': len(observed), 'expected_negative': True,
                      'no_timing_or_production_acceptance': True,
                      'external': args.external, 'git_source': args.git_source}))


if __name__ == '__main__':
    main()
