"""Bounded data-only replay. Never executes archived code, modules or native artifacts."""
import argparse
from copy import deepcopy
import gzip
import hashlib
import io
import json
import math
from pathlib import Path
import tempfile

MIB = 1024 ** 2
HERE = Path(__file__).resolve().parent


def need(value, label):
    if not value:
        raise ValueError(label)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        need(key not in result, 'Duplicate JSON key')
        result[key] = value
    return result


def parsed(data):
    return json.loads(data, object_pairs_hook=unique)


def bounded(path, cap):
    need(type(cap) is int and 0 <= cap <= 8 * MIB, 'Finite nonnegative read cap')
    with path.open('rb') as stream:
        data = stream.read(cap + 1)
    need(len(data) <= cap, 'Input byte cap: ' + str(path))
    return data


def load(root=HERE):
    manifest = parsed(bounded(root / 'manifest.json', 65536))
    need(manifest['version'] == 1 and manifest['format'] == 'incant-untrusted-rust-build-evidence', 'Manifest scope')
    archive = manifest['archive']
    need(archive['path'] == 'records.json.gz', 'Fixed archive name')
    packed = bounded(root / 'records.json.gz', 2 * MIB)
    need(len(packed) == archive['bytes'] and sha(packed) == archive['sha256'], 'Packed identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed), mode='rb') as stream:
        raw = stream.read(8 * MIB + 1)
    need(len(raw) <= 8 * MIB and len(raw) == archive['expanded_bytes']
         and sha(raw) == archive['expanded_sha256'], 'Expanded identity/bound')
    payload = parsed(raw)
    need(payload['version'] == 1 and len(payload['records']) == manifest['record_count'], 'Record count')
    records = {}
    for row in payload['records']:
        name = row['name']
        need(isinstance(name, str) and name not in records, 'Unique record name')
        data = row['text'].encode('utf-8')
        need(len(data) <= MIB and len(data) == row['bytes'] and sha(data) == row['sha256'], 'Record identity: ' + name)
        records[name] = row
    return manifest, records


def analyze(manifest, records):
    def read(name): return parsed(records[name]['text'])
    claims = manifest['claims']
    need(claims == {'selected_macos_request_and_network_io_controls': True,
         'original_socket_creation_denial_passed': False, 'all_socket_creation_denied': False,
         'original_native005_status': 'failed', 'blanket_security': False, 'cross_platform': False,
         'normal_project_import_history': False, 'source_free_export_rerun': False,
         'browser': False, 'performance': False, 'windows_gpu_capture_audio': False}, 'Scope/overclaim')
    names = ['native-001', 'startup-002', 'native-003', 'native-004', 'native-005', 'native-006', 'native-007']
    expected = ['failed', 'complete', 'failed', 'failed', 'failed', 'failed', 'complete']
    need([row['name'] for row in manifest['runs']] == names, 'Run sequence')
    elapsed = []
    for name, status, row in zip(names, expected, manifest['runs']):
        close = read(name + '-closeout.json')
        result = read(name + '/result.json')
        need(close['status'] == result['status'] == row['status'] == status, 'Preserve run status: ' + name)
        need(close['source'] == result['source'] == row['source'], 'Producer identity: ' + name)
        need(close['actual_tool_exit'] == row['actual_tool_exit'] == (0 if status == 'complete' else 1), 'Actual terminal: ' + name)
        need(close['fresh_absence'] and all(p['pid_absent'] is True and p['group_absent'] is True
             for p in close['fresh_absence']), 'Fresh owned absence: ' + name)
        need(result['elapsed_seconds'] == row['elapsed_seconds'], 'Exact elapsed receipt')
        elapsed.append(row['elapsed_seconds'])
    need(all(math.isfinite(value) and value > 0 for value in elapsed), 'Finite positive charges')
    need(math.isclose(sum(elapsed), manifest['cumulative_native_seconds'], abs_tol=1e-8), 'All native time retained')
    need(math.isclose(180 - sum(elapsed), manifest['remaining_seconds'], abs_tol=1e-8)
         and manifest['original_native_account_seconds'] == 180, 'Original account')
    need(read('native-001/result.json')['error_type'] == 'Refused' and 'sandbox initialization failed' in read('native-001/result.json')['error'], 'Startup refusal retained')
    need('E0428' in records['native-003/valid-relative-sdk/work/05-untrusted-compile.stderr']['text'], 'Callback fixture failure retained')
    need(read('native-004-diagnosis.json')['actual_artifact']['bytes'] == 1633057
         and read('native-004-diagnosis.json')['actual_artifact']['header_hex'] == '0061736d01000000', 'Oversized Wasm fixture retained')
    original = read('native-005/result.json')
    need([(x['name'], x['status']) for x in original['cases']] == [
         ('valid-relative-sdk', 'complete'), ('cargo-refused', 'expected-refusal'),
         ('build-script-refused', 'expected-refusal'), ('private-include-refused', 'expected-refusal'),
         ('proc-macro-refused', 'expected-refusal'), ('external-macro-refused', 'expected-refusal'),
         ('normal-validator-refused', 'expected-refusal'), ('cancelled-compiler', 'expected-refusal')], 'Eight exact original request cases')
    old_control = read('native-005/02-trusted-control-run.stdout')
    need(old_control == {'read_denied': True, 'write_denied': True, 'exec_denied': True,
         'socket_denied': False, 'clean_environment': True}, 'Original creation criterion failed')
    need([x['returncode'] for x in original['calls']] == [0, 101], 'Original canary actual failure')
    setup = read('native-006/result.json')
    need(len(setup['calls']) == 1 and setup['calls'][0]['returncode'] == 0
         and setup['error_type'] == 'OSError' and 'Protocol not available' in setup['error'], 'Setup failure retained')
    need(read('native-006/control-network.json')['endpoints_closed'] is True, 'Failed setup closes endpoints')
    result = read('native-007/result.json')
    need(result['source'] == manifest['source'] == '6e265da1ed76664445f10c8b5fcf09a72076df12', 'Followup producer')
    need(result['original_full_episode_status'] == result['original_socket_creation_criterion'] == 'failed', 'No relabel')
    controls = read('native-007/01-trusted-control-run.stdout')
    need(controls == result['os_control'] and controls['criterion'] == 'network-io-v2'
         and controls['sockets_created'] == 4 and controls['socket_creation_denied'] is False, 'Distinct socket creation')
    need(all(controls[key] is True for key in ['read_denied', 'write_denied', 'exec_denied', 'clean_environment']), 'Kept controls')
    need(set(controls['network']) == {'tcp_connect', 'tcp_bind', 'udp_bind', 'udp_send_to'}
         and all(row == {'return': -1, 'errno': 1} for row in controls['network'].values()), 'Actual four EPERM outcomes')
    network = read('native-007/control-network.json')
    need(network['status'] == 'complete' and network['endpoints_closed'] is True
         and network['sink_datagrams'] == network['sink_payload_bytes'] == 0
         and network['tcp_listen_called'] is False, 'Loopback sink/cleanup')
    need(all(network[key][0] == '127.0.0.1' and 0 < network[key][1] <= 65535
         for key in ['tcp_reservation', 'udp_sink']), 'Owned loopback endpoints')
    need(len(result['calls']) == 1 and result['calls'][0]['returncode'] == 0
         and result['calls'][0]['label'] == 'trusted-control-run', 'No compiler or request rerun')
    reuse = read('native-007/trusted-canary-reuse.json')
    need(reuse['bytes'] == 602400 and reuse['sha256'] == '2fd5c90f31a9007259ac880eb6b41fd7da87a454bd6ad70214e639a2538ba1c6'
         and result['calls'][0]['argv'][3] == reuse['path'], 'Exact cached canary')
    need(read('native-007-closeout.json')['cached_canary_hash_after'] == reuse['sha256'], 'Canary postflight')
    root = read('root-network-review.json')
    need(root['actual_terminal_exit'] == 0 and root['prior_failure_preserved'] is True, 'Independent root review')
    return {'status': 'pass', 'records': len(records), 'request_cases': 8,
            'network_operations': 4, 'cumulative_native_seconds': sum(elapsed),
            'original_creation_criterion': 'failed, preserved', 'scope': 'selected exact Mac controls only'}


def self_test(manifest, records):
    tests = []
    def changed(name, mutate):
        rows = deepcopy(records)
        value = parsed(rows[name]['text'])
        mutate(value)
        rows[name]['text'] = json.dumps(value)
        tests.append((manifest, rows))
    changed('native-007/result.json', lambda x: x['os_control']['network']['tcp_connect'].update(errno=61))
    changed('native-007/01-trusted-control-run.stdout', lambda x: x.update(sockets_created=0))
    changed('native-007/control-network.json', lambda x: x.update(endpoints_closed=False))
    changed('native-007/control-network.json', lambda x: x.update(sink_datagrams=1))
    changed('native-005/result.json', lambda x: x.update(status='complete'))
    changed('native-005/result.json', lambda x: x['cases'].pop())
    changed('native-005/02-trusted-control-run.stdout', lambda x: x.update(socket_denied=True))
    changed('native-006/result.json', lambda x: x['calls'].append(x['calls'][0]))
    changed('native-007-closeout.json', lambda x: x['fresh_absence'][0].update(group_absent=False))
    changed('native-007/trusted-canary-reuse.json', lambda x: x.update(sha256='0' * 64))
    claim = deepcopy(manifest); claim['claims']['normal_project_import_history'] = True
    tests.append((claim, records))
    for m, r in tests:
        try: analyze(m, r)
        except (ValueError, KeyError): pass
        else: raise AssertionError('Corrupt evidence accepted')
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        for cap in (-1, True, 8 * MIB + 1):
            try: bounded(root / 'not-opened', cap)
            except ValueError: pass
            else: raise AssertionError('Invalid read cap accepted')
        (root / 'manifest.json').write_bytes(b' ' * 65537)
        try: load(root)
        except ValueError: pass
        else: raise AssertionError('Oversized manifest accepted')
        (root / 'manifest.json').write_bytes((HERE / 'manifest.json').read_bytes())
        (root / 'records.json.gz').write_bytes(b' ' * (2 * MIB + 1))
        try: load(root)
        except ValueError: pass
        else: raise AssertionError('Oversized archive accepted')
        (root / 'records.json.gz').write_bytes((HERE / 'records.json.gz').read_bytes())
        original = globals()['bounded']
        def replace_after_read(path, cap):
            data = original(path, cap)
            if path.name == 'records.json.gz': path.write_bytes(b'replaced after bounded read')
            return data
        globals()['bounded'] = replace_after_read
        try: analyze(*load(root))
        finally: globals()['bounded'] = original
    return len(tests) + 6


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--artifacts', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    manifest, records = load()
    result = analyze(manifest, records)
    if args.originals:
        for row in records.values():
            if 'original' in row:
                need(bounded(Path(row['original']), row['bytes']) == row['text'].encode(), 'Original changed: ' + row['name'])
        result['originals'] = 'pass'
    if args.artifacts:
        need(sum(row['bytes'] for row in manifest['retained_external_artifacts']) <= 64 * MIB, 'Artifact total cap')
        for row in manifest['retained_external_artifacts']:
            need(row['bytes'] <= 4 * MIB and sha(bounded(Path(row['path']), row['bytes'])) == row['sha256'], 'Artifact changed')
        result['artifacts'] = len(manifest['retained_external_artifacts'])
    if args.self_test: result['negative_and_boundary_controls'] = self_test(manifest, records)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
