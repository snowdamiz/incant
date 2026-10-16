#!/usr/bin/env python3
"""Replay CPU evidence only. No archived program, player, GPU or window runs."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
BASE = '/Users/andreyyurlov/.codex/incant-evidence/native-wake-acquisition-boundaries-001/'
SOURCE = '51eab2289a8a14ce1da82ac90ddeabcee9d1470f'


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def file_sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'Duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs,
                      parse_constant=lambda _: require(False, 'Nonfinite JSON'))


def read_bounded(path, maximum):
    with Path(path).open('rb') as stream:
        raw = stream.read(maximum + 1)
    require(len(raw) <= maximum, 'Input exceeds byte cap: ' + str(path))
    return raw


def load():
    manifest = decode(read_bounded(HERE / 'manifest.json', 256 * 1024))
    compressed = read_bounded(HERE / 'records.json.gz', 2 * 1024**2)
    require(len(compressed) == manifest['archive_bytes'] <= 2 * 1024**2
            and sha(compressed) == manifest['archive_sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed), mode='rb') as stream:
        raw = stream.read(8 * 1024**2 + 1)
    require(len(raw) == manifest['expanded_bytes'] <= 8 * 1024**2, 'Expanded archive bound')
    packet = decode(raw)
    require(packet['format'] == 'incant-wake-acquisition-CPU-records-v1', 'Record format')
    records = {}
    for row in packet['records']:
        require(row['path'] not in records, 'Duplicate record')
        encoded = row['text'].encode()
        require(len(encoded) == row['bytes'] and sha(encoded) == row['sha256'], 'Saved record identity')
        records[row['path']] = encoded
    require(len(records) == manifest['record_count'], 'Record inventory')
    return manifest, records


def validate(manifest, records):
    get = lambda name: decode(records[BASE + name])
    first, final = [get('cpu-' + attempt + '/results.json') for attempt in ('001', '002')]
    p1, p2 = [get('cpu-proposal-' + attempt + '.json') for attempt in ('001', '002')]
    require(manifest['source'] == p1['source'] == p2['source'] == first['source'] == final['source'] == SOURCE,
            'Exact native producing source')
    for n, result, plan in [('001', first, p1), ('002', final, p2)]:
        grant = get('root-cpu-grant-' + n + '.json')
        require(result['proposal_sha256'] == grant['plan_sha256'] == sha(records[BASE + 'cpu-proposal-' + n + '.json'])
                and result['supervisor_sha256'] == grant['supervisor_sha256'] == sha(records[BASE + 'cpu-supervisor-' + n + '.py']),
                'Execution bindings')
        close = get('cpu' + n + '-closeout.json')
        require(result['supervisor_terminal'] and all(g['cleanup_confirmed'] for g in result['owned_groups'])
                and close['independent_PID_and_group_absence']
                and all(row['PID_absent'] and row['group_absent'] for row in close['processes']), 'Positive cleanup')
        require(close['supervisor_result']['sha256'] == sha(records[BASE + 'cpu-' + n + '/results.json']), 'Closeout binding')
        for name, row in result['logs'].items():
            raw = records[BASE + 'cpu-' + n + '/' + name]
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'Raw log identity')
    require(first['status'] == 'failed' and first['exit'] == 1 and 'Exact focused CPU counts' in first['error']
            and len(first['stages']) == 1 and first['stages'][0]['exit'] == 0
            and p1['steps'][0]['expected_test_counts'] == [19, 0], 'Count-refusal provenance')
    stdout = records[BASE + 'cpu-001/0.stdout.log'].decode()
    names = re.findall(r'^test (\S+) \.\.\. ok$', stdout, re.M)
    diagnosis = get('cpu001-count-diagnosis.json')
    require(re.findall(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;', stdout) == [('30', '0')]
            and len(names) == len(set(names)) == 30 and sorted(names) == diagnosis['all_exact_test_names']
            and sorted(names) == p2['retained_CPU_stage']['required_tests']
            and [row['count'] for row in diagnosis['source_test_membership']] == [6, 13, 11], 'Actual30-test membership')
    require(final['status'] == 'complete' and final['exit'] == 0 and len(final['stages']) == 2
            and all(row['exit'] == 0 and row['terminal_reaped'] for row in final['stages'])
            and p2['steps'] == p1['steps'][1:]
            and final['retained_CPU_controls_replayed_without_execution'] == 30, 'Only pending strict lints')
    artifact, = final['copied_artifacts']
    require(artifact == manifest['binary'] and artifact['sha256'] == p2['retained_CPU_stage']['artifact_sha256']
            and set(artifact['features']) == set(p1['steps'][0]['expected_features'])
            and artifact['bytes'] == 37007664 and artifact['tests_passed_in_CPU001'] == 30, 'Copied actual feature audit')
    close = get('cpu002-closeout.json')
    require(close['origin_target_allocated_bytes'] == 64799776768
            and close['peak_target_allocated_bytes'] == 65147740160 <= close['effective_target_ceiling_bytes'] == 65336647680
            and close['shared_target_ceiling_bytes'] == 65948704768
            and close['cumulative_seconds'] <= 1800 and close['cumulative_log_bytes'] == 564071
            and close['copied_artifact_bytes'] <= 64 * 1024**2, 'Cumulative resource account')
    require(not manifest['native_window_observed'] and not manifest['performance_gate_passed']
            and not manifest['pacing_cause_determined'], 'Unsupported live/performance claim')
    review = decode(records[manifest['independent_review']['path']])
    require(sha(records[manifest['independent_review']['path']]) == manifest['independent_review']['sha256']
            and review['source'] == SOURCE and review['actual_cpu_controls'] == 30
            and review['strict_lints_passed'] == 2 and not review['gpu_window_performance_claim'], 'Independent review scope')
    require(file_sha(REPO / 'tools/presentation_host_stages.py') == manifest['v1_analyzer_sha256'], 'v1 analyzer changed')
    for name, digest in manifest['previous_packets'].items():
        require(file_sha(REPO / name) == digest, 'Historical v1 evidence changed')
    freeze = get('source-001/freeze.json')
    require(freeze['source'] == SOURCE and len(freeze['inputs']) == manifest['source_inventory_records'] == 4346
            and p1['freeze_sha256'] == p2['freeze_sha256'] == sha(records[BASE + 'source-001/freeze.json']), 'Frozen input identity')
    return freeze, artifact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', metavar='REV')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest, records = load()
    freeze, artifact = validate(manifest, records)
    if args.originals:
        for name, raw in records.items():
            require(read_bounded(name, len(raw)) == raw, 'Retained original changed: ' + name)
    if args.binaries:
        path = Path(artifact['copy'])
        require(path.stat().st_size == artifact['bytes'] and file_sha(path) == artifact['sha256'], 'Retained executable changed')
    if args.git_source:
        revision = subprocess.check_output(['git', 'rev-parse', args.git_source], cwd=REPO, text=True).strip()
        require(revision == SOURCE, 'Use exact historical native producer for Git replay')
        requests = ''.join(revision + ':' + row['path'] + '\n' for row in freeze['inputs']).encode()
        raw = subprocess.run(['git', 'cat-file', '--batch'], input=requests, cwd=REPO,
                             check=True, stdout=subprocess.PIPE, timeout=60).stdout
        offset = 0
        for row in freeze['inputs']:
            end = raw.index(b'\n', offset)
            header = raw[offset:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'Git source blob absent')
            size = int(header[2])
            body = raw[end + 1:end + 1 + size]
            require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'Git source differs: ' + row['path'])
            offset = end + 2 + size
        require(offset == len(raw), 'Trailing Git source response')
    if args.source_root:
        # The outcome document necessarily differs from its source-only version.
        # Validate executable/tool inputs, explicitly excluding documentation.
        for row in freeze['inputs']:
            if not row['path'].startswith('docs/'):
                path = args.source_root / row['path']
                require(path.stat().st_size == row['bytes'] and file_sha(path) == row['sha256'],
                        'Current executable/tool source differs: ' + row['path'])
    print('PASS: ' + str(manifest['record_count']) + ' records;30actual CPU controls;2strict lints; count refusal retained; no live pacing verdict')


if __name__ == '__main__':
    main()
