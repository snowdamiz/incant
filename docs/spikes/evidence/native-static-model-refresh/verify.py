#!/usr/bin/env python3
"""Offline provenance/receipt/pixel replay; never runs an engine, GPU or build."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

HERE = Path(__file__).resolve().parent
STAGES = ['manual_import_original', 'watched_replacement', 'undo_original', 'redo_replacement',
          'malformed_source_retained_replacement', 'source_recovered_replacement']


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def colored(raw, channel, low=0, high=160, legacy=False):
    return sum(low <= i % 160 < high and p[channel] > 150 and p[3] > 200
               and (p[2-channel] < 50 if legacy else
                    all(p[channel]-p[other] > 128 for other in range(3) if other != channel))
               for i, p in enumerate(zip(*[iter(raw)]*4)))


def replay(manifest, data):
    read = lambda name: json.loads(data[name])
    # Every CPU claim is bound to the original exact command and raw log.
    producer = read('producer/cpu-source001-results.json')
    require(producer['source_sha'] == manifest['producer_cpu_source'], 'Producer executed source')
    for row, counts in zip(producer['results'][:3], ([5,6], [2,5], [20])):
        log = data['producer/' + Path(row['log']).name]
        require(row['exit_code'] == 0 and row['reaped'] is True, 'Producer CPU terminal')
        require([int(v) for v in re.findall(rb'test result: ok\. (\d+) passed; 0 failed', log)] == counts,
                'Original producer test counts')
        require(sha(log) == row['log_sha256'], 'Producer raw log identity')
    failed = producer['results'][3]
    require(failed['exit_code'] == 101 and b'function `record` is never used' in
            data['producer/' + Path(failed['log']).name], 'Original lint failure retained')
    corrected = read('producer/cpu-source002-results.json')
    require(corrected['source_sha'] == manifest['producer_lint_source'] and len(corrected['results']) == 2,
            'Test-only annotation correction source')
    for row in corrected['results']:
        require(row['exit_code'] == 0 and row['reaped'] is True, 'Corrected two lint scopes')
        require(sha(data['producer/' + Path(row['log']).name]) == row['log_sha256'], 'Corrected lint log')
    for version, count in [('002',4), ('003',2), ('004',2)]:
        rows = read('root/cpu-source' + version + '-results.json')['results']
        require(len(rows) == count and all(r['exit_code'] == 0 and r['reaped'] is True for r in rows),
                'Root joined CPU/build/lint:' + version)
        for row in rows:
            log = data['root/' + Path(row['log']).name]
            require(sha(log) == row['log_sha256'], 'Root command raw log identity')
            if row['kind'] == 'probe-feature-cpu':
                require(b'test result: ok. 20 passed; 0 failed' in log, 'Joined editor20')
    # Failed attempts remain failures. The corrected final classifier is replayed
    # over raw bytes, separately from the actual final-source GPU003 pass.
    pixels = {}
    for run, passed in [('001',False), ('002',False), ('003',True)]:
        prefix = 'root/model-gpu-' + run + '/'
        receipt = read(prefix + 'result.json')
        require(receipt['passed'] is passed and receipt['exit_code'] == (0 if passed else 101)
                and receipt['reaped'] is True, 'Distinct GPU outcome:' + run)
        require(receipt['plan']['source'] == manifest['gpu_executed_sources'][run], 'GPU source identity')
        require(sha(data[prefix + 'output.log']) == receipt['log_sha256'], 'GPU raw log identity')
        require('backend: Metal' in data[prefix + 'output.log'].decode(), 'Actual Metal backend')
        if not passed:
            require(b'assertion failed: old_red >= 80' in data[prefix + 'output.log'], 'Original classifier refusal')
        if run == '001':
            continue
        buffers = {name:data[prefix + 'pixels/' + name + '.rgba']
                   for name in ('initial','rejected_retained','replacement')}
        require(all(len(v) == 160*96*4 for v in buffers.values()), 'Bounded raw RGBA')
        require(buffers['initial'] == buffers['rejected_retained'], 'Exact old pixels after refusal')
        require((colored(buffers['initial'],0), colored(buffers['replacement'],2,0,70),
                 colored(buffers['replacement'],2,100,160)) == (200,1152,1152), 'Display-color area counts')
        require(colored(buffers['initial'],2) == colored(buffers['replacement'],0) == 0,
                'No wrong-color/background acceptance')
        require(colored(buffers['initial'],0,legacy=True) == colored(buffers['replacement'],2,legacy=True) == 0,
                'Retained old absolute secondary cap rejected intended display colors')
        for name, raw in buffers.items():
            summary = read(prefix + 'pixels/' + name + '.json')
            rgba = list(zip(*[iter(raw)]*4))
            require(summary['minimum_rgba'] == [min(p[c] for p in rgba) for c in range(4)]
                    and summary['maximum_rgba'] == [max(p[c] for p in rgba) for c in range(4)], 'Pixel extrema')
        pixels[run] = buffers
    require(pixels['002'] == pixels['003'], 'Classifier repair did not change fixture/rendered raw data')
    require(b'final_tick=2 frames=6 presents=6' in data['root/model-gpu-003/output.log']
            and b'test result: ok. 1 passed; 0 failed' in data['root/model-gpu-003/output.log'], 'Full GPU003 control')
    # Actual native-window proof remains separate from headless numerical pixels.
    result = read('root/window-001/result.json')
    proof = read('root/window-001/native-window.json')
    require(result['passed'] is True and result['window'] == proof and result['actual_app'] is True,
            'Actual owned native-window result')
    for key in ('actual_window_presentations','world_identity_unchanged','module_counter_continued',
                'manual_import_execute_history_handlers','real_source_watcher','undo_not_overwritten',
                'source_recovery','stale_generation_refused','stop_releases_world'):
        require(proof[key] is True, 'Actual workflow control:' + key)
    for value in (result,proof):
        require(all(value[k] is False for k in ('pixel_capture','appearance_review','performance_or_phase_gate')),
                'Scope boundaries')
    require(proof['asset_kind'] == 'model' and proof['platform'] == 'macos' and 'backend: Metal' in proof['adapter'],
            'Model native Mac identity')
    require((proof['paused_tick'],proof['final_tick']) == (3,4)
            and proof['vm_counter'] == {'before_translation':12,'expected_delta':8,'observed_delta':8}, 'Continuing module state')
    stages = proof['stages']
    require([s['stage'] for s in stages] == STAGES and [s['frames'] for s in stages] == [9,159,164,260,309,338],
            'Six actual later presentations')
    require(all(s['tick'] == 3 and s['recipe_matches'] is True for s in stages), 'Paused world/catalog')
    hashes = [s['accepted_fingerprint'] for s in stages]
    require(hashes[0] == hashes[2] and hashes[1] == hashes[3] == hashes[4] == hashes[5] and hashes[0] != hashes[1],
            'Import/Undo/Redo/refusal/recovery selection')
    require([s['history']['revision'] for s in stages] == [3,4,5,6,6,6]
            and stages[1]['history']['transactions'][-1]['actor']['origin'] == 'import', 'Real command history/watcher')
    require(proof['malformed_source_refusal']['code'] == 'asset.import', 'Structured malformed-source refusal')
    fixture = read('root/window-001/fixture.json')
    for name, expected in fixture['inputs'].items():
        require(sha(data['root/window-001/' + name]) == expected, 'Unrelated authored input retention')
    require(data['root/window-001/refresh.gltf'] == data['root/window-001/refresh-replacement.gltf'], 'Recovered source')
    require(len(data['root/window-001/project.journal.jsonl']) > fixture['journal_bytes_before'], 'Durable journal extended')
    ledger = read('root/window-001/processes.json')
    launch = read('root/window-launch-001.json')
    require(len(ledger) == result['native_processes'] == 6 and all(r['reaped'] is True and r['exit_code'] == 0 for r in ledger),
            'Six managed/reaped children')
    require(ledger[4]['argv'] == launch['expected_app_argv'], 'Exact owned app argv')
    recovered = json.loads(ledger[-1]['stdout'])['result']
    require(recovered['revision'] == result['recovered_revision'] == 6, 'Journal reopening revision')
    asset = next(a for a in recovered['project']['assets'].values() if a['path'] == 'refresh.gltf')
    require(asset['sha256'] == hashes[-1], 'Durable model identity')
    supervisor = read('root/window-supervisor-001-result.json')
    correction = read('root/window-extra-launch-correction-001.json')
    require(supervisor['exit_code'] == 0 and supervisor['managed_children_reaped'] is True
            and supervisor['monitor_joined'] is True and supervisor['owned_app_readiness'][0]['pid'] == 90263,
            'Owned supervisor terminal')
    require(correction['owned_fixture_pid'] == 90263 and correction['extra_no_argument_pid'] == 91320
            and correction['owned_fixture_pass_unchanged'] is True and correction['extra_excluded_from_fixture_evidence'] is True
            and correction['extra_process_gone'] is True, 'Later extra-app correction/exclusion/cleanup')
    closure = read('source-closure.json')
    require(closure['producer_cpu_to_final_production_equal'] is True
            and closure['window_to_final_delta'] == ['crates/incant_render_gpu_host/tests/game/model_refresh.rs']
            and not closure['ui_bridge_package_delta_since_genuine_frontend'], 'Source relevance distinctions')
    require(all(p == 'STATUS.md' or p.startswith('docs/') for p in
                closure['producer_to_main288_delta'] + closure['final_to_ledger_delta']), 'Docs-only joins')
    inventory = read(manifest['compiler_inventory']['record'])
    lookup = {r['path']:r for r in inventory['records']}
    require(inventory['source'] == manifest['final_code_source'] and len(lookup) == manifest['compiler_inventory']['count'],
            'Final frozen full-input hash inventory')
    for row in manifest['source_records']:
        require(sha(data[row['record']]) == row['sha256'] == lookup[row['path']]['sha256'], 'Frozen changed source')
    return {'passed':True,'records':len(data),'producer_cpu':38,'joined_editor_cpu':20,'gpu_original_failures':2,
            'gpu_final_passed':True,'raw_counts':[200,1152,1152],'window_stages':6,'journal_revision':6,
            'scope':'Offline replay only; no new build, GPU, app, RPC or performance execution'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='Rehash retained external binaries/frontend')
    parser.add_argument('--git-source', nargs='?', const='executed', help='Compare changed source/seam bytes at a Git ref; omitted value uses final executed code')
    parser.add_argument('--source-root', type=Path, default=HERE.parents[3], help='Repository for optional read-only Git checks')
    args = parser.parse_args()
    manifest = json.loads((HERE/'manifest.json').read_text())
    require(manifest['format'] == 'incant-native-static-model-refresh-v1', 'Known packet format')
    archive = (HERE/'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'] and len(archive) == manifest['archive_bytes'], 'Archive binding')
    with gzip.open(HERE/'records.json.gz','rb') as stream:
        raw = stream.read(32*1024**2+1)
    require(len(raw) <= 32*1024**2, 'Bounded decoded archive')
    data = {}
    for row in json.loads(raw):
        name = row['name']
        require(not PurePosixPath(name).is_absolute() and '..' not in PurePosixPath(name).parts and name not in data,
                'Safe unique record names')
        require(row['encoding'] in ('utf8','base64'), 'Known encoding')
        data[name] = row['data'].encode('utf8') if row['encoding'] == 'utf8' else base64.b64decode(row['data'],validate=True)
    require(set(data) == {r['name'] for r in manifest['records']}, 'Complete record set')
    for row in manifest['records']:
        require(sha(data[row['name']]) == row['sha256'] and len(data[row['name']]) == row['bytes'], 'Raw record identity')
        if args.originals:
            require(sha(Path(row['original']).read_bytes()) == row['sha256'], 'Retained original:' + row['name'])
    if args.originals:
        for row in manifest['retained_source_copies']:
            require(sha(Path(row['path']).read_bytes()) == row['sha256'], 'Retained frozen source copy')
    if args.binaries:
        for row in manifest['retained_artifacts']:
            require(sha(Path(row['path']).read_bytes()) == row['sha256'], 'Retained derived artifact')
    if args.git_source:
        ref = manifest['final_code_source'] if args.git_source == 'executed' else args.git_source
        commit = subprocess.check_output(['git','-C',str(args.source_root),'rev-parse','--verify','--end-of-options',ref+'^{commit}'],text=True).strip()
        delta = subprocess.check_output(['git','-C',str(args.source_root),'diff','--name-only',manifest['final_code_source'],commit],text=True).splitlines()
        require(all(p == 'STATUS.md' or p.startswith('docs/') for p in delta), 'Git ref has only documentation delta from executed code')
        for row in manifest['source_records'] + manifest['compiler_seam_records']:
            body = subprocess.check_output(['git','-C',str(args.source_root),'show',commit+':'+row['path']])
            require(sha(body) == row['sha256'], 'Git source input:' + row['path'])
    print(json.dumps(replay(manifest,data)))


if __name__ == '__main__':
    main()
