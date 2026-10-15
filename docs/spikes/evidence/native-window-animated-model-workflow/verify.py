#!/usr/bin/env python3
"""Offline source/receipt replay; never starts a build, engine, RPC or GPU."""
import argparse
import base64
from datetime import datetime
import gzip
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import re
import subprocess

HERE = Path(__file__).resolve().parent
STAGES = ['manual_import_original', 'watched_keys', 'undo_original', 'redo_keys',
          'incompatible_duration_retained_replacement', 'source_recovered_keys']


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(manifest, data):
    read = lambda name: json.loads(data[name])
    freezes = {v: read(f'source-{v}/freeze.json') for v in ('001', '002')}
    for version, freeze in freezes.items():
        require(freeze['clean'] is True and freeze['source_sha'] == manifest['sources'][version],
                'Frozen producing source')
        for row in freeze['records']:
            name = f'source-{version}/files/' + row['path']
            if row['path'] in manifest['changed_paths']:
                require(sha(data[name]) == row['sha256'] and len(data[name]) == row['bytes'], 'Changed source copy')
    first = {r['path']: r['sha256'] for r in freezes['001']['records']}
    second = {r['path']: r['sha256'] for r in freezes['002']['records']}
    require([p for p in first if first[p] != second[p]] == ['editor/app/src/play/probe/asset_workflow/animation.rs'],
            'One source002 import correction')
    require(freezes['001']['frontend_inputs'] == freezes['002']['frontend_inputs']
            and len(freezes['002']['frontend_inputs']) == 145, 'Unchanged frontend source closure')
    require(manifest['changed_paths'] == freezes['002']['changed_paths'] == freezes['001']['changed_paths'],
            'Exact changed source inventory')
    inventory = [{k: r[k] for k in ('path', 'bytes', 'sha256')} for r in freezes['002']['records']]
    require(manifest['git_input_inventory'] == inventory + freezes['002']['frontend_inputs']
            and manifest['final_source_files'] == [r for r in inventory if r['path'] in manifest['changed_paths']],
            'Final source/inventory bound to freeze')
    nonnative = read('non-native-source001-results.json')
    require(nonnative['source_sha'] == manifest['sources']['001'] and len(nonnative['results']) == 4,
            'Historical non-native producing source')
    for row in nonnative['results']:
        require(row['exit_code'] == 0 and sha(data[Path(row['log']).name]) == row['log_sha256'], 'Non-native raw outcome')
    require(b'Ran 13 tests' in data['non-native-source001-0.raw.log'], 'Actual13 Python controls')
    for version, expected in [('001', [0, 101]), ('002', [0, 0, 0, 0])]:
        result = read(f'cpu-source{version}-results.json')
        launch = read(f'cpu-launch-{version}.json')
        proposal = read(f'cpu-proposal-{version}.json')
        require(result['source_sha'] == launch['source_sha'] == proposal['source_sha'] == manifest['sources'][version],
                'CPU producing source')
        require(sha(data[f'cpu-proposal-{version}.json']) == launch['proposal_sha256']
                and sha(data[f'cpu-supervisor-{version}.py']) == launch['supervisor_sha256'], 'CPU proposal/supervisor binding')
        require(all(not value for value in launch['ambient_override_values'].values()), 'Ambient override refusal')
        require(launch['build_env'] == {'RUSTUP_TOOLCHAIN': '1.99.0', 'CARGO_BUILD_JOBS': '2', 'RAYON_NUM_THREADS': '2'},
                'Pinned bounded build environment')
        require(all(row['exit_code'] == 0 for row in launch['identities'].values()), 'Actual host/tool identities')
        require([r['exit_code'] for r in result['results']] == expected, 'Failure and pass remain distinct')
        for index, row in enumerate(result['results']):
            require(row['argv'] == proposal['commands'][index]['argv'] and row['reaped'] is True
                    and sha(data[Path(row['log']).name]) == row['log_sha256'], 'Actual CPU command/reap/log')
    require(b'E0432' in data['cpu-source001-1.raw.log']
            and not re.search(rb'test result: ok\.', data['cpu-source001-1.raw.log']), 'Original compile failure; no Rust test pass')
    require(re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed', data['cpu-source002-0.raw.log']) == [(b'23', b'0')],
            'Actual23 focused Play CPU controls')
    frontend = read('frontend-source001.json')
    require(frontend['producing_source'] == manifest['sources']['001'] and len(frontend['files']) == 19
            and frontend['argv'] == ['npm', 'run', 'build', '--workspace', 'editor/ui'], 'Genuine historical frontend build')
    fixture = read('fixture-import-001/result.json')
    fixture_processes = read('fixture-import-001/processes.json')
    require(fixture['passed'] is True and fixture['children_reaped'] is True and fixture['native_processes'] == 4
            and len(fixture_processes) == 4 and all(r['exit_code'] == 0 and r['reaped'] is True for r in fixture_processes),
            'Actual four-child CPU importer control')
    fingerprints = fixture['fingerprints']
    require(len(set(fingerprints)) == 3 and [r['revision'] for r in fixture['imports']] == [1, 2, 3]
            and all(r['changed'] is True and r['asset']['id'] == fixture['stable_asset_id'] for r in fixture['imports']),
            'Stable identity, three valid changed source recipes')
    result = read('window-001/result.json')
    proof = read('window-001/native-window.json')
    require(result['window'] == proof and result['passed'] is True and result['asset_kind'] == 'animated-model', 'Actual app proof binding')
    for key in ('actual_app', 'unrelated_authored_bytes_preserved', 'durable_journal_recovery'):
        require(result[key] is True, key)
    for key in ('actual_window_presentations', 'animation_continued', 'committed_incompatible_refused',
                'world_identity_unchanged', 'module_counter_continued', 'manual_import_execute_history_handlers',
                'real_source_watcher', 'undo_not_overwritten', 'source_recovery', 'stale_generation_refused', 'stop_releases_world'):
        require(proof[key] is True, key)
    for receipt in (result, proof):
        require(all(receipt[k] is False for k in ('pixel_capture', 'appearance_review', 'performance_or_phase_gate')),
                'No pixel/appearance/performance/gate claim')
    require(proof['platform'] == 'macos' and 'Apple M5 Pro' in proof['adapter'] and 'backend: Metal' in proof['adapter'], 'Actual Mac Metal')
    require(type(proof['play_generation']) is int and proof['play_generation'] == 1
            and all(type(proof[k]) is int for k in ('paused_tick', 'final_tick'))
            and (proof['paused_tick'], proof['final_tick']) == (3, 4), 'Same generation and normal tick continuation')
    counter = proof['vm_counter']
    require(all(type(v) in (int, float) and math.isfinite(v) for v in counter.values())
            and counter == {'before_translation': 12, 'expected_delta': 8, 'observed_delta': 8}, 'Persistent module state')
    stages = proof['stages']
    require([s['stage'] for s in stages] == STAGES and all(type(s['tick']) is int and s['tick'] == 3
            and s['recipe_matches'] is True for s in stages), 'Exact six-stage paused workflow')
    require([s['frames'] for s in stages] == [9, 155, 160, 256, 305, 365]
            and all(type(s['frames']) is int for s in stages), 'Actual later presentations')
    require([s['history']['revision'] for s in stages] == [3, 4, 5, 6, 7, 8]
            and all(type(s['history']['revision']) is int for s in stages), 'Committed incompatible/recovery revisions')
    require([(s['history']['applied'], s['history']['redo']) for s in stages]
            == [(3, 0), (4, 0), (3, 1), (4, 0), (5, 0), (6, 0)], 'Undo/Redo history')
    require([s['accepted_fingerprint'] for s in stages] == [fingerprints[i] for i in (0, 1, 0, 1, 1, 1)], 'Live catalog selection')
    for index in (1, 4, 5):
        actor = stages[index]['history']['transactions'][-1]['actor']
        require(actor['origin'] == 'import' and actor['actor'] == 'source-watch', 'Real watcher provenance')
    before, after = (proof['animation_step'][k] for k in ('before', 'after'))
    for state in (before, after):
        require(set(state) == {'playback', 'completed', 'transition'} and state['completed'] is False and state['transition'] is None,
                'Complete fixture animation state')
        playback = state['playback']
        require(set(playback) == {'clip', 'seconds', 'speed', 'playing', 'looping'}
                and type(playback['clip']) is int and playback['clip'] == 0
                and type(playback['seconds']) in (int, float) and math.isfinite(playback['seconds'])
                and type(playback['speed']) in (int, float) and playback['speed'] == 1
                and playback['playing'] is True and playback['looping'] is True, 'Typed full playback snapshot')
    require(all(s['animation'] == before for s in stages)
            and abs(before['playback']['seconds'] - (.5 + 3/60)) < 1e-12
            and abs(after['playback']['seconds'] - (before['playback']['seconds'] + 1/60)) < 1e-12,
            'Paused snapshot retention and actual next-clock continuation')
    refusal = proof['incompatible_refresh_refusal']
    event = refusal['event']
    require(refusal['committed_revision'] == 7 and refusal['authored_fingerprint'] == fingerprints[2]
            and refusal['accepted_fingerprint'] == fingerprints[1] and refusal['native_code_available'] is False
            and 'code' not in refusal and event['level'] == 'error'
            and 'animation clip bindings or duration table changed' in event['message']
            and stages[4]['refresh_error'] == event and stages[4]['authored_fingerprint'] == fingerprints[2],
            'Actual committed incompatibility, retained live catalog and explicit console error')
    authored = read('window-001/fixture.json')
    require(authored['fixture_authored_through_cmd'] is True and authored['source_revision'] == 1, 'Command-authored fixture')
    for name, expected in authored['inputs'].items():
        require(sha(data['window-001/' + name]) == expected, 'Unrelated checkpoint/source retained')
    require(data['window-001/refresh.gltf'] == data['window-001/refresh-replacement.gltf']
            and len(data['window-001/project.journal.jsonl']) > authored['journal_bytes_before'], 'Watcher recovery and actual journal extension')
    journal = [json.loads(line) for line in data['window-001/project.journal.jsonl'].splitlines()]
    require([row['sequence'] for row in journal] == list(range(12)), 'Retained journal sequence')
    ledger = read('window-001/processes.json')
    launch = read('window-launch-001.json')
    require(sha(data['source-002/freeze.json']) == launch['source_freeze_sha256']
            and sha(data['cpu-source002-results.json']) == launch['cpu_results_sha256']
            and launch['frontend_producing_source'] == frontend['producing_source'], 'Window source/CPU/frontend launch binding')
    require(len(ledger) == result['native_processes'] == 6 and all(r['exit_code'] == 0 and r['reaped'] is True for r in ledger),
            'Six actual successful managed children')
    require(ledger[4]['argv'] == launch['expected_app_argv'], 'Exact owned app/fixture argv')
    recovered = json.loads(ledger[5]['stdout'])['result']
    require(type(recovered['revision']) is int and recovered['revision'] == result['recovered_revision'] == 8
            and next(a for a in recovered['project']['assets'].values() if a['path'] == 'refresh.gltf')['sha256'] == fingerprints[1],
            'Actual durable journal reopening')
    supervisor = read('window-supervisor-001-result.json')
    require(supervisor['source'] == launch['source_sha'] == manifest['sources']['002']
            and supervisor['exit_code'] == 0 and supervisor['monitor_joined'] is True
            and supervisor['managed_children_reaped'] is True and supervisor['native_processes'] == 6,
            'Terminal supervisor and source ownership')
    require(sha(data['window-launch-001.json']) == supervisor['launch_sha256']
            and sha(data['window-supervisor-001.py']) == launch['supervisor_sha256']
            and sha(data['window-supervisor-001.raw.log']) == supervisor['raw_log_sha256'], 'Window launch/raw byte binding')
    ready = read('window-owned-live-001.json')
    require(supervisor['owned_app_readiness'] == [ready] and ready['pid'] == 33370
            and ready['argv'] == ledger[4]['argv'], 'Observed owned live PID')
    require(result['editor_sha256'] == launch['bundle']['executable_sha256']
            and result['headless_sha256'] == fixture['headless_sha256']
            and sha(data['bundle/Info.plist']) == launch['bundle']['info_plist_sha256'], 'Actual executable/bundle identity selection')
    copies = {Path(r['copy']).name: r['sha256'] for r in read('cpu-source002-results.json')['copied_artifacts']}
    require(copies['1-incant_editor-source002'] == result['editor_sha256']
            and copies['1-incant_headless-source002'] == result['headless_sha256'], 'CPU-built actual app/headless copies')
    require(launch['bounds'] == {'app_timeout_seconds': 120, 'execution_deadline_seconds': 180,
            'startup_predicate_deadline_seconds': 20, 'minimum_free_bytes': 6442450944,
            'supervisor_log_stop_bytes': 67108864, 'no_retry': True}, 'Unchanged actual execution bounds')
    require(launch['expected_recipe_fingerprints'] == fingerprints, 'CPU and actual app fixture identity')
    closeout = read('root-cua-closeout-001.json')
    require(closeout['owned_window']['pid'] == 33370 and closeout['owned_window']['supervisor_finished_utc'] == supervisor['finished_utc'],
            'Root closeout bound to actual owned run')
    require(datetime.fromisoformat(supervisor['finished_utc']) < datetime.fromisoformat(closeout['cua'][0]['approximate_utc'])
            and 'excluded' in closeout['cua'][0]['result'] and 'No matching processes' in closeout['cua'][-1]['result'],
            'Late default launch excluded and closed')
    scope = manifest['outcome_scope']
    require(all(scope[k] is True for k in ('actual_animated_editor_window', 'real_handler_watcher_history_paths',
            'typed_animation_world_VM_continuation')) and all(scope[k] is False for k in ('pixel_capture',
            'GUI_import_history_click_coverage', 'appearance_acceptance', 'performance_or_phase_gate')), 'Scoped outcome claims')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='Rehash external copied executables, bundle plist and genuine frontend')
    parser.add_argument('--git-source', help='Immutable Git source for frozen compiler/config/frontend input comparison')
    parser.add_argument('--source-root', type=Path, help='Compare nine final probe/runner paths; no whole-runtime equality claim')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(manifest['format'] == 'incant-native-window-animated-model-workflow-v1', 'Known packet format')
    archive = (HERE / 'records.json.gz').read_bytes()
    require(len(archive) == manifest['archive_bytes'] and sha(archive) == manifest['archive_sha256'], 'Archive identity')
    require(0 < manifest['archive_decoded_bytes'] <= 16 * 1024**2, 'Bounded decoded archive')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(manifest['archive_decoded_bytes'] + 1)
    require(len(raw) == manifest['archive_decoded_bytes'], 'Decoded byte identity')
    data = {}
    for row in json.loads(raw)['records']:
        name = PurePosixPath(row['name'])
        require(not name.is_absolute() and '..' not in name.parts and str(name) not in data, 'Safe unique record')
        data[str(name)] = base64.b64decode(row['base64'], validate=True)
    require(set(data) == {r['name'] for r in manifest['records']}, 'Exact record set')
    for row in manifest['records']:
        require(len(data[row['name']]) == row['bytes'] and sha(data[row['name']]) == row['sha256'], 'Raw record identity')
        if args.originals:
            require(Path(row['original']).read_bytes() == data[row['name']], 'Retained original identity')
    if args.binaries:
        for row in manifest['external_artifacts']:
            raw = Path(row['path']).read_bytes()
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'External executable/frontend identity')
    for row in manifest['final_source_files']:
        if args.source_root:
            require(sha((args.source_root / row['path']).read_bytes()) == row['sha256'], 'Final probe/runner source')
    if args.git_source:
        require(re.fullmatch(r'[0-9a-f]{40}', args.git_source), 'Immutable full Git SHA')
        for row in manifest['git_input_inventory']:
            raw = subprocess.check_output(['git', 'show', args.git_source + ':' + row['path']], cwd=args.source_root or HERE, timeout=10)
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Frozen Git compiler/config/frontend input')
    replay(manifest, data)
    print(f'PASS {len(data)} records; Python13/PlayCPU23; original compile failure retained; '
          'actual app six stages, journal8, animation/world/VM continuation; no pixel/GUI/performance gate.')


if __name__ == '__main__':
    main()
