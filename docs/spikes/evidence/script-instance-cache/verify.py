#!/usr/bin/env python3
"""Check portable source generations, original command receipts and binary copies."""
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


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    m = json.loads((HERE / 'manifest.json').read_text())
    assert sha(Path(__file__).read_bytes()) == m['verifier_sha256']
    archive = (HERE / 'artifacts.json.gz').read_bytes()
    assert sha(archive) == m['archive_sha256']
    payloads = {k: base64.b64decode(v, validate=True) for k, v in json.loads(gzip.decompress(archive))['payloads'].items()}
    assert all(sha(v) == k for k, v in payloads.items())
    rows = {r['path']: r for r in m['records']}
    assert len(rows) == len(m['records'])
    for path, r in rows.items():
        if r['embedded']:
            assert len(payloads[r['sha256']]) == r['bytes']
        if args.external:
            p = Path(m['raw_root']) / path
            assert p.stat().st_size == r['bytes'] and sha(p.read_bytes()) == r['sha256'], p
            assert oct(p.stat().st_mode & 0o777) == r['mode'], p
    def read(path):
        return payloads[rows[path]['sha256']]
    def data(path):
        return json.loads(read(path))
    freezes = {}
    for i in range(1, 5):
        prefix = f'source-{i:03}/'
        f = data(prefix + 'freeze.json')
        assert f['base'] == m['base_commit']
        for r in f['rows']:
            assert rows[prefix + r['path']]['sha256'] == r['sha256']
        freezes[rows[prefix + 'freeze.json']['sha256']] = i
    assert data('source-004/freeze.json')['rows'] == m['sources']
    production = ('crates/incant_script/src/bindings.rs', 'crates/incant_script/src/lib.rs')
    for p in production:
        assert len({rows[f'source-{i:03}/' + p]['sha256'] for i in range(1, 5)}) == 1
    # The only failed generation changed test source; no behavior/admission fix.
    for row in m['sources']:
        if row['path'] != 'crates/incant_script/tests/project_bindings/cache.rs':
            assert rows['source-003/' + row['path']]['sha256'] == rows['source-004/' + row['path']]['sha256']
    counts, binaries = {}, 0
    for name in m['passing_commands'] + ['project-002']:
        receipt = data(name + '.json')
        assert receipt['exit'] == (101 if name == 'project-002' else 0)
        assert receipt['error'] is None and receipt['ended_ns'] >= receipt['started_ns']
        generation = freezes[receipt['source_freeze_sha256']]
        assert generation == (3 if name == 'project-002' else 4 if name in ('project-003', 'clippy-003') else 2)
        for kind, log in receipt['logs'].items():
            retained = rows[log['path']]
            assert retained['sha256'] == log['sha256'] and retained['bytes'] == log['bytes']
        argv = receipt['argv']
        assert argv[0] == './tools/cargo' and '--offline' in argv and '--locked' in argv and '-j2' in argv
        assert argv[argv.index('--profile') + 1] == 'ci'
        assert argv[argv.index('--target') + 1] == 'aarch64-apple-darwin'
        assert argv[argv.index('--features') + 1] == 'compiler,physics-deterministic'
        assert receipt['environment']['RAYON_NUM_THREADS'] == '2'
        if name.startswith('clippy'):
            assert argv[-3:] == ['--', '-D', 'warnings']
        for binary in receipt['binaries']:
            copy = rows[binary['copy']]
            assert copy['sha256'] == binary['sha256'] and copy['bytes'] == binary['bytes']
            assert int(copy['mode'], 8) & 0o111
            assert binary['original'] in read(name + '.stderr').decode()
            binaries += 1
        result = re.search(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed;', read(name + '.stdout').decode())
        if result:
            counts[name] = list(map(int, result.groups()))
    assert counts == {'private-001': [4, 0], 'project-001': [16, 0], 'checkpoint-001': [14, 0], 'project-003': [17, 0], 'project-002': [16, 1]}
    assert 'SyntaxError: invalid property name' in read('project-002.stdout').decode()
    # Validate immutable Git copies if the producer object is available; the
    # portable frozen bytes remain independently hash-bound when it is not.
    probe = subprocess.run(['git', 'cat-file', '-e', m['source_commit']], cwd=ROOT, capture_output=True)
    if probe.returncode == 0:
        for row in m['sources']:
            content = subprocess.check_output(['git', 'show', m['source_commit'] + ':' + row['path']], cwd=ROOT)
            assert sha(content) == row['sha256']
    print(json.dumps({'passed': True, 'records': len(rows), 'payloads': len(payloads), 'sources': len(m['sources']),
                      'binary_copies': binaries, 'commands': 7, 'historical_failures': 1, 'distinct_passing_cases': 35,
                      'external_checked': args.external, 'producer_git_checked': probe.returncode == 0}))


if __name__ == '__main__':
    main()
