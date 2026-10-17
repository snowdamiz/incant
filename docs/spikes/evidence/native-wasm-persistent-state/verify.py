"""Bounded data-only replay. Never executes archived scripts, Wasm or binaries."""
import gzip
import hashlib
import io
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
EXPECTED = {
    'zero-feature script strict compile': None,
    'default-feature script strict compile': None,
    'ABI and SDK boundary controls': [1, 1],
    'runtime transaction and restart controls': [8, 6],
    'existing bytecode ordinary native persisted controls': [34],
    'cooker capture and policy controls': [4],
    'current normal cooker': None,
    'seven actual Rust SDK guests and native translations': None,
    'actual wasm2c and Wasmtime checkpoint controls': [6, 3, 1, 3],
    'both-host tagged codec and bytecode goldens': [2],
    'strict ABI and SDK lint': None,
    'strict native runtime lint': None,
    'strict cooker lint': None,
    'strict hosted bytecode checkpoint lint': None,
    'strict actual-host conformance lint': None,
}
SOURCES = [
    '393951d1ba7e61835e7df82fe7ed2ecaeb1ccc4f',
    '92b52974424ef4e815cc74bddd6baa56102b6f8b',
    '9cfedb1d9991f42affca5d3238147dde00393039',
    '96eac7762bc4c8400c76f5f8b389d0beac7fd192',
    'ea6669143f9ee5790d8628b06f0ec197910c0f81',
]


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(packet):
    records = {}
    for item in packet['records']:
        name = item['name']
        require(not name.startswith('/') and '..' not in Path(name).parts, 'Relative data record')
        raw = item['text'].encode('utf-8')
        require(name not in records and len(raw) == item['bytes'] and sha(raw) == item['sha256'], 'Unique intact record')
        records[name] = raw
    read = lambda name: json.loads(records[name])
    successes = {}
    cumulative = 0.
    peak = 0
    for index, source in enumerate(SOURCES, 2):
        prefix = f'cpu{index:03d}/'
        plan, result, close = (read(prefix + f'{name}.json') for name in ('plan', 'result', 'terminal-closeout'))
        owner = read(prefix + 'output/owner.json')
        manifest = read(prefix + 'packet.json')
        require(plan['source_head'] == result['source_head'] == source, 'Exact producing source')
        require(result['source_tools_predecessors_unchanged'] is True and close['source_tools_verified'] is True, 'Input freeze held')
        require(result['packet_sha256'] == sha(records[prefix + 'packet.json']), 'Native packet binding')
        for name, value in manifest['files'].items():
            if prefix + name in records:
                require(sha(records[prefix + name]) == value['sha256'] and len(records[prefix + name]) == value['bytes'], 'Frozen input binding')
        require(result['owner_sha256'] == sha(records[prefix + 'output/owner.json']), 'Process owner binding')
        require(close['result_sha256'] == sha(records[prefix + 'result.json']), 'Actual terminal binding')
        require(close['actual_wrapper_exit'] == (0 if index == 6 else 1), 'Preserved actual wrapper outcome')
        require(result['passed'] is (index == 6), 'Preserved failed attempts')
        require(result['fresh_allocation'] is (index == 2) and result['historical_font_reused'] is False, 'Fresh target and exact continuation')
        require(close['all_pids_groups_absent'] is True, 'Terminal process absence')
        pids = [item['pid'] for item in result['absence']]
        require(bool(pids) and all(type(pid) is int and pid > 0 for pid in pids) and len(pids) == len(set(pids)), 'Distinct observed process identities')
        inventory = read(prefix + 'process-inventory-certificate.json')
        require(inventory['original_result_sha256'] == sha(records[prefix + 'result.json']) and inventory['original_owner_sha256'] == sha(records[prefix + 'output/owner.json']) and inventory['original_terminal_closeout_sha256'] == sha(records[prefix + 'terminal-closeout.json']), 'Process inventory provenance')
        managed = sorted({item['pid'] for item in owner['calls']} | {item['pid'] for item in owner['owned']})
        require(managed and managed == inventory['managed_call_and_group_pids'] and set(managed) <= set(pids), 'Exact managed identity inventory')
        require(result['wrapper_pid'] == owner['wrapper_pid'] == inventory['wrapper_pid'] and type(result['wrapper_pid']) is int and result['wrapper_pid'] > 0, 'Wrapper identity')
        exact = sorted(set(pids) | {result['wrapper_pid']})
        require(sorted(pids) == inventory['sampled_descendant_and_group_pids'] and exact == inventory['all_checked_pids_and_groups'] and len(exact) == close['pid_count'], 'Exact observed identity inventory')
        require(inventory['fresh_all_absent'] is True, 'Fresh full inventory absence')
        require(all(item['pid_and_group_absent'] is True for item in result['absence']), 'Observed process cleanup')
        require(all(group['cleanup_confirmed'] is True for group in owner['owned']), 'Managed group cleanup')
        require(plan['jobs'] == plan['rayon_threads'] == 2 and plan['target_ceiling_allocated_bytes'] == 12 * 1024**3, 'Unchanged native limits')
        require(result.get('prior_charged_seconds', 0) == cumulative, 'Cumulative failures retained')
        cumulative += result['seconds']
        require(abs(result.get('cumulative_seconds', cumulative) - cumulative) < 1e-8 and cumulative < 5400, 'Original time account')
        peak = max(peak, result['sampled_target_peak_allocated_bytes'])
        require(peak <= 12 * 1024**3 and result['final_packet_allocated_bytes'] <= 1024**3, 'Storage account')
        for name, value in result['retained_files'].items():
            if prefix + 'output/' + name in records:
                require(sha(records[prefix + 'output/' + name]) == value['sha256'], 'Retained native output binding')
        outcomes = read(prefix + 'output/outcomes.json') if prefix + 'output/outcomes.json' in records else []
        for outcome in outcomes:
            name = outcome['name']
            require(name in EXPECTED, 'Original matrix stage')
            if 'call' in outcome:
                call = owner['calls'][outcome['call']]
                require(call['exit'] == 0 and call['cleanup_confirmed'] is True, 'Successful managed native call')
                stage = next(stage for stage in plan['stages'] if stage['name'] == name)
                require(call['argv'] == stage['argv'], 'Exact matrix command')
                stdout = records[prefix + 'output/' + call['stdout']['path']]
                require(sha(stdout) == call['stdout']['sha256'], 'Exact native stdout')
                if EXPECTED[name] is not None:
                    groups = re.findall(r'test result: ok\. (\d+) passed; 0 failed; 0 ignored;', stdout.decode())
                    require(list(map(int, groups)) == EXPECTED[name] and outcome['tests'] == sum(EXPECTED[name]), 'Executed test groups')
                if 'strict' in name:
                    require(call['argv'][-2:] == ['-D', 'warnings'], 'Strict lint flags')
            else:
                require(name == 'seven actual Rust SDK guests and native translations' and outcome['guests'] == 7, 'Actual guest inventory')
            successes[name] = source
        if index != 6:
            require(owner['calls'][-1]['exit'] == 101, 'Retained native failure')
    require(set(successes) == set(EXPECTED), 'Complete original matrix')
    require(sum(sum(n) for n in EXPECTED.values() if n) == 69, '69 matrix test executions')
    prepared = read('cpu005/output/prepared-manifest.json')
    require([m['name'] for m in prepared['modules']] == ['rust-persistent', 'rust-persistent-owner', 'rust-persistent-callback', 'rust-persistent-quota', 'rust-persistent-restart', 'rust-persistent-start', 'rust-session-once'], 'Qualified SDK guests')
    require(all(m['persistent_state'] == {'bytes': 16, 'schema_sha256': '05' * 32} for m in prepared['modules'][:-1]) and prepared['modules'][-1]['persistent_state'] is None, 'Opt-in and legacy routing')
    for module in prepared['modules']:
        require(prepared['files'][module['prefix'] + '.original.wasm'] == module['input_sha256'] and prepared['files'][module['prefix'] + '.wasm'] == module['prepared_sha256'], 'Original and prepared module identity')
    require('UI numeric transform or velocity' in records['cpu004/output/035.stdout.log'].decode(), 'Original mixed-scene failure retained')
    require('5 passed; 1 failed;' in records['cpu004/output/035.stdout.log'].decode(), 'Partial failed parity boundary')
    return {'passed': True, 'records': len(records), 'test_executions': 69, 'strict_clippy_stages': 7, 'cumulative_seconds': cumulative, 'target_peak_bytes': peak, 'native_executed': False}


def load():
    with (HERE / 'manifest.json').open('rb') as file:
        manifest_bytes = file.read(65537)
    require(len(manifest_bytes) <= 65536, 'Bounded manifest')
    manifest = json.loads(manifest_bytes)
    with (HERE / 'records.json.gz').open('rb') as file:
        packed = file.read(1024 * 1024 + 1)
    require(len(packed) <= 1024 * 1024 and len(packed) == manifest['compressed_bytes'] and sha(packed) == manifest['sha256'], 'Bounded archive')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as file:
        raw = file.read(4 * 1024 * 1024 + 1)
    require(len(raw) <= 4 * 1024 * 1024 and len(raw) == manifest['uncompressed_bytes'], 'Bounded raw records')
    packet = json.loads(raw)
    require(packet['format'] == manifest['format'], 'Evidence format')
    result = replay(packet)
    require(result['records'] == manifest['record_count'] and result['cumulative_seconds'] == manifest['aggregate_seconds'] and result['target_peak_bytes'] == manifest['target_allocated_final_and_peak_bytes'], 'Summary agrees with raw evidence')
    require(manifest['final_native_source'] == SOURCES[-1] and manifest['matrix_test_executions'] == 69 and manifest['strict_clippy_stages'] == 7 and manifest['extra_passing_tests_in_failed_attempt'] == 5, 'Summary scope')
    require(manifest['original_seconds_cap'] == 5400 and manifest['target_cap_bytes'] == 12 * 1024**3 and manifest['packet_cap_bytes'] == 1024**3, 'Summary original caps')
    records = {item['name']: item['text'] for item in packet['records']}
    require(len(manifest['attempts']) == len(SOURCES), 'Summary attempt count')
    for attempt, source in zip(manifest['attempts'], SOURCES):
        result = json.loads(records[attempt['name'] + '/result.json'])
        inputs = json.loads(records[attempt['name'] + '/packet.json'])
        require(attempt['source'] == source == result['source_head'] and attempt['source_inventory'] == inputs['files']['source.json'] and attempt['passed'] is result['passed'] and attempt['seconds'] == result['seconds'], 'Summary source and attempt binding')
    return packet


if __name__ == '__main__':
    print(json.dumps(replay(load())))
