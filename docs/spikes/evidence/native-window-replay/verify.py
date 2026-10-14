#!/usr/bin/env python3
"""Verify retained native replay evidence; no executable or GPU is started."""
import base64
import gzip
import hashlib
import json
from pathlib import Path


def sha(value):
    return hashlib.sha256(value).hexdigest()


here = Path(__file__).resolve().parent
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
def read(name):
    return json.loads(data[name])

cpu = read('cpu/qualify-001/result.json')
assert cpu['outcome'] == 'passed' and cpu['error'] is None
assert cpu['processes_returned_after_cleanup'] and not cpu['gpu_execution']
assert cpu['plan_sha256'] == sha(data['cpu/plan.json'])
assert cpu['runner_sha256'] == sha(data['cpu/qualify.py'])
assert cpu['source_freeze_sha256'] == sha(data['cpu/freeze.json'])
assert cpu['native_process_sha256'] == sha(data['cpu/native_process.py'])
assert [(c['name'], c['exit_code']) for c in cpu['commands']] == [('player-cpu', 0), ('player-clippy', 0)]
for command in cpu['commands']:
    for stream in ('stdout', 'stderr'):
        assert command[stream + '_sha256'] == sha(data[f"cpu/qualify-001/{command['name']}.{stream}"])
assert b'85 passed; 0 failed; 0 ignored' in data['cpu/qualify-001/player-cpu.stdout']
review = read('review.json')
cpu_files = {r['path']:r['sha256'] for r in read('cpu/freeze.json')['files']}
shipping = read('shipping/result.json')
freeze = read('shipping/source-freeze.json')
ship_files = {r['path']:r['sha256'] for r in freeze['tracked_files']}
assert shipping['source_sha'] == freeze['source_sha'] == manifest['source']
for row in review['reviewed_files']:
    assert row['sha256'] == cpu_files[row['path']] == ship_files[row['path']] == sha(data['source/' + row['path']])
assert shipping['exit_code'] == 0 and shipping['scratch_empty'] and shipping['owned_children_cleanup_returned']
assert shipping['runner_sha256'] == sha(data['shipping/run-export.py'])
assert shipping['plan_sha256'] == sha(data['shipping/export-plan.json'])
assert shipping['export_receipt_sha256'] == sha(data['shipping/package/export.json'])
export = read('shipping/package/export.json')
assert export['mode'] == 'window' and export['startup_source_parsing'] is False
assert export['build_storage_removed_before_startup'] is True
assert export['player_sha256'] == shipping['player_sha256']
assert export['codegen']['incant_game_player']['lto'] == 'fat'
for settings in export['codegen'].values():
    assert settings['codegen-units'] == '1' and settings['panic'] == 'abort'
plan = read('window/plan.json')
run = read('window/result.json')
assert plan['source'] == manifest['source']
assert plan['wrapper']['executable_sha256'] == shipping['player_sha256']
assert run['plan_sha256'] == sha(data['window/plan.json']) and run['runner_sha256'] == sha(data['window/run.py'])
assert run['exit_code'] == run['child_exit'] == 0 and run['managed_child_cleanup_returned']
assert run['performance_acceptance'] is False
report = read('window/report.json')
assert report['mode'] == 'native-window' and report['backend'] == 'Metal'
assert report['tick'] == report['host_steps'] == 2156
assert report['accepted_frames'] == 2157 and report['presentation_refreshes'] == 1
assert report['profile']['completed_frames'] == 2156 and report['profile']['failed_frames'] == 0
assert report['host_cadence']['host_wall']['samples'] == 2156
assert report['host_cadence']['display_or_gpu_completion'] is False
assert report['surface_skipped'] == 1 and report['dropped_wall_ticks'] == '0'
record = report['input_recording']
assert record['completed'] and record['requested_host_steps'] == record['accepted_host_steps'] == 2156
assert record['sha256'] == sha(data['window/input.json'])
assert record['physical_devices'] is False and record['native_focus_and_surface_required'] is True
assert report['ui']['widgets'] == 7 and report['ui']['glyph_quads'] == 81
observations = report['observations']
assert observations == read('headless-reference.json')['observations']
assert observations['passed'] and len(observations['checks']) == 8 and len(observations['entities']) == 17
assert read('window/parity.json')['baseline_sha256'] == sha(data['headless-reference.json'])
assert read('window/listener-schema/not-launched.json')['launched'] is False
print('PASS: 85 CPU controls, shipping export, 2156 native gameplay steps and exact headless parity; no FPS acceptance')
