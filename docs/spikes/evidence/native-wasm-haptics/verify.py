"""Bounded inert-record replay. No archived code, Wasm or native process executes."""
import gzip
import hashlib
import io
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SOURCES = ['0d7c2e5af34f2eb40eb27305ff20f27042e559e7'] + ['c218cce4c483ca85d6d7b4e083f56b3a6496a9bf'] * 3
COUNTS = {
    'SDK haptic wire and ignored error controls': [4],
    'shared capability and adjacent transaction controls': [8, 8, 6],
    'actual static and development haptic parity': [2, 6],
    'legacy static numeric guest': [1],
    'existing TypeScript haptic regression controls': [10],
    'ordinary static player owner': [1],
    'existing composed TypeScript owner': [1],
}
GUESTS = ['rust-haptics', 'rust-haptics-scratch', 'rust-haptics-amplitude',
          'rust-haptics-quota', 'rust-haptics-owner', 'rust-haptics-callback',
          'rust-haptics-trap', 'rust-haptics-saved-target', 'bulk', 'haptics-start']


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def replay(packet):
    records = {}
    for item in packet['records']:
        name, raw = item['name'], item['text'].encode('utf-8')
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'Relative inert record')
        require(name not in records and len(raw) == item['bytes'] and sha(raw) == item['sha256'], 'Unique intact record')
        records[name] = raw
    read = lambda name: json.loads(records[name])
    cumulative, peak, successes, lints = 1004.3550879609538, 0, {}, 0
    for index, source in enumerate(SOURCES, 1):
        prefix = f'cpu{index:03d}/'
        plan, result, close, owner, frozen = (read(prefix + name) for name in
            ['plan.json', 'result.json', 'terminal-closeout.json', 'output/owner.json', 'packet.json'])
        require(plan['source_head'] == result['source_head'] == close['source_head'] == source, 'Exact native producer')
        require(result['passed'] is (index == 4) and close['exec_terminal_exit'] == (0 if index == 4 else 1), 'Retained actual outcome')
        require(result['source_tools_predecessors_protected_unchanged'] is True, 'Native input freeze held')
        require(result['packet_sha256'] == close['packet_sha256'] == sha(records[prefix + 'packet.json']), 'Frozen packet binding')
        require(close['result_sha256'] == sha(records[prefix + 'result.json']) and result['owner_sha256'] == close['owner_sha256'] == sha(records[prefix + 'output/owner.json']), 'Terminal and owner binding')
        for name, identity in frozen['files'].items():
            if prefix + name in records:
                raw = records[prefix + name]
                require(len(raw) == identity['bytes'] and sha(raw) == identity['sha256'], 'Archived frozen input binding')
        for name, identity in result['retained'].items():
            if prefix + 'output/' + name in records:
                raw = records[prefix + 'output/' + name]
                require(len(raw) == identity['bytes'] and sha(raw) == identity['sha256'], 'Retained output binding')
        require(result['wrapper_pid'] == owner['wrapper_pid'] == close['wrapper_pid'], 'Wrapper identity')
        managed = {item['pid'] for item in owner['owned']} | {item['pid'] for item in owner['calls']}
        observed = result['observed_pids']
        require(managed and observed and all(type(n) is int and n > 0 for n in observed)
                and len(observed) == len(set(observed)) and managed <= set(observed), 'Observed managed process inventory')
        require(set(result['groups']) == {item['pid'] for item in owner['owned']}, 'Exact owned groups')
        for rows, expected in [(result['absence'], set(observed)),
                               (close['absence'], set(observed) | {result['wrapper_pid']})]:
            require(rows and len(rows) == len(expected) and {r['pid'] for r in rows} == expected
                    and all(r['pid_and_group_absent'] is True for r in rows), 'Exact nonempty absence inventory')
        require(all(item['cleanup_confirmed'] is True for item in owner['owned']), 'Managed groups reaped')
        for call in owner['calls']:
            require(call['finished'] is True and call['cleanup_confirmed'] is True, 'Managed call terminal')
            for stream in ['stdout', 'stderr']:
                raw = records[prefix + 'output/' + call[stream]['path']]
                require(sha(raw) == call[stream]['sha256'], 'Exact managed stream')
        require(result['prior_seconds'] == plan['prior_seconds'] == cumulative, 'All earlier charges retained')
        cumulative += result['seconds']
        require(result['cumulative_seconds'] == cumulative and cumulative < 6000, 'Original time account')
        require(plan['target_cap'] == 12033904640 and plan['aggregate_seconds'] == 6000
                and plan['environment']['CARGO_BUILD_JOBS'] == plan['environment']['RAYON_NUM_THREADS'] == '2', 'Original bounds')
        peak = max(peak, result['target_peak'])
        require(peak <= plan['target_cap'] and result['final_packet'] <= 1024**3, 'Storage bounds')
        require(result['gpu'] is False and result['window'] is False and result['native_shipping_export'] is False, 'CPU-only scope')
        for outcome in result['completed_stages']:
            name = outcome['name']
            if 'call' not in outcome:
                require(index == 2 and name == 'actual SDK and legacy guests' and outcome['guests'] == 10, 'Prepared guest count')
                continue
            call = owner['calls'][outcome['call']]
            stage = next(s for s in plan['stages'] if s['name'] == name)
            require(call['argv'] == stage['argv'] and call['exit'] == 0, 'Exact successful command')
            text = records[prefix + 'output/' + call['stdout']['path']].decode()
            if name in COUNTS:
                found = re.findall(r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text, re.M)
                require(found == [('ok', str(n), '0', '0') for n in COUNTS[name]] and outcome['tests'] == sum(COUNTS[name]), 'Actual accepted test counts')
                require(index == 2 and name not in successes, 'Exact behavior producer')
                successes[name] = outcome['tests']
            if 'strict' in name:
                require(call['argv'][1] == 'clippy' and call['argv'][-2:] == ['-D', 'warnings'], 'Strict lint selection')
                rows = [json.loads(line) for line in text.splitlines() if line.startswith('{')]
                require([r['success'] for r in rows if r.get('reason') == 'build-finished'] == [True], 'Actual lint completion')
                lints += 1
        if index == 1:
            require(result['error'] == "ValueError('Invalid/duplicate static module name')", 'Original preparation failure')
        if index in [2, 3]:
            require(owner['calls'][-1]['exit'] == 101, 'Retained failed lint')
    require(set(successes) == set(COUNTS) and sum(successes.values()) == 47 and lints == 3, 'Complete scoped qualification')
    require('INCANT_FLIPBOOK_GUEST' in records['cpu002/output/028.stderr.log'].decode(), 'Broad lint refusal retained')
    require('error[E0061]' in records['cpu003/output/000.stderr.log'].decode(), 'Feature-selection failure retained')
    corrected = read('cpu004/plan.json')['stages']
    original = read('cpu003/plan.json')['stages']
    args = original[0]['argv'];args[args.index('--features') + 1] += ',incant_game_runtime/ui,incant_game_runtime/native-checkpoints'
    require(corrected == original, 'Only owning feature closure correction')
    require(corrected[1]['argv'][corrected[1]['argv'].index('--lib'):] == ['--lib', '--test', 'haptics', '--test', 'haptics_parity', '--message-format=json-render-diagnostics', '--', '-D', 'warnings'], 'Focused development test lint')
    prepared = read('cpu002/output/prepared-manifest.json')
    require([m['name'] for m in prepared['modules']] == GUESTS, 'Exact ten guest inventory')
    for index, module in enumerate(prepared['modules']):
        require(module['persistent_state'] == ({'bytes': 8, 'schema_sha256': '11' * 32} if index < 8 else None), 'Actual declared guest state')
        require(prepared['files'][module['prefix'] + '.original.wasm'] == module['input_sha256'] and prepared['files'][module['prefix'] + '.wasm'] == module['prepared_sha256'], 'Original/prepared guest identities')
    compat = read('cpu002/source-compatibility.json')
    require(compat['native_producer_head'] == SOURCES[0] and compat['continuation_head'] == SOURCES[1]
            and compat['carried_cooker_and_guest_compiler_inputs_unchanged'] is True
            and compat['all_other_tracked_file_identities_equal'] is True, 'Exact reused producer provenance')
    require(compat['changed'] == ['crates/incant_wasm_dev/tests/haptics_parity.rs',
            'tools/probes/shipping-wasm2c.py', 'tools/tests/test_wasm2c_shipping.py'], 'Only named fixture correction')
    return {'passed': True, 'records': len(records), 'actual_tests': 47, 'strict_lints': lints,
            'aggregate_seconds': cumulative, 'haptics_seconds': cumulative - 1004.3550879609538,
            'target_peak_allocated_bytes': peak, 'native_executed': False}


def load():
    with (HERE / 'manifest.json').open('rb') as file:
        manifest_raw = file.read(65537)
    require(len(manifest_raw) <= 65536, 'Bounded manifest')
    manifest = json.loads(manifest_raw)
    with (HERE / 'records.json.gz').open('rb') as file:
        packed = file.read(2 * 1024**2 + 1)
    require(len(packed) <= 2 * 1024**2 and len(packed) == manifest['compressed_bytes'] and sha(packed) == manifest['sha256'], 'Bounded archive')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as file:
        raw = file.read(12 * 1024**2 + 1)
    require(len(raw) <= 12 * 1024**2 and len(raw) == manifest['uncompressed_bytes'], 'Bounded raw records')
    packet = json.loads(raw)
    require(packet['format'] == manifest['format'], 'Evidence format')
    result = replay(packet)
    require(result == manifest['replay'], 'Manifest agrees with raw records')
    read = lambda name: json.loads(next(r['text'] for r in packet['records'] if r['name'] == name))
    require(len(manifest['attempts']) == len(SOURCES), 'Summary attempt inventory')
    for index, (attempt, source) in enumerate(zip(manifest['attempts'], SOURCES), 1):
        prefix = f'cpu{index:03d}/'
        result, frozen = read(prefix + 'result.json'), read(prefix + 'packet.json')
        require(attempt['native_source'] == source == result['source_head']
                and attempt['source_inventory'] == frozen['files']['source.json']
                and attempt['packet_sha256'] == result['packet_sha256']
                and attempt['passed'] is result['passed'] and attempt['seconds'] == result['seconds'], 'Summary producing identities')
    return packet


if __name__ == '__main__':
    print(json.dumps(replay(load())))
