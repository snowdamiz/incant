#!/usr/bin/env python3
"""Verify data and identities only; never import or execute archived/native code."""
import argparse
import gzip
import hashlib
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
CPU = 'successor-007-h5kcam20'
GPU = 'gpu-003-r3chn8no'
STAGES = [('ordinary', name) for name in (
    'capture', 'reuse', 'miss', 'empty', 'disabled', 'catalog-seed',
    'catalog-reuse', 'catalog-invalid',
)] + [('cooked', name) for name in ('catalog-seed', 'catalog-reuse')]


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(data):
    require(data['schema'] == 1, 'Evidence schema')
    records = data['records']
    for path, row in records.items():
        relative = Path(path)
        require(not relative.is_absolute() and '..' not in relative.parts, 'Record path')
        raw = row['text'].encode()
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Record identity: ' + path)

    def read(path):
        return json.loads(records[path]['text'])

    def lines(path, prefix):
        return [json.loads(line[len(prefix):]) for line in records[path]['text'].splitlines()
                if line.startswith(prefix)]

    def count(text, name):
        values = re.findall(r'\b' + name + r': (\d+)\b', text)
        require(len(values) == 1, 'Unambiguous count: ' + name)
        return int(values[0])

    def clean(path):
        row = read(path + '/result.json')
        require(row['exit_code'] == 0 and row['error'] is None and row['cleanup_confirmed']
                and not row['cleanup_errors'], 'Successful process and cleanup: ' + path)
        return row

    outcome = data['outcome']
    require(outcome['tested_source_commit'] == '505d05e988eacc5a7e65a5b503193e97c00745d0', 'Tested source')
    for key in ('feature_default_enabled', 'frames_submitted', 'performance_measured',
                'all_optional_effects_qualified', 'driver_parser_allocation_bounded',
                'external_driver_services_accounted', 'retained_setuid_refusal_exercised'):
        require(outcome[key] is False, 'Preserve limitation: ' + key)
    require(outcome['cpu_counts'] == {'ordinary': 30, 'cooked': 30}
            and outcome['native_stages'] == 10, 'Scoped inventory')
    require((outcome['catalog_render'], outcome['catalog_compute'],
             outcome['strict_catalog_successes_per_route']) == (41, 2, 43), 'Catalog scope')
    source = read(CPU + '/source-manifest.json')
    require(source['head'] == outcome['tested_source_commit'] and len(source['files']) == 21981,
            'Complete tested source inventory')
    require(len(read(CPU + '/tools.json')) == 5, 'Five pinned tool identities')
    require(read('evidence/controls-complete.json')['all_passed'], 'Original containment controls')
    first = read('evidence/build-001/result.json')
    require(first['exit_code'] == 101 and first['cleanup_confirmed'], 'Preserve initial build failure')
    require('E0308' in records['evidence/build-001/stderr.log']['text'], 'Original NSURL diagnostic')
    failed_cpu = 'successor-003-dw8z9_pf/evidence/cpu-001'
    require(read(failed_cpu + '/result.json')['exit_code'] != 0, 'Preserve original CPU failure')
    require('18 passed; 9 failed' in records[failed_cpu + '/stdout.log']['text'], 'Original CPU counts')

    artifacts = read(CPU + '/evidence/artifacts.json')
    require(len(artifacts) == 4, 'Two distinct copied binaries per route')
    external = data['external_artifacts']
    identity = {}
    for route in ('ordinary', 'cooked'):
        selected = [a for a in artifacts if a['route'] == route]
        require({a['target'] for a in selected} == {'incant_render', 'metal_archives'}, 'Route artifacts')
        for artifact in selected:
            path = CPU + '/retained-artifacts/' + route + '/' + Path(artifact['path']).name
            require(external[path]['sha256'] == artifact['sha256']
                    and external[path]['bytes'] == artifact['bytes'], 'Copied executable identity')
            require(('cooked-native-shaders' in artifact['features']) == (route == 'cooked'), 'Feature mode')
            require(('cooked-metalfx-boundary' in artifact['features']) == (route == 'cooked'), 'Joined cooked aggregate')
            require({'metal-binary-archives', 'native-compilation'} <= set(artifact['features']), 'Native features')
        require(selected[0]['source_identity'] == selected[1]['source_identity'], 'Route source identity')
        identity[route] = selected[0]['source_identity']
        clean(CPU + '/evidence/build-' + route + '-001')
        path = CPU + '/evidence/cpu-' + route + '-001'
        clean(path)
        require('test result: ok. 30 passed; 0 failed' in records[path + '/stdout.log']['text'], 'Actual CPU tests')
        for kind in ('renderer',):
            row = clean(CPU + '/evidence/lint-' + kind + '-' + route + '-001')
            require(row['argv'][-2:] == ['-D', 'warnings'], 'Strict lint')
    require(identity['ordinary'] != identity['cooked'], 'Distinct feature/source keys')

    cpu_qualification = read(CPU + '/evidence/cpu-qualification.json')
    cpu_review = read(CPU + '/evidence/root-cpu-review.json')
    require(cpu_qualification['wrapper_exits'] == {'build': 0, 'cpu': 0, 'lint': 0}
            and cpu_qualification['cleanup_independently_verified'], 'CPU outer terminal cleanup')
    require(cpu_review['cpu_qualification_sha256'] == records[CPU + '/evidence/cpu-qualification.json']['sha256'], 'CPU root review binding')
    cpu_pids = set()
    for stage in cpu_qualification['stages']:
        path = CPU + '/evidence/' + stage['stage']
        row = clean(path)
        cpu_pids.update((row['supervisor_pid'], read(path + '/process.json')['pid']))
        cpu_pids.update(item['pid'] for item in row['observed_descendants'])
    require(cpu_pids == set(cpu_review['independently_absent_pids'])
            == set(cpu_qualification['owned_pids_absent']) and len(cpu_pids) == 51,
            'CPU independent absence inventory')

    require(outcome['hal_lints_reused_by_exact_identity'] and not outcome['metalfx_vendor_or_boundary_executed'], 'Composition scope')
    old_cpu = 'successor-006-cktqk5zy'
    old_source = {r['path']: r for r in read(old_cpu + '/source-manifest.json')['files']}
    current_source = {r['path']: r for r in source['files']}
    hal_paths = {p for p in current_source if p.startswith('third_party/wgpu-hal/')}
    require(len(hal_paths) == 94 and hal_paths == {p for p in old_source if p.startswith('third_party/wgpu-hal/')}, 'HAL inventory')
    for path in hal_paths | {'Cargo.lock', 'Cargo.toml', '.cargo/config.toml'}:
        require(old_source[path] == current_source[path], 'HAL and resolution source byte identity')
    require(read(old_cpu + '/tools.json') == read(CPU + '/tools.json'), 'Reused HAL tools')
    closures = read(CPU + '/evidence/hal-resolved-closures.json')
    proof = read(CPU + '/hal-reuse-proof.json')
    for route in ('ordinary', 'cooked'):
        require(closures[route]['old'] == closures[route]['new'] and len(closures[route]['new']) == 131, 'HAL exact resolved feature/dependency closure')
        item = next(x for x in proof['routes'] if x['route'] == route)
        require(item['old_metadata_sha256'] == closures[route]['old_metadata_sha256'] and item['new_metadata_sha256'] == closures[route]['new_metadata_sha256'], 'HAL closure source metadata binding')
        require(clean(old_cpu + '/evidence/lint-hal-' + route + '-001')['argv'][-2:] == ['-D', 'warnings'], 'Reused strict HAL lint')
    continuation = read(GPU + '/continuation.json')
    require(continuation['gpu_total_seconds'] == 180 - continuation['prior_gpu_elapsed_seconds'], 'No GPU budget reset')
    require(continuation['prior_gpu_elapsed_seconds'] == read('gpu-002-hiqqefw5/evidence/gpu-complete.json')['phase_elapsed_seconds'], 'Previous GPU phase charged')


    qualification = read(GPU + '/evidence/gpu-qualification.json')
    complete = read(GPU + '/evidence/gpu-complete.json')
    require(qualification['wrapper_exit'] == 0 and qualification['cleanup_independently_verified'], 'Outer terminal and cleanup')
    require(qualification['source_manifest_sha256'] == records[CPU + '/source-manifest.json']['sha256'], 'GPU source binding')
    require([(r['route'], r['stage']) for r in complete['results']] == STAGES, 'Exactly ten stages')
    require(complete['phase_elapsed_seconds'] <= continuation['gpu_total_seconds'] and complete['no_performance_claim']
            and not complete['window_or_frame_executed'], 'Bounded non-frame phase')
    root = read(GPU + '/evidence/root-gpu-review.json')
    require(root['qualification_sha256'] == records[GPU + '/evidence/gpu-qualification.json']['sha256']
            and root['all_ten_stages_pass'] and root['packet_and_artifacts_unchanged']
            and root['source'] == outcome['tested_source_commit']
            and not root['performance_claim'], 'Independent root review binding')
    pids = {qualification['supervisor_absent']}
    for route, stage in STAGES:
        path = GPU + '/evidence/gpu-' + route + '-' + stage + '-001'
        row = clean(path)
        require(row['elapsed_seconds'] < 31 and row['peak_rss_bytes'] <= 2 * 1024**3, 'Observed child budget')
        require(row['final_run_bytes'] <= 8 * 1024**3 and row['final_build_bytes'] <= 6 * 1024**3, 'Storage budget')
        require('test result: ok. 1 passed; 0 failed' in records[path + '/stdout.log']['text'], 'Actual native test success')
        pids.add(read(path + '/process.json')['pid'])
        ids = lines(path + '/stderr.log', 'METAL_ARCHIVE_IDENTITY ')
        require(len(ids) == 1 and ids[0]['stage'] == stage, 'Actual native identity')
        if stage.startswith('catalog-'):
            require(ids[0]['source'] == identity[route], 'Catalog source identity')
            canonical = {key: ids[0][key] for key in ('source', 'native', 'adapter')}
            require(canonical == complete['catalog_identities'][route], 'Actual device/OS/adapter identity')
            reports = lines(path + '/stderr.log', 'METAL_ARCHIVE_CATALOG ')
            require(len(reports) == 1, 'One actual catalog report')
            report = reports[0]
            require(report['source'] == identity[route]
                    and report['cache_identity'] == complete['catalog_keys'][route], 'Catalog identity key')
            text = report['report']
            require((count(text, 'render_attached'), count(text, 'compute_attached')) == (41, 2), 'Actual catalog coverage')
            if stage == 'catalog-reuse':
                require(report['require_hit'] and ids[0]['require_hit'], 'Real-catalog strict policy')
                require(count(text, 'require_succeeded') == 43 and 'write: NotAttempted' in text, 'Strict membership without persistence')
                require(count(text, 'render_added') == count(text, 'compute_added') == 0, 'No capture during strict reuse')
                require(count(text, 'require_failed') == count(text, 'disabled_require_refusals') == 0
                        and 'disabled: false' in text, 'No strict catalog refusals')
            else:
                require(not report['require_hit'] and 'write: Stored' in text, 'Ordinary capture publication')
                require((count(text, 'render_added'), count(text, 'compute_added')) == (41, 2), 'Complete capture')
                if stage == 'catalog-invalid':
                    require('read: Rejected' in text and 'MTLBinaryArchiveDomain' in text and 'code: 1' in text,
                            'Native parser refusal and recovery')
                else:
                    require('read: Missing' in text, 'Cold seed')
        else:
            require(ids[0]['identity'].startswith(identity[route] + ':'), 'Fixed fixture source')
            reports = lines(path + '/stderr.log', 'METAL_ARCHIVE_COUNTS ')
            require(len(reports) == 1, 'One fixed count report')
            text = reports[0]['counts']
            if stage == 'capture':
                require(count(text, 'render_added') == count(text, 'compute_added') == 1
                        and count(text, 'render_attached') == count(text, 'compute_attached') == 1, 'Both native descriptor types captured')
            elif stage == 'reuse':
                require(count(text, 'require_succeeded') == 2 and count(text, 'require_failed') == 0, 'Fixed strict hits')
            elif stage in ('miss', 'empty'):
                require(count(text, 'require_failed') == 2 and count(text, 'require_succeeded') == 0, 'Both native misses refused')
                require('AGXMetalG17X' in text and 'code: 3' in text, 'Native miss diagnostics')
                pipelines = lines(path + '/stderr.log', 'METAL_ARCHIVE_PIPELINES ')
                require(len(pipelines) == 2 and not pipelines[0]['require_failure'] and pipelines[1]['require_failure'], 'Ordinary warmup cannot substitute for membership')
            else:
                require(count(text, 'disabled_require_refusals') == 2
                        and count(text, 'render_attached') == count(text, 'compute_attached') == 0, 'Disabled strict refusal without native call')
        if not stage.startswith('catalog-') and stage != 'capture':
            require(count(text, 'render_added') == count(text, 'compute_added') == 0, 'Strict fixtures cannot capture')
        require(count(text, 'fallback_attempts') == count(text, 'addition_failures')
                == count(text, 'serialization_failures') == 0, 'No unexpected native fallback/failure')
    require(pids == set(root['owned_pids_independently_absent']) and len(pids) == 11, 'Independent process absence inventory')
    require(complete['catalog_keys']['ordinary'] != complete['catalog_keys']['cooked'], 'Distinct catalog keys')
    return {'verified_records': len(records), 'cpu_tests_per_mode': 30, 'native_stages': 10,
            'strict_catalog_hits_per_mode': 43, 'archived_code_executed': False}


def load():
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(manifest['data_file'] == 'records.json.gz', 'Fixed data filename')
    compressed = (HERE / 'records.json.gz').read_bytes()
    require(len(compressed) == manifest['compressed_bytes'] and digest(compressed) == manifest['sha256'], 'Compressed identity')
    require(manifest['uncompressed_bytes'] < 16 * 1024**2, 'Uncompressed bound')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        raw = stream.read(16 * 1024**2 + 1)
    require(len(raw) == manifest['uncompressed_bytes'], 'Uncompressed identity')
    data = json.loads(raw)
    # This imports data only. No archived Python, fixture, or native file executes.
    previous = HERE.parent / 'metal-binary-archives' / 'records.json.gz'
    require(digest(previous.read_bytes()) == manifest['predecessor_data_sha256'], 'Preserved predecessor packet')
    with gzip.open(previous, 'rb') as stream:
        prior_raw = stream.read(16 * 1024**2 + 1)
    require(len(prior_raw) <= 16 * 1024**2, 'Predecessor bound')
    prior = json.loads(prior_raw)
    require(not set(data['records']) & set(prior['records']), 'Distinct composition record namespace')
    data['records'] = prior['records'] | data['records']
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external-root', type=Path, help='Optionally hash retained native files; never execute them.')
    args = parser.parse_args()
    data = load()
    result = verify(data)
    if args.external_root is not None:
        for relative, expected in data['external_artifacts'].items():
            path = Path(relative)
            require(not path.is_absolute() and '..' not in path.parts, 'External relative path')
            path = args.external_root / path
            with path.open('rb') as stream:
                actual = hashlib.file_digest(stream, 'sha256').hexdigest()
            require(path.stat().st_size == expected['bytes'] and actual == expected['sha256'], 'External identity: ' + relative)
        result['external_hashes_verified'] = len(data['external_artifacts'])
    print(json.dumps(result, sort_keys=True))
