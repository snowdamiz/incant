#!/usr/bin/env python3
"""Bounded data-only replay; never runs an engine, build, RPC or GPU."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(packet, originals=False, binaries=False):
    data = {}
    for row in packet['records']:
        raw = base64.b64decode(row['base64'], validate=True)
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Retained bytes')
        require(row['name'] not in data, 'Unique record')
        data[row['name']] = raw
        if originals:
            with Path(row['original']).open('rb') as source:
                require(source.read(len(raw) + 1) == raw, 'Unchanged original')
    read = lambda name: json.loads(data[name])
    build, owner = read('build/result.json'), read('window/result.json')
    plan, result = read('window/plan.json'), read('window/workflow/result.json')
    proof = read('window/workflow/native-window.json')
    require(build['passed'] is True and owner['passed'] is True and result['passed'] is True, 'Actual outcomes')
    require(build['source'] == packet['source'] == plan['source_base'], 'Producing commit')
    require(plan['current_tree'] == packet['tree'], 'Producing tree')
    require(sha(data['build/result.json']) == plan['build_result_sha256'], 'Build binding')
    for prefix, receipt in [('build', build), ('window', owner)]:
        require(sha(data[prefix + '/plan.json']) == receipt['plan_sha256'], 'Plan binding')
        require(sha(data[prefix + '/run.py']) == receipt['runner_sha256'], 'Runner binding')
        require(all(r['pid_absent'] and r['group_absent'] for r in receipt['absence']), 'Owned cleanup')
    require([s['exit'] for s in build['stages']] == [0, 0], 'Frontend and native build')
    for key in ('editor', 'headless'):
        identity = plan[key + '_sha256']
        require(result[key + '_sha256'] == identity, 'Window executable identity')
        require(any(a['copied_sha256'] == identity for a in build['artifacts']), 'Actual Cargo output')
        if binaries:
            with Path(plan[key]).open('rb') as executable:
                require(hashlib.file_digest(executable, 'sha256').hexdigest() == identity, 'Retained executable')
    require(result['window'] == proof and result['model_variant'] == 'material-tables', 'Material workflow')
    require(all(result[k] is True for k in ('actual_app', 'durable_journal_recovery', 'unrelated_authored_bytes_preserved')), 'App and persistence')
    require(all(proof[k] is True for k in ('passed', 'actual_window_presentations', 'world_identity_unchanged',
            'module_counter_continued', 'real_source_watcher', 'undo_not_overwritten', 'source_recovery',
            'manual_import_execute_history_handlers', 'stale_generation_refused', 'stop_releases_world')), 'Native controls')
    require(all(not r[k] for r in (proof, result) for k in ('pixel_capture', 'appearance_review', 'performance_or_phase_gate')), 'Claim limits')
    require(proof['platform'] == 'macos' and 'Apple M5 Pro' in proof['adapter'] and 'backend: Metal' in proof['adapter'], 'Actual adapter')
    stages = proof['stages']
    require([s['stage'] for s in stages] == ['manual_import_original', 'watched_replacement', 'undo_original',
            'redo_replacement', 'malformed_source_retained_replacement', 'source_recovered_replacement'], 'Six stages')
    require([s['frames'] for s in stages] == [13, 274, 281, 469, 574, 634], 'Later renderer acceptance')
    require([s['history']['revision'] for s in stages] == [3, 4, 5, 6, 6, 6], 'History and refusal preservation')
    require(all(s['recipe_matches'] and s['tick'] == 3 for s in stages), 'Accepted paused catalog')
    recipes = [s['accepted_fingerprint'] for s in stages]
    require(recipes[0] != recipes[1] and recipes == [recipes[i] for i in (0, 1, 0, 1, 1, 1)], 'Undo/Redo complete recipes')
    require(proof['play_generation'] == 1 and (proof['paused_tick'], proof['final_tick']) == (3, 4), 'One continuing world')
    require(proof['vm_counter'] == {'before_translation': 12, 'expected_delta': 8, 'observed_delta': 8}, 'Persistent VM counter')
    require(proof['malformed_source_refusal']['code'] == 'asset.import', 'Structured refusal')
    processes = read('window/workflow/processes.json')
    require(len(processes) == result['native_processes'] == 6 and all(p['exit_code'] == 0 and p['reaped'] for p in processes), 'Six managed commands')
    recovered = json.loads(processes[-1]['stdout'])['result']
    require(result['recovered_revision'] == recovered['revision'] == 6, 'Actual journal reopen')
    asset = next(a for a in recovered['project']['assets'].values() if a['path'] == 'refresh.gltf')
    require(asset['sha256'] == recipes[-1], 'Durable replacement recipe')
    require(data['window/workflow/refresh.gltf'] == data['window/workflow/refresh-replacement.gltf'], 'Recovered source')
    fixture = read('window/workflow/fixture.json')
    require(fixture['fixture_authored_through_cmd'] and fixture['model_variant'] == 'material-tables', 'Command-authority fixture')
    for path, identity in fixture['inputs'].items():
        require(sha(data['window/workflow/' + path]) == identity, 'Unrelated authored bytes')
    require(len(data['window/workflow/project.journal.jsonl']) > fixture['journal_bytes_before'], 'Real journal extension')
    return len(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    args = parser.parse_args()
    with (HERE / 'manifest.json').open('rb') as source:
        manifest_raw = source.read(64 * 1024 + 1)
    require(len(manifest_raw) <= 64 * 1024, 'Bounded manifest')
    manifest = json.loads(manifest_raw)
    with (HERE / 'records.json.gz').open('rb') as source:
        packed = source.read(4 * 1024 * 1024 + 1)
    require(len(packed) <= 4 * 1024 * 1024 and len(packed) == manifest['compressed_bytes'] and sha(packed) == manifest['sha256'], 'Bounded archive')
    with gzip.GzipFile(fileobj=io.BytesIO(packed), mode='rb') as source:
        raw = source.read(8 * 1024 * 1024 + 1)
    require(len(raw) <= 8 * 1024 * 1024 and len(raw) == manifest['uncompressed_bytes'], 'Bounded contents')
    packet = json.loads(raw)
    require(all(packet[k] == manifest[k] for k in ('format', 'source', 'tree', 'host')), 'Manifest identity')
    count = replay(packet, args.originals, args.binaries)
    require(count == manifest['records'], 'Record count')
    print(json.dumps({'passed': True, 'records': count, 'native_executed': False}))


if __name__ == '__main__':
    main()
