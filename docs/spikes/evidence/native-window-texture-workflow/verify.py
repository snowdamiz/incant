#!/usr/bin/env python3
"""Offline receipt replay only: never starts the editor, GPU, build or RPC."""
import argparse
import base64
import gzip
import hashlib
import json
import math
from pathlib import Path, PurePosixPath

HERE = Path(__file__).resolve().parent
STAGES = ['manual_import_red', 'watched_blue', 'undo_red', 'redo_blue',
          'malformed_source_retained_blue', 'source_recovered_blue']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(manifest, data):
    read = lambda name: json.loads(data[name])
    result = read('window-004/result.json')
    proof = read('window-004/native-window.json')
    require(result['window'] == proof and result['passed'] is True, 'Passed native receipt binding')
    for name in ('actual_app', 'unrelated_authored_bytes_preserved', 'durable_journal_recovery'):
        require(result[name] is True, name)
    for name in ('actual_window_presentations', 'world_identity_unchanged', 'module_counter_continued',
                 'manual_import_execute_history_handlers', 'real_source_watcher', 'undo_not_overwritten',
                 'source_recovery', 'stale_generation_refused', 'stop_releases_world'):
        require(proof[name] is True, name)
    for value in (result, proof):
        require(all(value[k] is False for k in ('pixel_capture', 'appearance_review', 'performance_or_phase_gate')),
                'No unearned appearance/performance/gate claim')
    require(proof['platform'] == 'macos' and 'backend: Metal' in proof['adapter'], 'Actual Mac Metal')
    require(type(proof['play_generation']) is int and proof['play_generation'] == 1, 'Generation identity')
    require(type(proof['paused_tick']) is int and type(proof['final_tick']) is int
            and (proof['paused_tick'], proof['final_tick']) == (3, 4), 'Observed tick continuation')
    counter = proof['vm_counter']
    require(all(type(v) in (int, float) and math.isfinite(v) for v in counter.values()), 'Finite counter values')
    require(counter == {'before_translation':12, 'expected_delta':8, 'observed_delta':8}, 'Persistent module counter')
    stages = proof['stages']
    require([s['stage'] for s in stages] == STAGES, 'Exact six-stage workflow')
    require(all(s['recipe_matches'] is True and type(s['tick']) is int and s['tick'] == 3 for s in stages),
            'Same paused world/catalog')
    require(all(type(s['frames']) is int for s in stages)
            and [s['frames'] for s in stages] == [9,146,151,247,296,326], 'Actual later surface presentations')
    hashes = [s['accepted_fingerprint'] for s in stages]
    require(all(len(h) == 64 and all(c in '0123456789abcdef' for c in h) for h in hashes), 'Recipe hashes')
    require(hashes[0] == hashes[2] and hashes[1] == hashes[3] == hashes[4] == hashes[5]
            and hashes[0] != hashes[1], 'Red/blue/history/refusal/recovery catalog selection')
    require(all(type(s['history']['revision']) is int for s in stages)
            and [s['history']['revision'] for s in stages] == [3,4,5,6,6,6], 'Committed revisions')
    require([(s['history']['applied'],s['history']['redo']) for s in stages]
            == [(3,0),(4,0),(3,1),(4,0),(4,0),(4,0)], 'Undo/Redo and refusal history')
    require(stages[1]['history']['transactions'][-1]['actor']['origin'] == 'import', 'Real watcher provenance')
    require(proof['malformed_source_refusal']['code'] == 'asset.import', 'Structured source refusal')
    fixture = read('window-004/fixture.json')
    for name, expected in fixture['inputs'].items():
        require(sha(data['window-004/' + name]) == expected, 'Unrelated authored bytes: ' + name)
    require(data['window-004/refresh.png'] == data['window-004/refresh-blue.png'], 'Identical source recovery')
    require(len(data['window-004/project.journal.jsonl']) > fixture['journal_bytes_before'], 'Journal extended')
    ledger = read('window-004/processes.json')
    require(len(ledger) == result['native_processes'] == 6 and all(r['reaped'] is True and r['exit_code'] == 0 for r in ledger),
            'Six successful managed/reaped children')
    launch = read('window-launch-004.json')
    require(ledger[4]['argv'] == launch['expected_app_argv'], 'Exact owned fixture app')
    recovered = json.loads(ledger[-1]['stdout'])['result']
    require(type(recovered['revision']) is int and recovered['revision'] == result['recovered_revision'] == 6, 'Durable revision')
    assets = [a for a in recovered['project']['assets'].values() if a['path'] == 'refresh.png']
    require(len(assets) == 1 and assets[0]['sha256'] == hashes[-1], 'Durable selected recipe')
    supervisor = read('window-supervisor-004-result.json')
    require(supervisor['exit_code'] == 0 and supervisor['managed_children_reaped'] is True
            and supervisor['monitor_joined'] is True, 'Terminal supervisor ownership')
    require(supervisor['owned_app_readiness'][0]['pid'] == 7779, 'Owned live PID')
    require(sha(data['window-supervisor-004.py']) == launch['supervisor_sha256'], 'Supervisor binding')
    cua = read('window-004/root-cua-actions.json')
    require(cua['owned_child_pid'] == 7779 and cua['owned_exit_code'] == 0, 'Root activation receipt')
    for name, expected in cua['bound_receipts'].items():
        require(sha(data[name]) == expected, 'Root bound receipt: ' + name)
    require('7870' in json.dumps(cua) and 'excluded' in json.dumps(cua), 'Excluded later default instance')
    for attempt in ('001','002','003'):
        require(read('window-' + attempt + '/result.json')['passed'] is False, 'Prior failure retained')
    diagnostic = json.loads(read('window-003/native-window.json')['error'])
    require(diagnostic['last_play_status']['tick'] == 0 and diagnostic['last_play_status']['presentedFrames'] == 1193
            and diagnostic['window']['focused'] == {'Ok':False}, 'Prior focus starvation retained')
    for source, count in (('003',15),('004',18),('005',18),('006',20)):
        rows = read('cpu-source' + source + '-results.json')['results']
        cpu = next(r for r in rows if r['kind'] == 'probe-feature-cpu')
        name = Path(cpu['log']).name
        require(cpu['exit_code'] == 0 and f'{count} passed; 0 failed'.encode() in data[name], 'Scoped CPU count:' + source)
    require(b'Ran 7 tests' in data['cpu-source002-0.raw.log'] and b'Ran 8 tests' in data['cpu-source004-0.raw.log'], 'Python counts')
    require(b'E0599' in data['cpu-source002-3.raw.log'] and b'items_after_test_module' in data['cpu-source004-3.raw.log'],
            'Original compile/lint failures retained')
    final_cpu = read('cpu-source006-results.json')['results']
    require(all(r['exit_code'] == 0 and r['reaped'] is True for r in final_cpu), 'Final editor qualification')
    closure = read('main284-main285-source-closure.json')
    require(closure['delta_paths'] == manifest['main285_composition']['delta_paths']
            and all(p == 'STATUS.md' or p.startswith('docs/') for p in closure['delta_paths']), 'Docs-only main285 closure')
    return {'passed':True, 'records':len(data), 'stages':6, 'paused_tick':3, 'final_tick':4,
            'recovered_revision':6, 'cpu_controls':20, 'python_controls':8,
            'scope':'Offline replay; no new app/GPU/build/measurement'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='Also rehash retained external binaries/frontend')
    parser.add_argument('--source-root', type=Path, help='Compare seven current probe/runner paths; does not assert whole-runtime equality')
    args = parser.parse_args()
    manifest = json.loads((HERE/'manifest.json').read_text())
    require(manifest['format'] == 'incant-native-window-texture-workflow-v1', 'Known packet format')
    archive = (HERE/'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'], 'Archive binding')
    with gzip.open(HERE/'records.json.gz', 'rb') as stream:
        raw = stream.read(32*1024**2+1)
    require(len(raw) <= 32*1024**2, 'Bounded archive')
    data = {}
    for row in json.loads(raw):
        name = row['name']
        require(not PurePosixPath(name).is_absolute() and '..' not in PurePosixPath(name).parts and name not in data, 'Safe unique record')
        require(row['encoding'] in ('utf8','base64'), 'Known record encoding')
        data[name] = row['data'].encode('utf-8') if row['encoding'] == 'utf8' else base64.b64decode(row['data'], validate=True)
    require(set(data) == {r['name'] for r in manifest['records']}, 'Complete record set')
    for row in manifest['records']:
        require(sha(data[row['name']]) == row['sha256'] and len(data[row['name']]) == row['bytes'], 'Raw record binding:' + row['name'])
        if args.originals:
            require(sha(Path(row['original']).read_bytes()) == row['sha256'], 'Retained original:' + row['name'])
    if args.binaries:
        for row in manifest['retained_artifacts']:
            require(sha(Path(row['path']).read_bytes()) == row['sha256'], 'Retained artifact:' + row['path'])
    if args.source_root:
        for row in manifest['source_records']:
            require(sha((args.source_root/row['path']).read_bytes()) == row['sha256'], 'Source input:' + row['path'])
    print(json.dumps(replay(manifest, data)))


if __name__ == '__main__':
    main()
