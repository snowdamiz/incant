#!/usr/bin/env python3
"""Verify source composition and accepted immutable producer evidence; no builds."""
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
def sha(data):
    return hashlib.sha256(data).hexdigest()
scope = json.loads((HERE/'source-composition.json').read_text())
paths = subprocess.check_output(['git','ls-files','--',*scope['roots']],cwd=ROOT).decode().splitlines()
assert paths == [r['path'] for r in scope['files']]
for row in scope['files']:
    assert sha((ROOT/row['path']).read_bytes()) == row['sha256'], row['path']
    assert row['producer_exact'] == (row['sha256'] == row['producer_sha256'])
exceptions = [r for r in scope['files'] if not r['producer_exact']]
assert exceptions == scope['exceptions'] and len(exceptions) == 1
assert exceptions[0]['path'] == 'crates/incant_physics_snapshot/src/lib.rs'
base = subprocess.check_output(['git','show',scope['base']+':'+exceptions[0]['path']],cwd=ROOT)
assert sha(base) == exceptions[0]['sha256']
assert sha((HERE/'source-seam.patch').read_bytes()) == scope['seam_sha256']
for review in scope['reviews']:
    assert sha((ROOT/review['path']).read_bytes()) == review['sha256']
final = json.loads((HERE/'physics-native-capture-final-root-20261013.json').read_text())
manifest_path = ROOT/'docs/spikes/evidence/physics-native-capture.json'
assert sha(manifest_path.read_bytes()) == final['manifest_sha256']
manifest = json.loads(manifest_path.read_text())
assert sha((ROOT/manifest['archive']['path']).read_bytes()) == final['archive_sha256']
assert final['status'] == 'accepted'
print(f'Verified {len(paths)} source rows, sole preexisting Error-order seam and accepted immutable producer archive; no new integrated build/execution claimed.')
