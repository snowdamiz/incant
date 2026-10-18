#!/usr/bin/env python3
"""Replay inert source/receipts only. Never import archive code or execute artifacts."""
import argparse
import base64
import copy
import gzip
import hashlib
import io
import json
import math
from pathlib import Path
import re
import tempfile

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


def test_result(stdout, inventory, required, ignored=None):
    """Serial parent ownership, with only predeclared ignored controls admitted.

    Nested crash headers may finish a pending parent only with standalone ok.
    Ignored names/reasons/counts are exact, disjoint from required successes,
    and returned separately by the caller; they never become passed controls.
    """
    ignored = {} if ignored is None else ignored
    require(inventory and len(set(inventory)) == len(inventory)
            and set(required) <= set(inventory) and set(ignored) <= set(inventory)
            and not set(required) & set(ignored), 'Invalid parent inventory')
    lines = [line.strip() for line in stdout.splitlines() if line.strip()]
    require(lines and lines[0] == f'running {len(inventory)} test'
            + ('s' if len(inventory) != 1 else ''), 'Parent suite header/count')
    names, passed, pending, finished = [], [], None, False
    for line in lines[1:]:
        require(not finished, 'Output after parent suite summary')
        if pending is not None:
            if line == 'running 1 test':
                continue
            require(line == 'ok', 'Nested or incomplete parent result')
            names.append(pending)
            passed.append(pending)
            pending = None
            continue
        match = re.fullmatch(r'test (\S+) \.\.\.(?: (ok|ignored)(?:, (.*))?)?', line)
        if match:
            name, state, reason = match.groups()
            require(len(names) < len(inventory) and name == inventory[len(names)],
                    'Parent test missing, duplicate, or out of order')
            if name in ignored:
                require(state == 'ignored' and reason == ignored[name], 'Expected ignored status/reason')
                names.append(name)
            else:
                require(state in (None, 'ok') and reason is None, 'Unexpected ignored/result')
                if state == 'ok':
                    names.append(name)
                    passed.append(name)
                else:
                    pending = name
            continue
        summary = re.fullmatch(r'test result: ok\. (\d+) passed; (\d+) failed; '
            r'(\d+) ignored; (\d+) measured; (\d+) filtered out; finished in [0-9.]+s', line)
        require(summary is not None and tuple(map(int, summary.groups()))
                == (len(inventory) - len(ignored), 0, len(ignored), 0, 0),
                'Actual suite count/failure/ignored mismatch')
        require(names == inventory and set(required) <= set(passed),
                'Named parent controls missing or duplicate')
        finished = True
    require(finished and pending is None, 'Incomplete parent suite')
    return passed

def module_contract(actual, published, plan):
    def catalog(rows):
        require(len({r['name'] for r in rows}) == len(rows), 'Duplicate tool names')
        return {r['name']: r for r in rows}
    after, before = catalog(actual), catalog(published)
    require(set(after) - set(before) == set(plan['registered_tools'])
            and not set(before) - set(after), 'Unexpected tool addition/removal')
    for name, row in before.items():
        expected = dict(row)
        if name in plan['description_changes']:
            change = plan['description_changes'][name]
            require(row['description'] == change['before'], 'Legacy description predecessor')
            expected['description'] = change['after']
        require(after[name] == expected, 'Existing tool contract changed: ' + name)
    for name, fields in plan['registered_tools'].items():
        tool = after[name]
        parameters = tool['parameters']
        require(tool['type'] == 'function' and tool['strict'] is False
                and parameters['additionalProperties'] is False
                and set(parameters['properties']) == set(fields)
                and set(parameters['required']) == set(fields), 'Registered tool authority/schema')

def load(manifest, originals=False):
    packed = read(HERE / 'records.json.gz', 2 * 2**20)
    archive = manifest['archive']
    require(len(packed) == archive['bytes'] and sha(packed) == archive['sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as stream:
        raw = stream.read(8 * 2**20 + 1)
    require(len(raw) == archive['uncompressed_bytes'] and len(raw) <= 8 * 2**20, 'Expansion bound')
    encoded, rows = json.loads(raw), manifest['records']
    require(len(rows) == len({r['name'] for r in rows}) and set(encoded) == {r['name'] for r in rows},
            'Exact record membership')
    result = {}
    for row in rows:
        data = base64.b64decode(encoded[row['name']], validate=True)
        require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record: ' + row['name'])
        if originals:
            require(read(Path(row['original']), row['bytes']) == data, 'Original: ' + row['name'])
        result[row['name']] = data
    return result


def verify(manifest, records):
    require(manifest['format'] == 'registered-module-tools-evidence-v1', 'Format')
    get = lambda name: json.loads(records[name])
    plan, grant, result, owner, close, terminal = [get(name) for name in (
        'cpu-packet-002/plan.json', 'cpu-packet-002/root-grant.json', 'cpu-002/result.json',
        'cpu-002/owner.json', 'cpu-002/closeout.json', 'cpu-002/terminal-absence.json')]
    require(plan['source'] == grant['source'] == result['source'] == close['source'] == manifest['source'],
            'Source identity')
    require(sha(records['cpu-packet-002/plan.json']) == grant['plan_sha256'] and
            sha(records['cpu-packet-002/run.py']) == grant['runner_sha256'], 'Exact granted packet')
    require(result['passed'] and owner['passed'] and close['passed'] and not result['source_changed']
            and not result['target_retirements'], 'Actual result/source/cache outcome')
    require(len(owner['calls']) == len(result['steps']) == 38, 'Exact managed calls')
    for step, call in zip(result['steps'], owner['calls'], strict=True):
        require(step['argv'] == call['argv'] and step['exit'] == call['exit'] == 0 and
                call['finished'] and call['cleanup_confirmed'], 'Actual child outcome')
        for channel in ('stdout', 'stderr'):
            stream = call[channel]
            require(sha(records['cpu-002/' + stream['path']]) == stream['sha256'], 'Raw stream identity')
    require(close['absence'] and all(a['pid_absent'] and a['group_absent'] for a in close['absence'])
            and result['protected_rechecked'] and close['protected_rechecked'], 'Owner cleanup')
    require(terminal['actual_exec']['session'] == 94474 and terminal['actual_exec']['exit'] == 0 and
            terminal['pid_count'] == len(terminal['absence']['pids']) == 199 and
            terminal['group_count'] == len(terminal['absence']['groups']) == 39 and
            all(row['absent'] for rows in terminal['absence'].values() for row in rows.values()),
            'Actual terminal/fresh absence')
    require(terminal['result_sha256'] == sha(records['cpu-002/result.json']) and
            terminal['closeout_sha256'] == sha(records['cpu-002/closeout.json']) and
            terminal['protected_release_sha256'] == sha(records['cpu-002/protected-release.json']),
            'Terminal receipt identities')
    protected = get('cpu-002/protected-release.json')
    before = get('parent-cpu005/protected-release.json')
    require(len(before) == 108 and len(protected) == terminal['protected_rehashed'] == 121 and
            protected[:108] == before, 'All protected history retained')
    copies = [{key: row[key] for key in ('path', 'bytes', 'sha256')} for row in result['copies']]
    require(copies == manifest['binaries'] == protected[108:] and len(copies) == 13,
            'Thirteen independently copied artifacts')
    cargo = []
    for call in owner['calls']:
        for line in records['cpu-002/' + call['stdout']['path']].decode().splitlines():
            if line.startswith('{'):
                row = json.loads(line)
                if row.get('reason') == 'compiler-artifact':
                    cargo.append(row)
    for copied in result['copies']:
        require(copied['cargo'] in cargo and copied['cargo']['fresh'] is False,
                'Fresh actual copied Cargo owner')
    total, ignored_total = 0, 0
    for suite in manifest['suites']:
        call = owner['calls'][suite['call']]
        inventory = re.findall(r'^(\S+): test$', records[suite['inventory']].decode(), re.M)
        ignored = plan['native_steps'][2]['ignored']['incant_module_build'] if suite['ignored'] else {}
        names = test_result(records[suite['stdout']].decode(), inventory, suite['tests'], ignored)
        step = result['steps'][suite['call']]
        require(names == suite['tests'] == step['tests'] and suite['ignored'] == step['ignored_tests']
                == sorted(ignored) and call['argv'][0] in {c['path'] for c in copies}, 'Exact suite attribution')
        total += len(names)
        ignored_total += len(ignored)
    require(total == manifest['tests'] == 129 and ignored_total == manifest['ignored_tests'] == 4,
            'Parent passes/ignored totals')
    nested = result['nested_crash_contract']
    require(nested == manifest['nested_crash_contract'] and nested['actual_children_required_by_parent'] == 21
            and nested['expected_exit'] == 73 and len(nested['operations']) * len(nested['phases']) == 21
            and nested['parent'] in result['steps'][5]['tests'], 'Separately asserted nested crash outcomes')
    parent = get('parent-cpu005/closeout.json')
    require(plan['handoff']['postcleanup_seconds'] == parent['postcleanup_seconds'] and
            plan['handoff']['including_deleted_history_seconds'] == parent['including_deleted_history_seconds'],
            'Preserved cumulative handoff')
    for field in ('postcleanup_seconds', 'including_deleted_history_seconds'):
        require(math.isclose(result[field], plan['handoff'][field] + result['seconds'], abs_tol=1e-6)
                and close[field] == result[field], 'Charged cumulative time')
    require(result['seconds'] <= plan['bounds']['aggregate_seconds'] == 900 and
            plan['handoff']['retained_peak_bytes'] <= result['retained_peak_bytes'] <=
            plan['bounds']['target_cap_bytes'] == 18 * 2**30 and
            close['current_bytes'] <= close['retained_peak_bytes'], 'Retained storage/time bounds')
    require(result['schema_delta']['changed'] == ['agent-tools.json'] and
            not result['schema_delta']['added'] and not result['schema_delta']['removed'] and
            len(result['generated']) == 83, 'Actual generated delta')
    module_contract(get('generated/schemas/agent-tools.json'), get('generation-before/agent-tools.json'), plan)
    generated = {Path(row['path']).name: row for row in result['generated']}
    for row in manifest['generated_inputs']:
        require(sha(records['generated/' + row['path']]) == row['sha256'] ==
                generated[Path(row['path']).name]['sha256'], 'Applied exact generated bytes')
    refused = get('cpu-packet-001/review-refusal.json')
    initial = get('cpu-packet-001/pure-preparation-001.json')
    require(not refused['native_executed'] and not initial['passed'] and initial['passed_controls'] == 24
            and initial['controls'] == 26 and initial['native_children'] == 0, 'Original setup failures preserved')
    review = get('reviews/independent-native.json')
    require(review['passed'] and review['source'] == manifest['source'] and
            review['managed_calls'] == 38 and review['parent_passes'] == 129 and review['fresh_absence'],
            'Independent native review')
    for row in manifest['code_inputs']:
        require(sha(records['source/' + row['path']]) == row['sha256'], 'Inert producer source')
    fast = get('integration-fast/root-closeout.json')
    require(fast['passed'] and fast['exit'] == 0 and fast['checks'] == 10 and
            fast['ui_tests'] == 1041 and fast['fresh_pid_group_absence'] and
            fast['source'] != manifest['source'], 'Separately produced integration checks')


def self_test(manifest, records):
    def refused(change):
        altered = copy.deepcopy(records)
        change(altered)
        try:
            verify(manifest, altered)
        except (ValueError, KeyError):
            return
        raise AssertionError('Corrupted claim was accepted')
    def edit(name, mutate):
        def change(rows):
            value = json.loads(rows[name]); mutate(value)
            rows[name] = json.dumps(value).encode()
        return change
    refused(edit('cpu-packet-002/root-grant.json', lambda x: x.update(runner_sha256='0' * 64)))
    refused(edit('cpu-002/result.json', lambda x: x.update(passed=False)))
    refused(edit('cpu-002/owner.json', lambda x: x['calls'][0].update(exit=1)))
    refused(lambda x: x.update({'cpu-002/005.stdout.log': b'running 0 tests\n'}))
    refused(edit('cpu-002/result.json', lambda x: x['copies'][0]['cargo'].update(fresh=True)))
    refused(edit('cpu-002/terminal-absence.json', lambda x: x['absence']['pids']['44574'].update(absent=False)))
    refused(edit('cpu-002/result.json', lambda x: x.update(postcleanup_seconds=0)))
    refused(edit('generated/schemas/agent-tools.json', lambda x: x.pop()))
    # Exercise parser semantics directly, independently of outer log digests.
    for text in [
        'running 1 test\ntest diagnostic ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0s',
        'running 1 test\ntest wrong ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0s',
    ]:
        try:
            test_result(text, ['diagnostic'], [], {'diagnostic': 'manual'})
        except ValueError:
            continue
        raise AssertionError('Ignored/name corruption accepted')
    with tempfile.TemporaryDirectory() as folder:
        oversized = Path(folder) / 'oversized'
        oversized.write_bytes(b'12345')
        try:
            read(oversized, 4)
        except ValueError:
            pass
        else:
            raise AssertionError('Bounded input accepted oversized bytes')
    return 11


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true', help='Hash external copied bytes; never execute them')
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    manifest = json.loads(read(HERE / 'manifest.json', 256 * 1024))
    records = load(manifest, args.originals)
    verify(manifest, records)
    if args.originals:
        external = manifest['external_source_manifest']
        require(external['bytes'] <= 16 * 2**20, 'Complete source manifest bound')
        data = read(Path(external['path']), external['bytes'])
        require(sha(data) == external['sha256'], 'External complete source manifest')
        source = json.loads(data)
        require(source['source'] == manifest['source'] and all(
            source['files'][row['path']] == row['sha256'] for row in manifest['code_inputs']),
            'Listed source versus original complete manifest')
    if args.binaries:
        for row in manifest['binaries']:
            require(row['bytes'] <= 128 * 2**20, 'External copied artifact bound')
            data = read(Path(row['path']), row['bytes'])
            require(len(data) == row['bytes'] and sha(data) == row['sha256'], 'External copied artifact')
    if args.source_root:
        root = args.source_root.resolve()
        for row in manifest['code_inputs'] + manifest['generated_inputs']:
            relative = Path(row['path'])
            require(not relative.is_absolute() and '..' not in relative.parts, 'Source path')
            path = (root / relative).resolve()
            require(path.is_relative_to(root) and sha(read(path, 16 * 2**20)) == row['sha256'],
                    'Listed producer/generated source: ' + row['path'])
    controls = self_test(manifest, records) if args.self_test else 0
    print(f'PASS: {len(records)} inert records;129 parent passes;4 ignored diagnostics;'
          f'21 nested crash exits asserted by parent;38 managed calls;13 external artifact identities;'
          f'{controls} controls. No worker/install/device/performance gate.')


if __name__ == '__main__':
    main()
