#!/usr/bin/env python3
"""Replay retained data only; does not launch the package or infer display FPS."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

here = Path(__file__).resolve().parent
sha = lambda b: hashlib.sha256(b).hexdigest()
manifest = json.loads((here / 'manifest.json').read_text())
archive = (here / 'records.json.gz').read_bytes()
assert sha(archive) == manifest['archive_sha256']
with gzip.open(here / 'records.json.gz', 'rb') as stream:
    payload = stream.read(16 * 1024 * 1024 + 1)
assert len(payload) <= 16 * 1024 * 1024
rows = json.loads(payload)
assert len(rows) == len(manifest['records'])
data = {}
for row, expected in zip(rows, manifest['records']):
    assert {k: row[k] for k in ('path', 'bytes', 'sha256')} == expected
    value = base64.b64decode(row['base64'], validate=True)
    assert len(value) == row['bytes'] and sha(value) == row['sha256']
    assert row['path'] not in data
    data[row['path']] = value
read = lambda p: json.loads(data[p])
export = read('result.json')
assert export['source_sha'] == manifest['shipping_source']
assert export['exit_code'] == 0 and export['scratch_empty']
assert export['plan_sha256'] == sha(data['export-plan.json'])
assert export['runner_sha256'] == sha(data['run-export.py'])
assert export['export_receipt_sha256'] == sha(data['package/export.json'])
assert export['player_sha256'] == read('package/export.json')['player_sha256']
window = read('window-001/result.json')
assert window['child_exit'] == 0 and window['exit_code'] == 1
assert window['error'] == 'AssertionError()' and window['managed_child_cleanup_returned']
assert window['plan_sha256'] == sha(data['window-001/plan.json'])
assert window['runner_sha256'] == sha(data['window-001/run.py'])
for name, identity in window['logs'].items():
    value = data['window-001/' + name]
    assert sha(value) == identity['sha256'] and len(value) == identity['bytes']
report = read('window-001/stdout.log')
assert report['tick'] == 600 and report['accepted_frames'] == 601
assert report['profile']['completed_frames'] == 600 and report['profile']['failed_frames'] == 0
assert report['ui']['widgets'] == 7 and report['ui']['glyph_quads'] == 102
assert report['surface_skipped'] == 0 and not report['host_cadence']['display_or_gpu_completion']
correction = read('window-001/analysis-correction.json')
assert all(correction['checks'].values()) and not correction['performance_or_gate_acceptance']
review = read('reviews/core-window001-correction-runtime-20261014.json')
assert review['correction_sha256'] == sha(data['window-001/analysis-correction.json'])
assert review['source_sha256'] == sha(data['source/ui.rs'])
assert review['raw_stdout_sha256'] == sha(data['window-001/stdout.log'])
lifecycle = read('lifecycle/final-receipt.json')
assert lifecycle['source'] == manifest['lifecycle_source']
assert lifecycle['counts'] == {'new_passed': 11, 'existing_passed': 80,
                               'existing_windows_only_skipped': 4, 'failed': 0}
assert lifecycle['result_sha256'] == sha(data['lifecycle/qualify-001/result.json'])
for i, command in enumerate(lifecycle['commands']):
    assert command['returncode'] == 0
    for stream in ('stdout', 'stderr'):
        assert command[stream]['sha256'] == sha(data[f'lifecycle/qualify-001/{i}.{stream}'])
print('PASS: shipping export and corrected window observations; original harness failure retained; no visual/FPS acceptance')
