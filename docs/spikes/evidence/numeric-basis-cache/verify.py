#!/usr/bin/env python3
"""Rehash frozen evidence and assert the exact scoped test/lint coverage."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sha = lambda data: hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    m = json.loads((HERE / 'manifest.json').read_text())
    archive = (HERE / 'records.json.gz').read_bytes()
    assert sha(archive) == m['archive_sha256']
    rows = json.loads(gzip.decompress(archive))
    assert len(rows) == m['records']
    payloads = {}
    for row in rows:
        if 'payload' in row:
            data = base64.b64decode(row['payload'], validate=True)
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
            payloads[row['path']] = data
        if args.external:
            data = Path(row['original']).read_bytes()
            assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
    assert len(payloads) == m['portable_payloads']
    run = json.loads(payloads['run-001.json'])
    assert run['exit_code'] == 0 and run['source_unchanged'] and run['managed_tree_reaped']
    assert len(run['commands']) == 6
    assert run['source_freeze_sha256'] == sha(payloads[m['source_freeze']])
    assert run['build_inputs_sha256'] == sha(payloads['build-inputs-001.json'])
    tests = 0
    binaries = []
    for command in run['commands']:
        assert sha(payloads[command['receipt']]) == command['sha256']
        r = json.loads(payloads[command['receipt']])
        assert r['exit_code'] == 0 and r['managed_tree_reaped']
        prefix = command['receipt'].removesuffix('.json')
        for stream in ['stdout', 'stderr']:
            assert sha(payloads[prefix + '.' + stream]) == r[stream + '_sha256']
        assert r['environment']['RUSTUP_TOOLCHAIN'] == '1.99.0'
        assert r['environment']['RAYON_NUM_THREADS'] == '2'
        for token in ['--offline','--locked','--profile','ci','-j2','--target','aarch64-apple-darwin','incant_runtime']:
            assert token in r['argv']
        if r['name'].endswith('-clippy'):
            assert 'clippy' in r['argv'] and r['argv'][-2:] == ['-D','warnings']
        else:
            count = 5 if r['name'].endswith('-basis') else 7
            filt = 'matrix_basis::tests' if count == 5 else 'columns::'
            assert filt in r['argv'] and '--lib' in r['argv']
            text = payloads[prefix + '.stdout'].decode()
            summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
            assert summaries == [(str(count), '0', '0')]
            assert len(re.findall(r'^test .* \.\.\. ok$', text, re.M)) == count
            tests += count
        binaries.extend(r['binaries'])
    assert tests == 24 and len(binaries) == 4 and len({b['sha256'] for b in binaries}) == 2
    for b in binaries:
        assert b['mode'] == '0o755'
        row = next(r for r in rows if r['original'] == b['copy'])
        assert row['sha256'] == b['sha256'] and row['bytes'] == b['bytes']
    guards = json.loads(payloads['guards-001.json'])
    assert [g['name'] for g in guards] == ['lean','fmt','conventions','diff']
    for g in guards:
        assert g['exit_code'] == 0 and g['managed_tree_reaped']
        assert g['environment']['CARGO_NET_OFFLINE'] == 'true'
        for stream in ['stdout','stderr']:
            data = payloads['guard-001-' + g['name'] + '.' + stream]
            assert sha(data) == g[stream]['sha256'] and len(data) == g[stream]['bytes']
    old = json.loads(payloads['source-001/freeze.json'])
    new = json.loads(payloads[m['source_freeze']])
    for prefix, freeze in [('source-001',old),('source-002',new)]:
        for r in freeze['inputs']:
            assert sha(payloads[prefix+'/files/'+r['path']]) == r['sha256']
        for r in freeze['pinned_registry']:
            assert sha(payloads[prefix+'/native/'+Path(r['path']).name]) == r['sha256']
    changed = [a['path'] for a,b in zip(old['inputs'],new['inputs']) if a['sha256'] != b['sha256']]
    assert changed == ['crates/incant_runtime/src/matrix_basis/tests.rs']
    a = payloads['source-001/files/'+changed[0]]
    b = payloads['source-002/files/'+changed[0]]
    assert b.replace(b'for_each_chunk(|mut chunk|', b'for_each_chunk(|chunk|') == a
    source_count = 0
    if args.git_source:
        for r in json.loads(payloads['build-inputs-001.json']):
            data = subprocess.check_output(['git','show',m['source_commit']+':'+r['path']], cwd=ROOT)
            assert len(data) == r['bytes'] and sha(data) == r['sha256'], r['path']
            source_count += 1
    print(json.dumps(dict(verified=True, records=len(rows), payloads=len(payloads),
                         test_executions=tests, commands=10, immediate_binary_copies=4,
                         source_inputs=source_count, performance_acceptance=False)))


if __name__ == '__main__':
    main()
