#!/usr/bin/env python3
"""Replay bounded retained data only; never execute an engine, compiler or archive code."""
import argparse
import base64
import gzip
import hashlib
import io
import json
import math
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent


def require(value, why):
    if not value:
        raise ValueError(why)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def read(path, limit):
    with path.open('rb') as stream:
        data = stream.read(limit + 1)
    require(len(data) <= limit, 'Byte bound: ' + str(path))
    return data


def parent_suite(text, inventory):
    # The serial parent owns the unfinished result prefix. Intentional crash
    # children print headers, but never supply named parent results or summaries.
    require(inventory and len(inventory) == len(set(inventory)), 'Parent inventory')
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    require(lines[0] == f'running {len(inventory)} test' +
            ('s' if len(inventory) != 1 else ''), 'Parent header')
    names, pending, finished = [], None, False
    for line in lines[1:]:
        require(not finished, 'Output after parent summary')
        if pending is not None:
            if line == 'running 1 test':
                continue
            require(line == 'ok', 'Ambiguous nested parent result')
            names.append(pending)
            pending = None
            continue
        match = re.fullmatch(r'test (\S+) \.\.\.(?: (ok))?', line)
        if match:
            require(len(names) < len(inventory) and match[1] == inventory[len(names)],
                    'Missing, duplicate or reordered parent')
            if match[2]:
                names.append(match[1])
            else:
                pending = match[1]
            continue
        match = re.fullmatch(r'test result: ok\. (\d+) passed; (\d+) failed; '
                            r'(\d+) ignored; (\d+) measured; (\d+) filtered out; '
                            r'finished in [0-9.]+s', line)
        require(match and tuple(map(int, match.groups())) == (len(inventory), 0, 0, 0, 0)
                and names == inventory, 'Parent count/failure/ignored mismatch')
        finished = True
    require(finished and pending is None, 'Incomplete parent suite')
    return names


def load_records(manifest, originals):
    archive = manifest['archive']
    compressed = read(HERE / 'records.json.gz', 4 * 2**20)
    require(len(compressed) == archive['bytes'] and sha(compressed) == archive['sha256'],
            'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        raw = stream.read(24 * 2**20 + 1)
    require(len(raw) <= 24 * 2**20 and len(raw) == archive['uncompressed_bytes'], 'Expansion bound')
    encoded = json.loads(raw)
    rows = manifest['records']
    require(len(rows) == len({r['name'] for r in rows}) and set(encoded) == {r['name'] for r in rows},
            'Exact record membership')
    records = {}
    for row in rows:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record: ' + row['name'])
        if originals:
            require(read(Path(row['original']), row['bytes']) == data, 'Original: ' + row['name'])
        records[row['name']] = data
    return records


def verify(manifest, records):
    require(manifest['format'] == 'registered-rust-source-evidence-v1', 'Format')
    get = lambda name: json.loads(records[name])
    sources = manifest['sources']
    f1, f2, f3 = [get(f'source-{n:03}/freeze.json') for n in (1, 2, 3)]
    require([f['source'] for f in (f1, f2, f3)] ==
            [sources['feature'], sources['composed'], sources['qualified']], 'Source generations')
    require(sha(records['source-001/source.patch']) == f1['patch_sha256'] and
            sha(records['source-002/composition.patch']) == f2['composition_patch_sha256'] and
            sha(records['source-003/source.patch']) == f3['patch_sha256'], 'Frozen patches')
    previous = None
    all_copies = {}
    expected_calls = [3, 6, 13, 12]
    for index, number in enumerate(range(2, 6)):
        label, packet = f'cpu-{number:03}', f'cpu-packet-{number:03}'
        plan, grant, result, owner, close, protected = [get(f'{folder}/{file}') for folder, file in [
            (packet, 'plan.json'), (packet, 'root-grant.json'), (label, 'result.json'),
            (label, 'owner.json'), (label, 'closeout.json'), (label, 'protected-release.json')]]
        require(sha(records[f'{packet}/plan.json']) == grant['plan_sha256'] and
                sha(records[f'{packet}/run.py']) == grant['runner_sha256'], 'Granted packet')
        source = sources['composed'] if number < 4 else sources['qualified']
        require(plan['source'] == grant['source'] == result['source'] == close['source'] == source,
                'Attempt source')
        require(result['passed'] == close['passed'] == (number == 5) and
                result['source_changed'] is False and result['target_retirements'] == [],
                'Retained outcome/source/cache scope')
        require(result['protected_rechecked'] and close['protected_rechecked'] and close['absence']
                and all(r['pid_absent'] and r['group_absent'] for r in close['absence']), 'Cleanup')
        require(len(owner['calls']) == expected_calls[index] and
                all(c['finished'] and c['cleanup_confirmed'] for c in owner['calls']), 'Owned calls')
        require(len(result['steps']) == [3, 6, 12, 12][index], 'Exact completed step count')
        for call in owner['calls']:
            for channel in ('stdout', 'stderr'):
                stream = call[channel]
                require(sha(records[f"{label}/{stream['path']}"]) == stream['sha256'], 'Raw stream')
        for step, call in zip(result['steps'], owner['calls'], strict=number != 4):
            require(step['argv'] == call['argv'] and step['exit'] == call['exit'], 'Recorded actual command')
        expected_exits = [0] * expected_calls[index]
        if number == 3:
            expected_exits[-1] = 101
        if number == 4:
            expected_exits[-1] = None
        require([c.get('exit') for c in owner['calls']] == expected_exits, 'Exact child outcomes')
        handoff = plan['handoff']
        for field in ('postcleanup_seconds', 'including_deleted_history_seconds'):
            require(math.isclose(result[field], handoff[field] + result['seconds'], abs_tol=1e-6)
                    and result[field] == close[field], 'Historical charges')
        require(close['retained_peak_bytes'] >= handoff['retained_peak_bytes'] and
                close['current_bytes'] <= close['retained_peak_bytes'] <= 18 * 2**30 and
                plan['bounds']['target_cap_bytes'] == 18 * 2**30 and
                result['seconds'] <= plan['bounds']['aggregate_seconds'], 'Resource bounds')
        if previous:
            require(handoff['postcleanup_seconds'] == previous['postcleanup_seconds'] and
                    handoff['retained_peak_bytes'] == previous['retained_peak_bytes'], 'No history reset')
        previous = close
        cargo_rows = []
        for call in owner['calls']:
            for line in records[f"{label}/{call['stdout']['path']}"].decode().splitlines():
                if line.startswith('{'):
                    row = json.loads(line)
                    if row.get('reason') == 'compiler-artifact':
                        cargo_rows.append(row)
        for copy in result['copies']:
            require(copy['cargo'] in cargo_rows, 'Copied executable Cargo provenance')
            all_copies[copy['path']] = {k: copy[k] for k in ('path', 'bytes', 'sha256')}
        require(len(protected) == [100, 102, 107, 108][index], 'Protected artifact count')
        if number > 2:
            terminal = get(f'{label}/terminal-absence.json')
            require(terminal['actual_exec']['exit'] == (0 if number == 5 else 1) and
                    all(terminal['absence']['pids'].values()) and all(terminal['absence']['groups'].values()),
                    'Observed terminal/absence')
    r2, r3, r4, r5 = [get(f'cpu-{n:03}/result.json') for n in range(2, 6)]
    require(r2['error'] == "ValueError('Named observable controls missing or duplicate')" and
            'command-compile-incant_cmd-execute failed' in r3['error'] and
            r4['error'] == "TimeoutExpired(['/bin/ps', '-axo', 'pid=,pgid=,rss='], 5)", 'Original failures')
    require('test result: FAILED. 32 passed; 1 failed; 0 ignored;' in
            records['cpu-003/005.stdout.log'].decode(), 'Actual audit failure retained')
    total = 0
    for suite in manifest['suites']:
        inventory = re.findall(r'^(\S+): test$', records[suite['inventory']].decode(), re.M)
        names = parent_suite(records[suite['stdout']].decode(), inventory)
        require(names == suite['tests'] and len(names) == suite['count'], 'Exact successful parent controls')
        call = get(suite['owner'])['calls'][suite['call']]
        require(call['exit'] == 0 and call['argv'][0] == suite['binary'], 'Executed test copy')
        total += len(names)
    require(total == manifest['tests'] == 72, '72 scoped controls')
    for copy in get('cpu-packet-004/retained-integrations.json'):
        require(copy['producing_source'] == sources['composed'], 'Retained integration provenance')
        require(copy['cargo'] in [json.loads(line) for line in
                records['cpu-003/003.stdout.log'].decode().splitlines() if line.startswith('{')],
                'Retained integration original Cargo row')
        all_copies[copy['path']] = {k: copy[k] for k in ('path', 'bytes', 'sha256')}
    require(sorted(all_copies.values(), key=lambda r: r['path']) == manifest['binaries']
            and len(all_copies) == 9, 'Nine external copied artifacts')
    require(len(r5['generated']) == 83 and len(r5['steps']) == 12 and
            all(s['exit'] == 0 for s in r5['steps']), 'Final lint/generation')
    require(r5['schema_delta']['added'] == ['ModuleSourceRead.schema.json', 'RustModuleSource.schema.json']
            and not r5['schema_delta']['removed'] and len(r5['schema_delta']['changed']) == 7,
            'Generated schema delta')
    applied = get('consumers-001/applied.json')
    require(applied['producer'] == sources['qualified'] and len(applied['files']) == 10, 'Generated successor')
    generated = {row['path']: row for row in r5['generated']}
    for row in applied['files']:
        data = records['generated/' + row['path']]
        require(sha(data) == row['after'] == generated[row['generated_original']]['sha256'], 'Applied generated bytes')
    before = get('generation-before/agent-tools.json')
    after = get('generated/schemas/agent-tools.json')
    for name in ('module_source_read', 'module_build', 'module_build_status', 'module_build_cancel'):
        a = [tool for tool in before if tool['name'] == name]
        b = [tool for tool in after if tool['name'] == name]
        require(len(a) == 1 and a == b, 'Legacy tool descriptor changed: ' + name)
    consumer, owner, terminal = [get('consumers-001/' + name) for name in
                                ('result.json', 'output/owner.json', 'terminal-absence.json')]
    require(consumer['passed'] and not consumer['target_access'] and owner['passed'] and
            len(owner['calls']) == len(consumer['steps']) == 6 and
            terminal['generated_successor'] == sources['generated'] and terminal['actual_exec']['exit'] == 0 and
            all(terminal['pids'].values()) and all(terminal['groups'].values()), 'Consumers and closure')
    for step, call in zip(consumer['steps'], owner['calls'], strict=True):
        require(step['exit'] == call['exit'] == 0 and step['argv'] == call['argv'] and
                call['cleanup_confirmed'], 'Consumer command')
        for channel in ('stdout', 'stderr'):
            row = call[channel]
            require(sha(records['consumers-001/output/' + row['path']]) == row['sha256'], 'Consumer raw log')
    review = get('reviews/registered-rust-cpu005-independent-20261018.json')
    require(review['passed'] and review['source'] == sources['qualified'] and
            review['result_sha256'] == sha(records['cpu-005/result.json']), 'Independent native replay')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='Also compare retained external raw records/manifests')
    parser.add_argument('--binaries', action='store_true', help='Hash the nine external copied executables; never run them')
    parser.add_argument('--source-root', type=Path, help='Hash listed producer code inputs and all generated contracts')
    args = parser.parse_args()
    manifest = json.loads(read(HERE / 'manifest.json', 512 * 1024))
    records = load_records(manifest, args.originals)
    verify(manifest, records)
    if args.originals:
        for row in manifest['external_source_manifests']:
            data = read(Path(row['path']), row['bytes'])
            require(sha(data) == row['sha256'], 'Full historical source manifest')
            source = json.loads(data)
            require(source['source'] in (manifest['sources']['composed'], manifest['sources']['qualified']),
                    'Historical manifest source')
            if source['source'] == manifest['sources']['qualified']:
                require(all(source['files'][r['path']] == r['sha256'] for r in manifest['code_inputs']),
                        'Listed inputs versus full qualified manifest')
    if args.binaries:
        for row in manifest['binaries']:
            data = read(Path(row['path']), row['bytes'])
            require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'External executable')
    if args.source_root:
        root = args.source_root.resolve()
        for row in manifest['code_inputs'] + manifest['generated_inputs']:
            relative = Path(row['path'])
            require(not relative.is_absolute() and '..' not in relative.parts, 'Source path')
            path = (root / relative).resolve()
            require(path.is_relative_to(root) and sha(read(path, 16 * 2**20)) == row['sha256'],
                    'Listed source/contract drift: ' + str(relative))
    print(f"PASS: {len(records)} records; 72 scoped parent controls; retained three failures; "
          "12 final qualification calls and six consumer checks. Nine recorded executable identities. "
          f"Optional originals={args.originals}, binaries={args.binaries}, listed_source={bool(args.source_root)}. "
          "No complete source/build/apply workflow or device/performance claim.")


if __name__ == '__main__':
    main()
