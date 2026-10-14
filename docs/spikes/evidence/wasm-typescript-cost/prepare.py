"""Bind actual public transactions, workload membership and captured guest identity."""
import copy
import gzip
import hashlib
import json
from oracle import COUNT, owner


def audit(read, data, rows, frozen):
    p = data('prepare-003/prepared.json')
    assert (p['format'], p['version'], p['entities'], p['ticks']) == ('incant-wasm-ts-workload', 1, 1024, 3600)
    assert p['binary_sha256'] == rows['build-001/incant_headless']['sha256']
    assert p['guest'] == data('prepare-003/guest-compile.stdout')
    for leaf in ('init', 'guest-compile', 'import'):
        c = data('prepare-003/' + leaf + '.json')
        assert c['exit'] == 0
        for suffix in ('.stdout', '.stderr'):
            assert c[suffix + '_sha256'] == rows['prepare-003/' + leaf + suffix]['sha256']
    for name, sha in p['files'].items():
        assert rows['prepare-003/' + name]['sha256'] == sha
    for name, sha in p['guest']['sdk']['source_sha256'].items():
        assert frozen[name] == sha
    for name, sha in p['source'].items():
        assert frozen['tools/probes/wasm-ts-cost/' + name] == sha
    ts, wasm = (data('prepare-003/' + kind + '/project.json') for kind in ('typescript', 'wasm'))
    scene = p['scene_id']
    assert ts['id'] == wasm['id'] == p['project_id']
    assert set(ts['scenes']) == set(wasm['scenes']) == {scene}
    for kind, project in [('typescript', ts), ('wasm', wasm)]:
        entities = project['scenes'][scene]['entities']
        assert set(entities) == {owner(n) for n in range(COUNT)}
        for n in range(COUNT):
            components = entities[owner(n)]['components']
            assert components['Transform'] == {'translation': [n / 8, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
            assert set(components) == ({'Transform', 'Script'} if kind == 'typescript' and n == 0 else {'Transform'})
            assert entities[owner(n)]['parent'] is None
    assert ts['settings']['tick_rate'] == 60 and ts['settings'].get('physics_mode', 'deterministic') == 'deterministic'
    assert len(ts['scripts']) == 1 and not wasm['scripts'] and not ts['assets'] and len(wasm['assets']) == 1
    left, right = copy.deepcopy(ts), copy.deepcopy(wasm)
    left['scripts'] = right['scripts'] = {}
    left['assets'] = right['assets'] = {}
    del left['scenes'][scene]['entities'][owner(0)]['components']['Script']
    left['scenes'][scene]['entities'][owner(0)]['provenance'] = right['scenes'][scene]['entities'][owner(0)]['provenance']
    right['scenes'][scene].pop('program')
    assert left == right
    recorded = data('prepare-003/input.json')
    assert recorded['ticks'] == 3600 and recorded['tick_rate'] == 60 and recorded['start_tick'] == 0
    expected_frames = [{'tick': t, 'events': [{'type': 'key', 'code': 'ArrowRight', 'down': down}]}
                       for base in range(0, 3600, 600) for t, down in ((base + 1, True), (base + 301, False))]
    assert recorded['frames'] == expected_frames
    imported = data('prepare-003/import.stdout')
    m = p['module']
    assert imported['details']['input_sha256'] == m['input_sha256'] == rows['prepare-003/wasm/behavior.wasm']['sha256']
    for key in ('prepared_sha256', 'registry_sha256'):
        assert imported['details'][key] == m[key]
    assert m['registry_sha256'] == frozen['sdk/abi/native-v1.json']
    identity = '\0'.join(('incant-wasm-capture-v1-wabt-1.0.42-fixed-memory-meter-v1',
                          m['registry_sha256'], m['input_sha256'], m['prepared_sha256']))
    assert hashlib.sha256(identity.encode()).hexdigest() == m['fingerprint'] == imported['asset']['sha256']
    transactions = 0
    for name, count in [('base-author', 5), ('ts-author', 1), ('wasm-author', 1)]:
        prefix = 'prepare-003/' + name
        receipt = data(prefix + '.json')
        assert receipt['exit_code'] == 0 and receipt['failure'] is None and receipt['owned_process_reaped'] is True
        lines = gzip.decompress(read(prefix + '.rpc.jsonl.gz')).splitlines()
        calls = data(prefix + '.transactions.json')['calls']
        assert len(lines) == len(calls) * 2 == receipt['calls'] * 2
        history, index, expected, adds = [], 0, None, 0
        for i, call in enumerate(calls):
            req, res = json.loads(lines[i * 2])['request'], json.loads(lines[i * 2 + 1])['response']
            for raw, key, sha in [(lines[i * 2], b'{"request":', call['request_sha256']),
                                  (lines[i * 2 + 1], b'{"response":', call['response_sha256'])]:
                assert hashlib.sha256(raw.removeprefix(key).removesuffix(b'}') + b'\n').hexdigest() == sha
            assert req['id'] == res['id'] and 'error' not in res and req['method'] == call['method']
            method = req['method']
            if method == 'command.execute':
                assert res['result']['revision'] == req['params']['expected_revision'] + 1
                expected = 'new'; adds += 1
            elif method in ('history.undo', 'history.redo'):
                index += -1 if method == 'history.undo' else 1
                expected = history[index]
            elif method == 'project.read':
                project = res['result']['project']
                if not history:
                    history = [project]
                elif expected == 'new':
                    history = history[:index + 1] + [project]; index += 1
                elif expected is not None:
                    assert project == expected
                expected = None
        assert adds == count and index == count
        assert sum(c['method'] == 'history.undo' for c in calls) == count
        assert sum(c['method'] == 'history.redo' for c in calls) == count
        reopened = [json.loads(line) for line in gzip.decompress(read(prefix + '-reopen.rpc.jsonl.gz')).splitlines()]
        assert reopened[1]['response']['result'] == res['result']
        reopened_receipt = data(prefix + '-reopen.json')
        assert reopened_receipt['owned_process_reaped'] is True and reopened_receipt['exit_code'] == 0
        transactions += count
    return {'entities_each': COUNT, 'public_transactions_undone_redone': transactions, 'reopens': 3, 'guest': m}
