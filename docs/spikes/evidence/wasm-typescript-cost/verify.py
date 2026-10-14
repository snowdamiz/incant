#!/usr/bin/env python3
"""Read-only replay: raw bytes, actual build flags, public transactions and p95."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

from collect import file_digest
from prepare import audit as prepare_audit
from runs import audit as runs_audit

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
H = lambda b: hashlib.sha256(b).hexdigest()


def git_source(commit, rows):
    process = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=ROOT, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for path, sha in rows.items():
            process.stdin.write((commit + ':' + path + '\n').encode()); process.stdin.flush()
            header = process.stdout.readline().split()
            assert len(header) == 3 and header[1] == b'blob', path
            raw = process.stdout.read(int(header[2]))
            assert process.stdout.read(1) == b'\n' and H(raw) == sha, path
    finally:
        process.stdin.close(); process.stdout.close()
        assert process.wait(timeout=10) == 0
    return len(rows)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--external', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    for name, sha in manifest['audit_files'].items():
        assert file_digest(HERE / name) == sha
    archive = (HERE / 'artifacts.json.gz').read_bytes()
    assert H(archive) == manifest['archive_sha256']
    payloads = {sha: base64.b64decode(raw, validate=True) for sha, raw in json.loads(gzip.decompress(archive))['payloads'].items()}
    assert all(H(raw) == sha for sha, raw in payloads.items())
    rows = {r['path']: r for r in manifest['records']}
    assert len(rows) == len(manifest['records'])
    for name, row in rows.items():
        if row['embedded']:
            assert len(payloads[row['sha256']]) == row['bytes'], name
        if args.external:
            p = Path(row['original'])
            assert p.is_file() and p.stat().st_size == row['bytes'] and file_digest(p) == row['sha256'], name
    read = lambda path: payloads[rows[path]['sha256']]
    data = lambda path: json.loads(read(path))
    source = data('source-005/freeze.json')
    assert source['head'] == manifest['executed_source']
    for row in source['sources'] + source['baseline']:
        assert rows['source-005/files/' + row['path']]['sha256'] == row['sha256']
    prior = data('source-004/freeze.json')
    old = {r['path']: r['sha256'] for r in prior['sources'] + prior['baseline']}
    current = {r['path']: r['sha256'] for r in source['sources'] + source['baseline']}
    assert [name for name in current if current[name] != old[name]] == ['docs/spikes/wasm-typescript-cost.md']
    freeze = data('export-001/freeze.json')
    frozen = {r['path']: r['sha256'] for r in freeze['files']}
    assert freeze['head'] == manifest['executed_source']
    assert read('export-001/wabt-version.stdout').decode().strip() == '1.0.42'
    for name, value in freeze['wabt_files'].items():
        assert rows['trusted-wabt/' + name]['sha256'] == value['sha256']
    assert rows['trusted-node']['sha256'] == source['node']['sha256']
    assert source['node']['version'] == 'v22.23.3'
    assert freeze['environment']['CARGO_NET_OFFLINE'] == 'true'
    assert all(frozen[p] == sha for p, sha in current.items() if p in frozen)
    headless = data('build-001/inputs.json'); build = data('build-001/receipt.json')
    assert build['exit'] == 0 and build['sources_unchanged'] is True
    assert build['input_sha256'] == rows['build-001/inputs.json']['sha256']
    for name, sha in build['logs'].items():
        assert rows['build-001/' + name]['sha256'] == sha
    assert build['binary']['sha256'] == rows['build-001/incant_headless']['sha256']
    assert build['binary']['artifact']['features'] == ['physics-deterministic']
    for row in headless['files']:
        if row['path'].startswith(('crates/', 'sdk/', 'third_party/')):
            assert frozen[row['path']] == row['sha256']
    for generation, expected in [('001', 1), ('002', 1), ('003', 0)]:
        c = data('prepare-' + generation + '.command.json')
        assert c['exit'] == expected and c['binary_sha256'] == build['binary']['sha256']
        for name, sha in c['logs'].items():
            assert rows[name]['sha256'] == sha
    assert 'Public import identity differs' in data('prepare-001.command.json')['error']
    assert 'Fixed workload settings differ' in data('prepare-002.command.json')['error']
    assert data('log-shape-red.json')['exit'] != 0
    checks = data('checks-004/checks.json')['commands']
    assert len(checks) == 5 and all(c['exit'] == 0 for c in checks)
    for c in checks:
        assert rows['checks-004/' + c['log']]['sha256'] == c['sha256']
    count = sum(int(re.search(rb'Ran (\d+) tests?', read('checks-004/' + n))[1]) for n in ('workload.log', 'metrics.log'))
    assert count == 21
    prepared = prepare_audit(read, data, rows, frozen)
    result = data('export-001/receipt.json')
    assert result['exit'] == 0 and result['sources_unchanged'] is True
    assert result['freeze_sha256'] == rows['export-001/freeze.json']['sha256']
    assert result['runner_sha256'] == rows['export-001/run.py']['sha256']
    assert result['pair_sha256'] == rows['packages-001/pair.json']['sha256']
    pair = data('packages-001/pair.json')
    assert pair['prepared_sha256'] == rows['prepare-003/prepared.json']['sha256']
    for kind in ('typescript', 'wasm'):
        assert data('packages-001/' + kind + '.command.json')['exit'] == 0
        assert pair['export_sha256'][kind] == rows['packages-001/' + kind + '/export.json']['sha256']
    runs = runs_audit(read, data, rows)
    git_rows = 0
    if args.git_source:
        git_rows += git_source(manifest['executed_source'], frozen | current)
        git_rows += git_source(headless['head'], {r['path']: r['sha256'] for r in headless['files']})
    print(json.dumps({'passed': True, 'records': len(rows), 'payloads': len(payloads),
                      'external_checked': args.external, 'git_source_rows': git_rows,
                      'source005_rows': len(current), 'python_cases': count,
                      'preparation': prepared, 'runs': runs}, indent=2))


if __name__ == '__main__':
    main()
