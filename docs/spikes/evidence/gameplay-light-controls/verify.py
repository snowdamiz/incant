#!/usr/bin/env python3
"""Offline light-control evidence replay; never runs an engine/compiler/device."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    manifest = json.loads((here / 'manifest.json').read_bytes())
    archive = (here / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(32 * 1024**2 + 1)
    require(len(payload) <= 32 * 1024**2, 'Replay archive exceeds bound')
    records = json.loads(payload)
    require(len(manifest['records']) == len(records)
            and set(records) == {r['key'] for r in manifest['records']}, 'Membership differs')
    raw = {}
    source_root = args.source_root.resolve() if args.source_root else here.parents[3]
    for row in manifest['records']:
        data = base64.b64decode(records[row['key']], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = data
        if args.originals:
            require(Path(row['original']).read_bytes() == data, 'Original differs: ' + row['key'])
        if row.get('code_input'):
            if args.source_root:
                require((source_root / row['source_path']).read_bytes() == data,
                        'Live code differs: ' + row['source_path'])
            if args.git_source:
                result = subprocess.run(['git', 'show', args.git_source + ':' + row['source_path']],
                                        cwd=source_root, check=True, capture_output=True)
                require(result.stdout == data, 'Git code differs: ' + row['source_path'])
    if args.binaries:
        for row in manifest['external_artifacts']:
            path = Path(row['path'])
            require(path.stat().st_size == row['bytes'] and digest(path.read_bytes()) == row['sha256'],
                    'External artifact differs: ' + row['path'])

    def data(key):
        return json.loads(raw[key])

    closeout = data('cpu-closeout-001.json')
    require(closeout['source'] == manifest['source'] and closeout['actual_unique_tests_passed'] == 32,
            'CPU source/count differs')
    summaries = []
    for row in closeout['results']:
        key = str(Path(row['path']).relative_to(Path(manifest['external_root'])))
        require(digest(raw[key]) == row['sha256'], 'Selected log differs')
        totals = [tuple(map(int, m)) for m in re.findall(
            rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw[key])]
        actual = tuple(sum(t[i] for t in totals) for i in range(3))
        require(actual == (row['passed'], row['failed'], row['ignored']), 'Actual CPU count differs')
        summaries.append(actual)
    require(sum(r[0] for r in summaries) == 32 and sum(r[2] for r in summaries) == 6, 'Coverage differs')
    require(b'Ran 20 tests' in raw['pure-00.log'] and b'OK' in raw['pure-00.log'], 'Python count differs')
    for row in data('pure-results-001.json'):
        key = Path(row['log']).name
        require(row['exit'] == 0 and digest(raw[key]) == row['sha256'], 'Pure command differs')

    cleanup = 0
    for attempt in range(1, 5):
        prefix = f'cpu-00{attempt}/'
        result = data(prefix + 'results.json')
        plan = data(f'cpu-plan-00{attempt}.json')
        require(result['source'] == plan['source'] and result['terminal'], 'CPU identity/terminal differs')
        require(result['proposal_sha256'] == digest(raw[f'cpu-plan-00{attempt}.json'])
                and result['supervisor_sha256'] == digest(raw[f'cpu-supervisor-00{attempt}.py']),
                'CPU protocol differs')
        require(not result['cleanup_errors'] and not result['scope_gpu_window'], 'CPU ownership/scope differs')
        require(all(r['original_stop_returned'] for r in result['cleanup_records']), 'CPU cleanup unconfirmed')
        cleanup += len(result['cleanup_records'])
        for name, row in result['logs'].items():
            require(len(raw[prefix + name]) == row['bytes'] and digest(raw[prefix + name]) == row['sha256'],
                    'Raw CPU log differs')
        expected = 'passed' if attempt == 4 else 'failed'
        require(result['status'] == expected, 'Failure chronology differs')
        exits = [r.get('exit') for r in result['commands']]
        require(all(x == 0 for x in exits) if attempt == 4 else exits[-1] == 101,
                'Command result differs')
    require(cleanup == closeout['mandatory_cleanup_records'] == 17, 'CPU cleanup count differs')
    require(b'2 passed; 1 failed' in raw['cpu-001/2.stdout.log'], 'QuickJS fixture failure missing')
    require(b'0 passed; 1 failed' in raw['cpu-002/1.stdout.log'], 'Node-order fixture failure missing')
    require(b'0 passed; 1 failed' in raw['cpu-003/2.stdout.log'], 'Native-label fixture failure missing')
    require(closeout['font_target_and_cpu_builder_released']
            and closeout['final_target_growth_bytes'] == 1269138025
            and closeout['final_target_growth_bytes'] <= 4 * 1024**3
            and closeout['minimum_free_bytes'] >= 6 * 1024**3, 'CPU resource receipt differs')

    guest = data('cpu-004/guest-inputs/result.json')
    bundle = data('cpu-004/guest-inputs/bundle/manifest.json')
    require(guest['guest_names'] == ['rust-light-controls', 'rust-light-scratch', 'rust-light-range', 'rust-light-owner']
            and len(guest['guests']) == 4 and guest['bundle'] == bundle, 'Actual guest provenance differs')
    require(digest(raw['cpu-004/guest-inputs/bundle/manifest.json']) == guest['manifest_sha256']
            and digest(raw['cpu-004/guest-inputs/rust-sdk.json']) == guest['sdk_provenance_sha256'],
            'Guest inputs differ')

    gpu = data('gpu-001/result.json')
    proposal = data('gpu-proposal-001.json')
    audit = data('gpu-feature-audit-001.json')
    artifact = audit['compiler_artifact']
    messages = [json.loads(line) for line in raw['cpu-004/3.stdout.log'].splitlines() if line.startswith(b'{')]
    require(artifact in messages and artifact['features'] == ['default', 'physics-deterministic']
            and artifact['profile']['test'], 'Actual GPU feature audit differs')
    require(gpu['source'] == manifest['source'] and gpu['status'] == 'passed' and gpu['exit'] == 0
            and gpu['terminal'] and gpu['cleanup_confirmed'] and not gpu['cleanup_errors']
            and not gpu['native_window_or_capture'], 'GPU result/scope differs')
    require(gpu['proposal_sha256'] == digest(raw['gpu-proposal-001.json'])
            and gpu['supervisor_sha256'] == digest(raw['gpu-supervisor-001.py']), 'GPU protocol differs')
    require(gpu['argv'] == proposal['argv'] and len(gpu['cleanup_records']) == 1
            and gpu['cleanup_records'][0]['original_stop_returned'], 'GPU command/cleanup differs')
    require(b'1 passed; 0 failed; 0 ignored' in raw['gpu-001/stdout.log']
            and b'Apple M5 Pro' in raw['gpu-001/stderr.log']
            and b'backend: Metal' in raw['gpu-001/stderr.log'], 'Actual GPU execution differs')
    for name, row in gpu['logs'].items():
        require(len(raw['gpu-001/' + name]) == row['bytes']
                and digest(raw['gpu-001/' + name]) == row['sha256'], 'GPU raw log differs')
    require(audit['binary_sha256'] == proposal['binary']['sha256'], 'GPU binary binding differs')
    print(f'PASS {len(records)} records; 20 Python/32 distinct Rust CPU/1 full Metal numerical control; '
          'three fixture failures retained; 18 owned groups cleaned; no window/performance claim')


if __name__ == '__main__':
    main()
