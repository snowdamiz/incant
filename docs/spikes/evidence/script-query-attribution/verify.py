#!/usr/bin/env python3
"""Bounded data-only replay. Archived code and binaries are never executed."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(value):
    return hashlib.sha256(value).hexdigest()


def read(path, cap):
    with Path(path).open('rb') as stream:
        raw = stream.read(cap + 1)
    require(len(raw) <= cap, 'Input bound: ' + str(path))
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--git-source', type=Path)
    args = parser.parse_args()
    m = json.loads(read(HERE / 'manifest.json', 256 * 1024))
    require(m['format'] == 'script-query-attribution-cpu-evidence-v1', 'Manifest format')
    raw = read(HERE / 'records.json.gz', 4 * 1024**2)
    require(len(raw) == m['archive']['bytes'] and sha(raw) == m['archive']['sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(raw)) as stream:
        raw = stream.read(24 * 1024**2 + 1)
    require(len(raw) <= 24 * 1024**2 and len(raw) == m['archive']['uncompressed_bytes'], 'Expansion bound')
    encoded = json.loads(raw)
    require(len(m['records']) == len({r['name'] for r in m['records']})
            and set(encoded) == {r['name'] for r in m['records']}, 'Exact record set')
    records = {}
    for row in m['records']:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record identity')
        if args.originals:
            require(read(row['original'], row['bytes']) == data, 'External original: ' + row['name'])
        records[row['name']] = data

    def value(name):
        return json.loads(records[name])

    require(value('source-preflight-correction.json')['original_native_execution'] is False,
            'Initial source was unexecuted')
    copies = []
    executions = 0
    test_names = set()
    results = []
    for generation, total, stage_count in [('002', 36, 5), ('003', 6, 3)]:
        prefix = 'cpu-' + generation + '/'
        packet = 'cpu-packet-' + generation + '/'
        p, r, c = [value(prefix + n) for n in ['plan.json', 'result.json', 'closeout.json']]
        f = value('source-' + generation + '/freeze.json')
        require(p['source'] == r['source'] == c['source'] == f['source'] == m['sources'][generation], 'Producing source')
        require(sha(records[prefix + 'plan.json']) == r['plan_sha256']
                and sha(records[packet + 'run.py']) == r['runner_sha256']
                and sha(records[prefix + 'result.json']) == c['result_sha256'], 'Command/result chain')
        g = value(packet + 'root-cpu-grant.json')
        require(g['source'] == p['source'] and g['plan_sha256'] == r['plan_sha256']
                and g['runner_sha256'] == r['runner_sha256'], 'Recorded authorization binding')
        require(r['supervisor_terminal'] and c['all_absent'] and c['checks']
                and all(x['pid_absent'] and x['group_absent'] for x in c['checks']), 'Terminal cleanup')
        require(len(r['stages']) == stage_count, 'Executed stage count')
        count = 0
        for i, actual in enumerate(r['stages']):
            step = p['steps'][i]
            require(actual['argv'] == step['argv'], 'Exact native command')
            failure = generation == '002' and i == 4
            require(actual['status'] == ('failed' if failure else 'complete')
                    and (actual['exit'] != 0 if failure else actual['exit'] == 0), 'Actual stage outcome')
            if 'required_tests' in step:
                text = records[prefix + f'{i}.stdout.log'].decode()
                names = re.findall(r'^test (\S+) \.\.\. ok$', text, re.M)
                summary = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
                require(sorted(names) == sorted(step['required_tests']) and len(names) == len(set(names))
                        and names == actual['tests'] and summary
                        and sum(int(s[0]) for s in summary) == len(names)
                        and all(s[1:] == ('0', '0') for s in summary), 'Raw selected test membership')
                count += len(names)
                test_names.update(names)
        require(count == total, 'Selected test count')
        executions += count
        require(r['status'] == ('failed' if generation == '002' else 'complete')
                and c['actual_exit'] == (1 if generation == '002' else 0), 'Preserved supervisor status')
        if generation == '003':
            require(r['source_tools_protected_postverified'], 'Final identity checks')
            for index, selected in [(1, True), (2, False)]:
                messages = [json.loads(line) for line in records[prefix + f'{index}.stdout.log'].decode().splitlines() if line.startswith('{')]
                rows = [x for x in messages if x.get('reason') == 'compiler-artifact']
                for package in ['incant_profile', 'incant_script', 'incant_game_player']:
                    flags = [x['features'] for x in rows if x['target']['name'] == package]
                    require(flags and (any('diagnostic-script-queries' in f for f in flags) if selected
                                       else all('diagnostic-script-queries' not in f for f in flags)), 'Selected compiler feature mode')
        require(r['peak_allocation'] >= p['retained_peak_bytes']
                and r['peak_allocation'] <= p['target_cap_bytes'] == 12 * 2**30
                and r['elapsed_seconds'] <= p['bounds']['total_seconds'], 'Retained resource bounds')
        source_name = packet + 'source.json'
        require(any(x['path'] == p['source_manifest'] and x['sha256'] == sha(records[source_name]) for x in p['inputs']), 'Full source manifest binding')
        copies.extend({k:x[k] for k in ['path', 'bytes', 'sha256']} for x in r['copies'])
        results.append(r)
    require(executions == 42 and len(test_names) == 33 and copies == m['binaries']
            and len(copies) == 5, '42 executions/33 names/five retained binaries')
    require(results[0]['elapsed_seconds'] + results[1]['elapsed_seconds'] <= 900
            and results[1]['prior_native_seconds'] == results[0]['cumulative_native_seconds'], 'No time reset')
    fast = value('integration-fast/result.json')
    owner = value('integration-fast/output/owner.json')
    closeout = value('integration-fast/closeout.json')
    require(fast['source'] == closeout['source'] == m['integration_source']
            and fast['passed'] and owner['passed'] and closeout['exit'] == 0
            and closeout['groups_absent'] and len(fast['stages']) == len(owner['calls']) == 10,
            'Separate integrated development checks')
    for stage, call in zip(fast['stages'], owner['calls'], strict=True):
        require(stage['command'] == call['argv'] and stage['exit'] == call['exit'] == 0
                and call['finished'] and call['cleanup_confirmed'], 'Development command outcome')
        for stream in ['stdout', 'stderr']:
            require(sha(records['integration-fast/output/' + call[stream]['path']])
                    == call[stream]['sha256'], 'Development raw stream')
    if args.binaries:
        for row in copies:
            raw = read(row['path'], row['bytes'])
            require(len(raw) == row['bytes'] and sha(raw) == row['sha256'], 'External executable identity')
    if args.source_root:
        root = args.source_root.resolve()
        source = value('cpu-packet-003/source.json')
        require(source['source'] == m['sources']['003'], 'Current source manifest')
        for name, digest in source['files'].items():
            rel = Path(name)
            require(not rel.is_absolute() and '..' not in rel.parts, 'Source path')
            path = (root / rel).resolve()
            require(path.is_relative_to(root) and sha(read(path, 64 * 1024**2)) == digest, 'Current compiler/source input: ' + name)
    if args.git_source:
        for generation, commit in m['sources'].items():
            require(re.fullmatch('[0-9a-f]{40}', commit), 'Commit syntax')
            for row in value('source-' + generation + '/freeze.json')['files']:
                rel = Path(row['path'])
                require(not rel.is_absolute() and '..' not in rel.parts, 'Git source path')
                raw = subprocess.check_output(['git', 'show', commit + ':' + row['path']], cwd=args.git_source, timeout=10)
                require(sha(raw) == row['sha256'], 'Historical source bytes')
    print(f'PASS: {len(records)} records; 33 test names/36 feature-qualified controls/42 passing executions; final on/off strict lint; original lint failure retained; five recorded binary identities; ten integrated development checks. Optional originals={args.originals}, binaries={args.binaries}, full_source={bool(args.source_root)}, historical_source={bool(args.git_source)}. No Core/performance/window/GPU claim.')


if __name__ == '__main__':
    main()
