#!/usr/bin/env python3
"""Read-only proof replay; no compiler, test rerun, network, or diagnostic dispatch."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for data in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(data)
    return digest.hexdigest()


def blobs(ref, paths):
    query = ''.join(f'{ref}:{path}\n' for path in paths).encode()
    stream = io.BytesIO(subprocess.check_output(['git', 'cat-file', '--batch'], input=query, cwd=ROOT))
    for path in paths:
        header = stream.readline().split()
        assert len(header) == 3 and header[1] == b'blob', path
        size = int(header[2])
        data = stream.read(size)
        assert len(data) == size and stream.read(1) == b'\n'
        yield path, data
    assert not stream.read()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--current', action='store_true')
    parser.add_argument('--external', action='store_true')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    assert file_sha(Path(__file__)) == manifest['verifier_sha256']
    packed = (HERE / 'records.json.gz').read_bytes()
    assert sha(packed) == manifest['archive_sha256']
    rows = json.loads(gzip.decompress(packed))
    assert len(rows) == manifest['records']
    records = {}
    for row in rows:
        data = base64.b64decode(row['payload'], validate=True)
        assert len(data) == row['bytes'] and sha(data) == row['sha256']
        assert row['name'] not in records
        records[row['name']] = data
        if args.external:
            assert file_sha(Path(row['original'])) == row['sha256'], row['original']
    freeze = json.loads(records['freeze-001.json'])
    assert freeze['base'] == manifest['base']
    assert sha(records['source-001.patch']) == freeze['patch_sha256']
    assert len(freeze['rows']) == 8
    for row in freeze['rows']:
        assert sha(records['source-001/' + row['path']]) == row['sha256']
    inputs = json.loads(records['build-inputs.json'])
    expected = {row['path']: row for row in inputs}
    assert len(expected) == len(inputs) == manifest['inputs'] == 3909
    for path, data in blobs(manifest['source'], list(expected)):
        assert sha(data) == expected[path]['sha256'], path
        assert len(data) == expected[path]['size']
        if args.current:
            assert file_sha(ROOT / path) == expected[path]['sha256'], path
    for path, data in blobs(manifest['source'], [row['path'] for row in freeze['rows']]):
        assert data == records['source-001/' + path], path
    # The observed authored contract itself is byte-exact to the failed candidate.
    original_test = 'crates/incant_agent_native/tests/fallback.rs'
    assert dict(blobs(manifest['source'], [original_test])) == dict(blobs(manifest['failed_candidate'], [original_test]))
    source = records['source-001/crates/incant_cook/src/typescript.rs'].decode()
    assert 'const DEADLINE: Duration = Duration::from_secs(8);' in source
    assert 'native TypeScript cook exceeded its 8 second deadline' in source
    protocol = json.loads(records['protocol-001.json'])
    review = json.loads(records['source-review-message.json'])
    assert review['patch_sha256'] == freeze['patch_sha256']
    assert review['protocol_sha256'] == sha(records['protocol-001.json'])
    assert protocol['production_deadline_seconds'] == 8
    results = json.loads(records['results.json'])
    assert [r['name'] for r in results] == ['node', 'unit', 'integration', 'fallback', 'clippy', 'fmt', 'policy', 'actionlint', 'conventions', 'diff']
    copies = []
    for result in results:
        name = result['name']
        assert result == json.loads(records[name + '.json'])
        assert result['exit_code'] == 0 and result['source_unchanged'] and result['managed_child_tree_reaped']
        assert result['runner_sha256'] == sha(records['run.py'])
        assert result['freeze_sha256'] == sha(records['freeze-001.json'])
        assert result['stdout_sha256'] == sha(records[name + '.stdout'])
        assert result['stderr_sha256'] == sha(records[name + '.stderr'])
        for binary in result['binaries']:
            copies.append(binary)
            if args.external:
                path = Path(binary['copy'])
                assert path.stat().st_size == binary['size'] and file_sha(path) == binary['sha256']
                assert path.stat().st_mode & 0o111
    assert copies == manifest['binaries'] and len(copies) == 3
    for name, count in [('unit', 3), ('integration', 2), ('fallback', 1)]:
        text = records[name + '.stdout'].decode()
        matches = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', text)
        assert matches == [str(count)], name
    node = records['node.stdout'].decode()
    assert re.search(r'^# tests 23$', node, re.M) and re.search(r'^# pass 23$', node, re.M) and re.search(r'^# fail 0$', node, re.M)
    names = protocol['milestones']
    observations = [json.loads(line.split('INCANT_NATIVE_COOK_PHASES ', 1)[1])
                    for line in records['fallback.stderr'].decode().splitlines()
                    if line.startswith('INCANT_NATIVE_COOK_PHASES ')]
    assert len(observations) == 2
    assert [r['outcome'] for r in observations] == ['exited_success', 'refused']
    for row in observations:
        assert set(row) == {'version', 'outcome', 'parent_elapsed_us', 'state', 'phases'}
        assert row['version'] == 1 and row['state'] == 'valid_prefix'
        phases = row['phases']
        assert [r['phase'] for r in phases] == names[:len(phases)]
        assert all(set(r) == {'phase', 'elapsed_us'} and type(r['elapsed_us']) is int for r in phases)
        times = [r['elapsed_us'] for r in phases]
        assert times == sorted(times)
    assert len(observations[0]['phases']) == 8 and len(observations[1]['phases']) == 3
    old = json.loads(records['failure/job.json'])
    assert old['conclusion'] == 'failure' and old['id'] == 115682817775
    assert old['head_sha'] == manifest['failed_candidate']
    assert 'native TypeScript cook exceeded its 8 second deadline' in records['failure/job.log'].decode()
    assert 'test result: ok. 3 passed; 0 failed;' in records['failure/previous-checkpoint31-windows.log'].decode()
    post = json.loads(records['post-checks.json'])
    assert post == {'inputs': 3909, 'all_unchanged': True, 'commands': 10,
                    'all_exit_zero': True, 'session': 99632, 'terminal_reaped': True}
    print(json.dumps({'verified': True, 'records': len(rows), 'inputs': len(inputs),
                      'commands': len(results), 'binaries': len(copies),
                      'node_controls': 23, 'rust_controls': 6,
                      'production_deadline_seconds': 8, 'windows_execution': 'pending'}))


if __name__ == '__main__':
    main()
