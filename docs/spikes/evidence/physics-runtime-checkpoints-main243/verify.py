#!/usr/bin/env python3
"""Verify current source equality and retained producer/graph evidence; no builds."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]

def sha(data):
    return hashlib.sha256(data).hexdigest()

scope = json.loads((HERE / 'source-composition.json').read_text())
paths = subprocess.check_output(['git', 'ls-files', '--', *scope['roots']], cwd=ROOT).decode().splitlines()
assert paths == [r['path'] for r in scope['files']]
frozen_blob = (ROOT / scope['producer_prepared_freeze_gzip']).read_bytes()
assert sha(frozen_blob) == scope['producer_prepared_freeze_gzip_sha256']
frozen = gzip.decompress(frozen_blob)
assert sha(frozen) == scope['producer_prepared_freeze_sha256']
old = {r['path']: r['sha256'] for r in json.loads(frozen)['sources']}
for row in scope['files']:
    assert sha((ROOT / row['path']).read_bytes()) == row['sha256'] == old[row['path']], row['path']
review = scope['review']
assert sha((ROOT / review['path']).read_bytes()) == review['sha256']
archive = (HERE / 'graph-records.json.gz').read_bytes()
meta = json.loads((HERE / 'graph-manifest.json').read_text())
assert sha(archive) == meta['sha256']
records = json.loads(gzip.decompress(archive))
assert len(records) == meta['records']
by_name = {r['name']: r for r in records}
assert len(by_name) == len(records)
for row in records:
    assert sha(row['text'].encode()) == row['sha256'], row['name']
for mode in ['ordinary', 'det', 'fast8']:
    graphs = []
    for label in ['prepared', 'current']:
        command = json.loads(by_name[f'graph-{mode}-{label}.json']['text'])
        stdout = by_name[f'graph-{mode}-{label}.txt']['text']
        assert command['exit_code'] == 0
        assert sha(stdout.encode()) == command['stdout_sha256']
        normalized = stdout.replace(command['cwd'], '<workspace>')
        assert sha(normalized.encode()) == command['normalized_sha256']
        graphs.append(normalized)
    assert graphs[0] == graphs[1], mode
    assert ('incant-checked-restore' in graphs[1]) == (mode != 'ordinary')
    assert ('incant_physics_snapshot' in graphs[1]) == (mode != 'ordinary')
subprocess.run(['python3', str(HERE.parent/'physics-runtime-checkpoints/verify.py')], check=True)
print(f'Verified {len(paths)} exact current/prepared inputs and three selected normal/build feature graphs; no fresh build/execution claim.')
