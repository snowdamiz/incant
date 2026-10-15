#!/usr/bin/env python3
"""Read-only replay of bounded source/CPU evidence; never launches the engine."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess

HERE = Path(__file__).resolve().parent
MAX_RECORD = 4 * 1024**2
MAX_ARCHIVE = 8 * 1024**2
MAX_INFLATED = 32 * 1024**2


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(path, limit):
    require(not path.is_symlink(), 'Linked record: ' + str(path))
    info = path.stat()
    require(stat.S_ISREG(info.st_mode) and info.st_size <= limit,
            'Invalid or oversized record: ' + str(path))
    with path.open('rb') as stream:
        result = stream.read(limit + 1)
    require(len(result) <= limit, 'Read limit: ' + str(path))
    return result


def decode(data):
    def object_pairs(pairs):
        value = {}
        for key, item in pairs:
            require(key not in value, 'Duplicate JSON key: ' + key)
            value[key] = item
        return value
    return json.loads(data, object_pairs_hook=object_pairs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--binaries', action='store_true')
    parser.add_argument('--git-source', action='store_true')
    args = parser.parse_args()
    manifest = decode(read(HERE / 'manifest.json', 256 * 1024))
    require(manifest['format'] == 'incant-project-template-evidence'
            and type(manifest['version']) is int and manifest['version'] == 1, 'Manifest contract')
    archive = manifest['archive']
    require(archive['path'] == 'records.json.gz', 'Archive path')
    compressed = read(HERE / archive['path'], MAX_ARCHIVE)
    require(len(compressed) == archive['compressed_bytes']
            and digest(compressed) == archive['sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(compressed)) as stream:
        raw = stream.read(MAX_INFLATED + 1)
    require(len(raw) <= MAX_INFLATED and len(raw) == archive['uncompressed_bytes'], 'Inflated bound')
    document = decode(raw)
    require(document['format'] == 'incant-project-template-records'
            and type(document['version']) is int and document['version'] == 1, 'Record contract')
    rows = document['records']
    require(isinstance(rows, list) and len(rows) == archive['records'] and len(rows) <= 256, 'Record count')
    records = {}
    for row in rows:
        name = row['path']
        path = PurePosixPath(name)
        require(isinstance(name, str) and not path.is_absolute() and '..' not in path.parts
                and str(path) == name and name not in records, 'Record path')
        require(type(row['bytes']) is int and 0 <= row['bytes'] <= MAX_RECORD, 'Record size')
        data = base64.b64decode(row['base64'], validate=True)
        require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Record hash: ' + name)
        records[name] = data
        if args.originals:
            require(read(Path(manifest['original_root']) / name, MAX_RECORD) == data, 'Original differs: ' + name)
    for name, expected in manifest['bindings'].items():
        require(digest(records[name]) == expected, 'Bound identity: ' + name)

    def value(name):
        return decode(records[name])

    freezes = {}
    for number, source in [('001', manifest['production_source']), ('002', manifest['qualified_test_source'])]:
        prefix = 'source-' + number
        freeze = value(prefix + '/freeze.json')
        require(freeze['source'] == source and len(freeze['rows']) == 28, 'Source generation')
        files = {}
        for row in freeze['rows']:
            data = records[prefix + '/inputs/' + row['path']]
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'Frozen input: ' + row['path'])
            files[row['path']] = data
            if args.git_source:
                actual = subprocess.check_output(['git', 'show', source + ':' + row['path']],
                                                 cwd=HERE.parents[3], timeout=15)
                require(actual == data, 'Git source differs: ' + row['path'])
        freezes[number] = files
    changed = [name for name in freezes['001'] if freezes['001'][name] != freezes['002'][name]]
    require(changed == ['crates/incant_headless/tests/templates.rs'], 'Expected only test successor')
    require('cpu-001/result.json' not in records, 'Proposal001 must remain unexecuted')
    admission = value('admission-correction-002.json')
    require(len(admission['tests']) == 19 and admission['native_build_test_player_calls'] == 0
            and admission['capture_calls'] == 0, 'Pure admission control scope')
    outcomes = {}
    artifact_rows = []
    for number in ('002', '003'):
        result = value('cpu-' + number + '/result.json')
        plan_path = 'cpu-plan-' + number + '.json'
        plan = value(plan_path)
        runner_path = 'qualify-' + number + '.py'
        grant = value('root-cpu-grant-' + number + '.json')
        require(result['plan_sha256'] == digest(records[plan_path])
                and result['runner_sha256'] == digest(records[runner_path])
                and result['source'] == plan['source'] == grant['source'], 'Executed source/plan/runner')
        require(grant['accepted_for_cpu'] and grant['plan_sha256'] == result['plan_sha256']
                and grant['runner_sha256'] == result['runner_sha256'], 'Granted identity')
        require(result['native_process_sha256'] == digest(records['native_process.py']), 'Supervisor helper')
        require(not result['managed_call_active'], 'Managed child still active')
        require(all(result[name] is False for name in ['gpu_execution', 'window_execution',
                'listener_execution', 'native_player_execution']), 'Unexpected native execution scope')
        require(result['minimum_sampled_free_bytes'] >= plan['minimum_free_bytes']
                and result['target_last_sampled_bytes'] - result['target_initial_bytes']
                <= plan['maximum_target_growth_bytes'], 'Recorded storage limits')
        require(result['elapsed_seconds'] <= plan['overall_seconds'], 'Recorded deadline')
        commands = result['commands']
        require(commands[0]['name'] == 'rust-version' and commands[0]['exit_code'] == 0, 'Compiler identity probe')
        expected_steps = plan['steps'] if number == '003' else plan['steps'][:2]
        require(len(commands) == len(expected_steps) + 1, 'Stopped/executed stage count')
        for step, command in zip(expected_steps, commands[1:]):
            require(command['name'] == step['name'] and command['argv'] == step['argv'], 'Exact planned argv')
        for command in commands:
            name = command['name']
            require(command['managed_returned_after_cleanup'] is True, 'Child cleanup: ' + name)
            for stream in ('stdout', 'stderr'):
                data = records['cpu-' + number + '/' + name + '.' + stream]
                require(digest(data) == command[stream + '_sha256'], 'Command log: ' + name)
            if 'test_summaries' in command:
                text = records['cpu-' + number + '/' + name + '.stdout'].decode()
                parsed = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored', text)
                expected = [dict(zip(('status', 'passed', 'failed', 'ignored'), row)) for row in parsed]
                require(expected == command['test_summaries'] and expected, 'Raw test result correspondence')
            for artifact in command.get('artifacts', []):
                artifact_rows.append((artifact['path'], artifact['sha256'], number, name, artifact['target']['name']))
        outcomes[number] = result
    first, final = outcomes['002'], outcomes['003']
    require(first['outcome'] == 'failure' and [r['exit_code'] for r in first['commands']] == [0, 0, 101], 'Retained first failure')
    require(first['commands'][1]['test_summaries'] == [{'status':'ok','passed':'6','failed':'0','ignored':'0'}], 'Authoring six')
    require(first['commands'][2]['test_summaries'] == [{'status':'FAILED','passed':'2','failed':'1','ignored':'0'}], 'Original headless scope')
    require(b'0.019087493419647217 != 0' in records['cpu-002/template-native-behavior.stdout'], 'Exact-axis failure evidence')
    require(final['outcome'] == 'success' and all(r['exit_code'] == 0 for r in final['commands']), 'Successor stages')
    require(final['commands'][1]['test_summaries'] == [{'status':'ok','passed':'1','failed':'0','ignored':'0'}], 'Focused spatial pass')
    require(first['elapsed_seconds'] + final['elapsed_seconds'] <= value('cpu-plan-002.json')['overall_seconds'], 'Cumulative deadline')
    require(first['target_initial_bytes'] == final['target_initial_bytes'], 'Cumulative target baseline')
    require(value('cpu-plan-003.json')['prior_attempt']['sha256'] == digest(records['cpu-002/result.json']), 'Retained failure link')
    expected = [(r['path'], r['sha256'], r['generation'].removeprefix('cpu-'), r['stage'], r['target'])
                for r in manifest['copied_binaries']]
    require(artifact_rows == expected, 'Copied artifact provenance')
    if args.binaries:
        for row in manifest['copied_binaries']:
            data = read(Path(row['path']), 512 * 1024**2)
            require(len(data) == row['bytes'] and digest(data) == row['sha256'], 'External binary differs')
    require(manifest['outcomes']['total_distinct_passing_tests'] == 9
            and manifest['outcomes']['templates_exercised'] == 5, 'Scoped outcome counts')
    require(all(manifest['outcomes'][name] is False for name in ['native_window', 'gpu',
                'capture', 'gui_template_picker', 'appearance_acceptance', 'performance_acceptance']),
            'Unexpected acceptance claim')
    print(json.dumps({'verified_records':len(records), 'source_generations':2,
        'retained_original_failure':True, 'distinct_passing_tests':9, 'templates':5,
        'external_originals_verified':args.originals, 'external_binary_copies_verified':5 if args.binaries else 0,
        'git_source_verified':args.git_source, 'native_execution':False,
        'gui_appearance_performance_acceptance':False}))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, AssertionError, subprocess.SubprocessError) as error:
        raise SystemExit('Evidence verification failed: ' + str(error)) from error
