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


def artifact_sha(path, size):
    """Hash a retained native artifact without allocating its binary contents."""
    need(type(size) is int and 0 <= size <= 64 * MIB, 'Artifact file cap')
    digest = hashlib.sha256()
    remaining = size
    with path.open('rb') as stream:
        while remaining:
            chunk = stream.read(min(65536, remaining))
            need(chunk, 'Truncated artifact')
            digest.update(chunk)
            remaining -= len(chunk)
        need(not stream.read(1), 'Appended artifact')
    return digest.hexdigest()


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
         'normal_project_import_history': True, 'source_free_export_rerun': False,
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
    authored = analyze_authored(manifest, records)
    elapsed.append(authored['elapsed_seconds'])
    need(math.isclose(sum(elapsed), manifest['cumulative_native_seconds'], abs_tol=1e-8), 'All native time retained')
    need(math.isclose(180 - sum(elapsed), manifest['remaining_seconds'], abs_tol=1e-8)
         and manifest['original_native_account_seconds'] == 180, 'Original account')
    return {'status': 'pass', 'records': len(records), 'request_cases': 8,
            'network_operations': 4, 'cumulative_native_seconds': sum(elapsed),
            'authored': authored, 'original_creation_criterion': 'failed, preserved',
            'scope': 'selected exact Mac controls and ordinary public authoring only'}


def analyze_authored(manifest, records):
    def read(name): return parsed(records['authored-001/' + name]['text'])
    result, receipt = read('result.json'), read('authored.json')
    close = parsed(records['authored-001-closeout.json']['text'])
    plan = parsed(records['authored-packet-001/plan.json']['text'])
    equivalent = parsed(records['authored-packet-001/source-equivalence.json']['text'])
    need(close['status'] == result['status'] == manifest['authored']['status'] == 'complete'
         and close['actual_tool_session'] == 29405 and close['actual_tool_exit'] == 0, 'Authored actual terminal')
    need(close['source'] == result['source'] == plan['source'] == manifest['authored']['source']
         == '8d6dffcc1aa55fa3612c17cf8a2225f8d250aab0', 'Authored controller producer')
    need(len(close['fresh_absence']) == 7 and all(row['pid_absent'] is True
         and row['group_absent'] is True for row in close['fresh_absence']), 'Authored fresh absence')
    need(close['postflight_bindings'] == {'count': 177, 'mismatches': []}, 'Authored immutable bindings')
    need(receipt == result['authored'] and receipt['headless'] == plan['headless']
         and receipt['raw'] == plan['raw'], 'Exact authored inputs/receipt')
    need(receipt['headless']['producer'] == result['headless_producer'] == equivalent['headless_producer']
         == '9d44372ae379c6d0372ff48854828e837f1fb747'
         and receipt['headless']['bytes'] == 63423424
         and receipt['headless']['sha256'] == 'f0fb0cf5c654dd03e4ad77e57901586cb6abb426a923dd77f86af169f272641f',
         'Retained headless producer, never relabelled current')
    need(len(equivalent['files']) == 75 and len({row['path'] for row in equivalent['files']}) == 75
         and equivalent['service_code_source'] == manifest['source']
         and equivalent['report_source'] == result['source'], 'Selected public-route equality scope')
    for row in equivalent['files']:
        source = records['authored-public-route-source/' + row['path']]
        need(source['bytes'] == row['bytes'] and source['sha256'] == row['sha256'], 'Public-route source snapshot')
    project = Path(receipt['project'])
    need(receipt['default_journal'] == str(project.with_suffix('.journal.jsonl'))
         and plan['journal'] == 'authoring/project.journal.jsonl, both CLI/RPC default project.with_extension(journal.jsonl)',
         'Same public CLI/RPC default journal')
    owned = result['owned']
    need(len(owned) == 6 and [row['exit'] for row in owned] == [0, 0, 0, 0, 1, 0]
         and all(row['cleanup_confirmed'] is True and row['argv'][0] == receipt['headless']['path'] for row in owned),
         'Six exact headless owner closures')
    need([row['argv'][1:] for row in owned] == [
         ['init', str(project), '--entities', '0'], ['import', str(project), 'game.wasm'],
         ['import', str(project), 'game.wasm'], ['rpc', str(project)],
         ['import', str(project), 'bad.wasm'], ['rpc', str(project)]], 'No alternate journal/route')
    need(len(result['calls']) == 4 and [row['exit'] for row in result['calls']] == [0, 0, 0, 1], 'Four CLI calls')
    imports = read('imports.json')
    first, again = imports['first'], imports['repeated']
    need(first == read('001.stdout.log') and again == read('002.stdout.log')
         and first['changed'] is True and again['changed'] is False and again['cache_hit'] is True
         and first['asset'] == again['asset'] and first['revision'] == again['revision'], 'Import/idempotent reimport')
    need(first['details']['input_sha256'] == receipt['original_raw_sha256'] == plan['raw']['sha256']
         == '8d86dbd4e85120728eb69f9792f854242c7036443277017571552314d01a65d9'
         and receipt['capture']['input_sha256'] == plan['raw']['sha256']
         and receipt['capture']['prepared_sha256'] == first['details']['prepared_sha256'], 'Service raw to normal capture')
    protocol = read('rpc-3.stderr.json')
    rows = protocol['requests']
    need(protocol['exit'] == 0 and [row['request']['method'] for row in rows] == [
         'project.read', 'history.read', 'command.execute', 'project.read', 'history.read',
         'history.undo', 'project.read', 'history.redo', 'project.read', 'project.read', 'history.read',
         'command.execute', 'project.read', 'history.read', 'project.save', 'project.read', 'history.read'], 'Actual RPC sequence')
    need(all(row['request']['id'] == row['response']['id'] == index + 1
         for index, row in enumerate(rows)), 'Matched RPC response IDs')
    values = [row['response'].get('result') for row in rows]
    need(values[6]['project'] == values[0]['project'] and values[8]['project'] == values[3]['project'], 'Undo/Redo exact project')
    need(values[2]['revision'] == values[0]['revision'] + 1
         and rows[2]['request']['params']['expected_revision'] == values[0]['revision'], 'Atomic selection revision')
    need(rows[2]['request']['params']['commands'] == [{'op': 'set_scene_program', 'scene_id': receipt['scene'],
         'program': {'kind': 'wasm', 'module': receipt['module']}}], 'Ordinary SetSceneProgram command')
    need(values[3]['project']['scenes'][receipt['scene']]['program'] == {'kind': 'wasm', 'module': first['asset']['id']}
         and values[4][:-1] == values[1] and values[4][-1]['actor'] == {
         'origin': 'user', 'actor': 'editor', 'model': None, 'conversation_id': None}, 'Selected asset and engine actor')
    need(rows[11]['response'] == receipt['stale_refusal'] == {'id': 12, 'error': {'message':
         f"document revision conflict: expected {values[0]['revision']}, current {values[9]['revision']}"}}
         and values[12] == values[9] and values[13] == values[10], 'Exact stale refusal/unchanged project and history')
    need(rows[14]['request']['params'] == {'expected_revision': values[9]['revision']}
         and values[15] == receipt['saved'] and values[16] == receipt['history'], 'Normal save')
    reopen = read('rpc-5.stderr.json')
    need(reopen['exit'] == 0 and [r['request']['method'] for r in reopen['requests']] == ['project.read', 'history.read']
         and [r['response']['result'] for r in reopen['requests']] == [values[15], values[16]], 'Saved journal reopen')
    need(read('authoring/project.json') == values[15]['project']
         and records['authored-001/003.stdout.log']['text'] == ''
         and 'static Wasm preparation: expected core Wasm <= 1 MiB' in records['authored-001/003.stderr.log']['text']
         and receipt['malformed_import_project_journal_history_unchanged'] is True, 'Malformed import refusal and byte-snapshot assertion')
    need(receipt['no_direct_document_mutation'] is True and receipt['no_export_or_runtime_execution'] is True
         and result['no_compiler_export_runtime_device_or_window'] is True, 'Authored scope')
    need(result['elapsed_seconds'] == manifest['authored']['elapsed_seconds']
         and 0 < result['elapsed_seconds'] <= 60, 'Authored bounded elapsed')
    independent = parsed(records['root-authored-review.json']['text'])
    need(independent['status'] == 'pass' and independent['actual_native_session'] == 29405
         and independent['actual_native_exit'] == 0 and independent['bindings_rehashed'] == 177
         and independent['result_sha256'] == records['authored-001/result.json']['sha256']
         and independent['closeout_sha256'] == records['authored-001-closeout.json']['sha256']
         and independent['rpc_record_sha256'] == records['authored-001/rpc-3.stderr.json']['sha256']
         and independent['reopen_record_sha256'] == records['authored-001/rpc-5.stderr.json']['sha256'], 'Independent authored review')
    return {'status': 'pass', 'owners': 6, 'rpc_requests': len(rows) + 2, 'elapsed_seconds': result['elapsed_seconds']}


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
    changed('authored-001/rpc-3.stderr.json', lambda x: x['requests'][6]['response']['result']['project'].update(name='bad undo'))
    changed('authored-001/rpc-3.stderr.json', lambda x: x['requests'][11]['response']['error'].update(message='unrelated failure'))
    changed('authored-001/rpc-3.stderr.json', lambda x: x['requests'][4]['response']['result'][-1]['actor'].update(actor='project'))
    changed('authored-001/rpc-5.stderr.json', lambda x: x['requests'][0]['response']['result'].update(revision=99))
    changed('authored-001/result.json', lambda x: x['owned'][3]['argv'].append('different.journal'))
    changed('authored-001/imports.json', lambda x: x['repeated'].update(changed=True))
    changed('authored-001/authored.json', lambda x: x['headless'].update(producer='0' * 40))
    changed('authored-001-closeout.json', lambda x: x['fresh_absence'][0].update(pid_absent=False))
    changed('authored-001/authored.json', lambda x: x.update(malformed_import_project_journal_history_unchanged=False))
    claim = deepcopy(manifest); claim['claims']['source_free_export_rerun'] = True
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
        artifact = root / 'artifact'
        artifact.write_bytes(b'abc')
        need(artifact_sha(artifact, 3) == sha(b'abc'), 'Streaming artifact positive')
        for size in (2, 4, -1, True, 64 * MIB + 1):
            try: artifact_sha(artifact, size)
            except ValueError: pass
            else: raise AssertionError('Wrong artifact size accepted')
    return len(tests) + 11


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
        need(sum(row['bytes'] for row in manifest['retained_external_artifacts']) <= 128 * MIB, 'Artifact total cap')
        for row in manifest['retained_external_artifacts']:
            need(artifact_sha(Path(row['path']), row['bytes']) == row['sha256'], 'Artifact changed')
        result['artifacts'] = len(manifest['retained_external_artifacts'])
    if args.self_test: result['negative_and_boundary_controls'] = self_test(manifest, records)
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
