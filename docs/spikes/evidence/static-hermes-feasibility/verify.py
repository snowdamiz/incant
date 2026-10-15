#!/usr/bin/env python3
"""Verify retained data and hashes. Never import, extract, or execute native code."""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(data):
    require(data['schema'] == 1, 'Evidence schema')
    records = data['records']
    for path, row in records.items():
        raw = row['text'].encode()
        require(len(raw) == row['bytes'] and digest(raw) == row['sha256'], 'Record identity: ' + path)

    def read(path):
        return json.loads(records[path]['text'])

    outcome = read('evidence/outcome.json')
    require(outcome['upstream_commit'] == '1d2fc48c021fbda8eb14478379112ed67bed8f0f', 'Pinned upstream')
    require(outcome['native_build_attempts'] == 1 and not outcome['historical_build_aggregate_rss_qualified'], 'Preserve build monitoring limitation')
    require(not outcome['performance_measured'] and not outcome['adopted'] and outcome['default_engine'] == 'QuickJS', 'No adoption/performance inference')
    require(read('evidence/acquisition-complete.json')['source_files'] == 12761, 'Complete source inventory count')
    require(read('evidence/source-post-build-verification.json')['all_source_bytes_unchanged'], 'Source revalidation')
    artifact = data['external_artifacts']
    for name in ['hermes.tar.gz', 'cmake.tar.gz', 'ninja.zip']:
        metadata = read('evidence/' + name + '.json')
        require(artifact['downloads/' + name]['sha256'] == metadata['sha256'], 'Official archive binding')
    for path, row in read('evidence/replay-build-inventory.json').items():
        require(artifact['replay-build/' + path] == row, 'Compiler/runtime identity: ' + path)

    expected = read('inputs/prepared/node-control.json')['cases']
    actual = read('evidence/erased-language-results.json')['cases']
    names = ['recursive-props', 'weak-metadata', 'closure', 'caught-error', 'for-of', 'bulk-kernel', 'sdk-behavior']
    require([row['name'] for row in expected] == names == [row['name'] for row in actual], 'Exact seven fixture inventory')
    for control, result in zip(expected, actual):
        name = control['name']
        source = records['inputs/prepared/' + name + '.hermes.js']
        require(source['sha256'] == control['prepared_sha256'] == result['input_sha256'], 'Unchanged fixture bytes')
        require(records['inputs/prepared/' + name + '.original.ts']['sha256'] == control['input_sha256'], 'Original strict TypeScript identity')
        require(result['matches_node'] and result['values'] == control['node_values'], 'Exact expected fixture values')
        stdout = records['evidence/unit1-' + name + '-execute/stdout.log']['text']
        require([json.loads(line) for line in stdout.splitlines()] == control['node_values'], 'Actual fixture stdout')
        status = read('evidence/unit1-' + name + '-execute/result.json')
        require(status['exit_code'] == 0 and status['error'] is None and status['cleanup_confirmed'], 'Finite execution and cleanup')
        require(artifact['native-units-001/' + name]['sha256'] == result['binary_sha256'], 'Finite native binary binding')
        require(artifact['native-units-001/' + name + '.c']['sha256'] == result['generated_c_sha256'], 'Finite emitted C binding')
    sdk = read('evidence/c-unit-results.json')
    require(sdk['matches_node'] and sdk['values'] == expected[-1]['node_values'], 'SDK C-unit preflight')
    require(sdk['runtime_no_eval_no_jit'] and not sdk['console_bindings_linked'], 'C-unit owner configuration')
    require(sdk['host_sha256'] == records['inputs/c-unit-host.cpp']['sha256'], 'Exact C++ owner identity')
    require(not any('shermes_console' in arg for arg in read('evidence/unit1-sdk-preflight-link/result.json')['argv']), 'No console library in successor')

    typed = read('evidence/typed-results.json')['cases']
    require(len(typed) == 9 and all(row['exit_code'] == 1 and not row['accepted'] for row in typed), 'Original typed/declaration diagnostic outcomes')
    for row in typed:
        require('evidence/unit1-typed-' + row['name'] + '/stderr.log' in records, 'Retain every typed diagnostic')
        original = ('inputs/prepared/' + row['name'] + '.original.ts') if row['name'] in names else ('inputs/sdk/' + row['name'])
        require(records[original]['sha256'] == row['source_sha256'], 'Unchanged original typed/declaration input')
    first = read('evidence/execute-recursive-props/result.json')
    require(first['exit_code'] == -6 and first['cleanup_confirmed'], 'Preserve original aborted CLI execution')
    require('Lean VM does not support bytecode generation' in records['evidence/execute-recursive-props/stderr.log']['text'], 'Original console bootstrap diagnostic')

    negative = read('evidence/negative-deadline-once/result.json')
    require(outcome['negative_deadline_attempts'] == 1, 'Exactly one negative process')
    require(negative['timeout_seconds'] == 2 and negative['exit_code'] == -15 and negative['error'] == "TimeoutError('Stage wall deadline')", 'Observed external deadline termination')
    require(negative['cleanup_confirmed'] and 2 <= negative['elapsed_seconds'] < 3, 'Bounded negative cleanup receipt')
    marker = read('evidence/negative-deadline-once/stdout.log')
    require(marker == {'stage': 'deadline-armed', 'watch_ms': 10}, 'Actual armed marker without claimed internal exit')
    compiled = read('evidence/deadline-compiled.json')
    require(compiled['binary_sha256'] == artifact['native/deadline-probe']['sha256'], 'Negative native binary binding')
    require(compiled['emitted_c_sha256'] == records['native/deadline.c']['sha256'], 'Negative generated C binding')
    require(compiled['host_sha256'] == records['inputs/deadline-host.cpp']['sha256'], 'Negative native host identity')
    require(not outcome['engine_rollback_proven'] and not outcome['negative_deadline_internal_interruption_observed'], 'External termination is not rollback')
    require(read('evidence/cleanup-complete.json')['all_stage_cleanups_confirmed'], 'Managed resources released')
    return {'verified_records': len(records), 'finite_exact_values': 7, 'typed_diagnostics': 9, 'negative_processes': 1, 'native_artifacts_executed': False}


def load():
    manifest = json.loads((HERE / 'manifest.json').read_text())
    compressed = (HERE / manifest['data_file']).read_bytes()
    require(len(compressed) == manifest['compressed_bytes'] and digest(compressed) == manifest['sha256'], 'Compressed evidence identity')
    require(manifest['uncompressed_bytes'] < 4 * 1024**2, 'Evidence size bound')
    with gzip.open(HERE / manifest['data_file'], 'rb') as stream:
        raw = stream.read(4 * 1024**2 + 1)
    require(len(raw) == manifest['uncompressed_bytes'], 'Uncompressed evidence size')
    return manifest, json.loads(raw)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--external-packet', type=Path, help='Optionally hash the external native packet; never extracts or executes it.')
    args = parser.parse_args()
    manifest, payload = load()
    result = verify(payload)
    if args.external_packet is not None:
        expected = manifest['external_native_packet']
        with args.external_packet.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        require(args.external_packet.stat().st_size == expected['bytes'] and actual == expected['sha256'], 'External native packet identity')
        result['external_packet_hash_verified'] = True
    print(json.dumps(result, sort_keys=True))
