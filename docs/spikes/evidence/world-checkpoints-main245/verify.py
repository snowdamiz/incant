#!/usr/bin/env python3
"""Read-only current-main input and producer evidence qualification; no builds."""
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
    parser.add_argument('--verify-only', action='store_true', help='Read-only is the only mode')
    parser.parse_args()
    scope = json.loads((HERE / 'source-composition.json').read_bytes())
    paths = subprocess.check_output(['git', 'ls-files', '--', *scope['roots']], cwd=ROOT).decode().splitlines()
    assert paths == [r['path'] for r in scope['files']]
    producer = HERE.parent / 'native-world-checkpoints'
    manifest_bytes = (producer / 'manifest.json').read_bytes()
    assert sha(manifest_bytes) == scope['producer_manifest_sha256']
    manifest = json.loads(manifest_bytes)
    archive = (producer / manifest['archive']['path']).read_bytes()
    assert sha(archive) == manifest['archive']['sha256']
    rows = json.loads(gzip.decompress(archive))
    assert len(rows) == manifest['archive']['records']
    originals = {}
    for row in rows:
        assert row['path'] not in originals
        data = base64.b64decode(row['data_base64'], validate=True)
        assert sha(data) == row['sha256'] and len(data) == row['bytes']
        originals[row['path']] = data
    freeze = originals['freeze-004.json']
    assert sha(freeze) == scope['producer_freeze_sha256'] == manifest['source_freeze_sha256']
    frozen_rows = json.loads(freeze)['sources']
    assert len(frozen_rows) == scope['producer_source_rows']
    frozen = {r['path']: r['sha256'] for r in frozen_rows}
    def selected(path):
        return any(path == root or path.startswith(root + '/') for root in scope['roots'])
    assert set(paths) == {path for path in frozen if selected(path)}
    for row in scope['files']:
        assert sha((ROOT / row['path']).read_bytes()) == row['sha256'] == frozen[row['path']], row['path']
    for review in scope['reviews']:
        assert sha((ROOT / review['path']).read_bytes()) == review['sha256']
    data = (HERE / 'graph-records.json.gz').read_bytes()
    meta = json.loads((HERE / 'graph-manifest.json').read_bytes())
    assert sha(data) == meta['sha256']
    records = json.loads(gzip.decompress(data))
    assert len(records) == meta['records']
    by_name = {r['name']: r for r in records}
    assert len(by_name) == len(records)
    for row in records:
        assert sha(row['text'].encode()) == row['sha256'], row['name']
    for mode in ['ordinary', 'det', 'fast8']:
        graphs = []
        flags = {'ordinary': [], 'det': ['--features', 'native-checkpoints'],
                 'fast8': ['--no-default-features', '--features', 'native-checkpoints,physics-fast-simd8']}[mode]
        for label in ['prepared', 'current']:
            name = f'graph-{mode}-{label}'
            command = json.loads(by_name[name + '.json']['text'])
            assert command['argv'] == ['tools/cargo', 'tree', '--offline', '--locked', '--target',
                                       'aarch64-apple-darwin', '-p', 'incant_runtime', '--edges',
                                       'normal,build', '--prefix', 'depth', '--format', '{p} {f}', *flags]
            assert command['exit_code'] == 0
            stdout = by_name[name + '.txt']['text']
            assert sha(stdout.encode()) == command['stdout_sha256']
            assert sha(by_name[name + '.stderr']['text'].encode()) == command['stderr_sha256']
            normalized = stdout.replace(command['cwd'], '<workspace>')
            assert sha(normalized.encode()) == command['normalized_sha256']
            graphs.append(normalized)
        assert graphs[0] == graphs[1], mode
        assert ('incant-checked-restore' in graphs[1]) == (mode != 'ordinary')
        assert ('incant_physics_snapshot' in graphs[1]) == (mode != 'ordinary')
    subprocess.run(['python3', str(producer / 'verify.py')], check=True)
    print(f'Verified {len(paths)} exact current/source004 inputs, three selected dependency graphs and '
          f'{len(rows)} retained producer records. No new build or runtime execution claim.')


if __name__ == '__main__':
    main()
