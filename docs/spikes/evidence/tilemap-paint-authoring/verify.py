#!/usr/bin/env python3
"""Data-only verification. Never extracts files or launches the engine."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(value, why):
    if not value:
        raise ValueError(why)


def file_bytes(path, cap):
    with path.open('rb') as stream:
        value = stream.read(cap + 1)
    require(len(value) <= cap, 'File exceeds bound: ' + str(path))
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--source-root', type=Path)
    args = parser.parse_args()
    manifest = json.loads(file_bytes(HERE / 'manifest.json', 256 * 1024))
    require(manifest['format'] == 'tilemap-authoring-evidence-v1', 'Manifest format')
    compressed = file_bytes(HERE / 'records.json.gz', 4 * 1024 * 1024)
    archive = manifest['archive']
    require(len(compressed) == archive['bytes'] and digest(compressed) == archive['sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        raw = stream.read(16 * 1024 * 1024 + 1)
    require(len(raw) <= 16 * 1024 * 1024 and len(raw) == archive['uncompressed_bytes'], 'Archive expansion bound')
    encoded = json.loads(raw)
    require(len(manifest['records']) == len({r['name'] for r in manifest['records']})
            and set(encoded) == {r['name'] for r in manifest['records']}, 'Exact record set')
    records = {}
    for row in manifest['records']:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Record identity: ' + row['name'])
        if args.originals:
            require(file_bytes(Path(row['original']), row['bytes']) == data, 'External original: ' + row['name'])
        records[row['name']] = data
    plan, result, close, source = [json.loads(records[n]) for n in ['plan.json', 'result.json', 'closeout.json', 'freeze.json']]
    require(all(x['source'] == manifest['source'] for x in [plan, result, close, source]), 'Producing source identity')
    external = manifest['external_source_manifest']
    require(any(r['path'] == external['original'] and r['sha256'] == external['sha256'] and r['bytes'] == external['bytes'] for r in plan['inputs']), 'Full source manifest binding')
    if args.originals:
        data = file_bytes(Path(external['original']), external['bytes'])
        require(len(data) == external['bytes'] and digest(data) == external['sha256'], 'External full source manifest')
    require(digest(records['plan.json']) == result['plan_sha256'] == close['plan_sha256']
            and digest(records['run.py']) == result['runner_sha256'] == close['runner_sha256']
            and digest(records['result.json']) == close['result_sha256'], 'Source/plan/runner/result chain')
    require(result['status'] == 'complete' and result['supervisor_terminal']
            and result['source_tools_protected_postverified'] and close['exec_exit'] == 0
            and close['all_absent'] and close['lease_released'], 'Successful terminal qualification')
    require(close['fresh_owned_absence'] and all(r['pid_absent'] and r['group_absent']
            for r in close['fresh_owned_absence']), 'Fresh process absence')
    require(len(plan['steps']) == len(result['stages']) == 3, 'Three selected stages')
    count = 0
    for i, (step, actual) in enumerate(zip(plan['steps'], result['stages'], strict=True)):
        require(actual['argv'] == step['argv'] and actual['exit'] == 0 and actual['status'] == 'complete', 'Stage command/outcome')
        if i < 2:
            raw_log = records[f'{i}.stdout.log'].decode()
            names = re.findall(r'^test (\S+) \.\.\. ok$', raw_log, re.M)
            summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw_log)
            require(sorted(names) == sorted(step['required_tests']) and len(names) == len(set(names))
                    and actual['tests'] == names and summaries
                    and sum(int(s[0]) for s in summaries) == len(names)
                    and all(s[1:] == ('0', '0') for s in summaries), 'Raw test membership')
            count += len(names)
    require(count == manifest['tests'] == close['executed_tests'] == 17, '17 actual tests')
    require(result['peak_allocation'] >= plan['retained_peak_bytes']
            and close['target_retained_peak_bytes'] <= plan['target_cap_bytes'] == 12 * 2**30
            and result['elapsed_seconds'] <= plan['bounds']['total_seconds'] == 950, 'Retained resource bounds')
    copies = [{k:row[k] for k in ['path','bytes','sha256']} for row in result['copies']]
    require(copies == manifest['binaries'] and len(copies) == 3, 'Copied artifact records')
    if args.binaries:
        for row in copies:
            data = file_bytes(Path(row['path']), row['bytes'])
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'External executable identity')
    if args.source_root:
        root = args.source_root.resolve()
        for row in manifest['code_inputs']:
            rel = Path(row['path'])
            require(not rel.is_absolute() and '..' not in rel.parts, 'Source relative path')
            path = (root / rel).resolve()
            require(path.is_relative_to(root) and digest(file_bytes(path, 16 * 1024 * 1024)) == row['sha256'], 'Source byte mismatch: ' + str(rel))
    print(f"PASS: {len(records)} records, 17 actual CPU tests, strict lint, 3 recorded artifact identities; optional checks originals={args.originals}, binaries={args.binaries}, listed_source={bool(args.source_root)}. No GPU/window/GUI claim.")


if __name__ == '__main__':
    main()
