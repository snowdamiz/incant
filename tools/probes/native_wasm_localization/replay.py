"""Data-only replay; never runs a recorded binary, command, or guest source."""
import json
from pathlib import Path

from ..native_wasm_checkpoint.replay import audit_build, data, managed_logs, read
from ..native_wasm_checkpoint.route import inventory
from ..native_wasm_restart.verify import equal, require
from . import author, fixture as f, oracle


def authoring(root):
    paths = sorted(root.glob('rpc-*.stderr.json'), key=lambda p: int(p.name.split('-')[1].split('.')[0]))
    traces = [data(root, path.name) for path in paths]
    require(len(traces) == 4 and all(t['exit'] == 0 for t in traces), 'Four closed public journal owners')
    methods = [
        ['project.read', 'command.execute', 'project.read', 'history.undo', 'project.read', 'history.redo',
         'project.read', 'project.read', 'command.execute', 'project.read', 'project.save', 'project.read'],
        ['project.read', 'command.execute', 'script.read', 'project.save', 'project.read'],
        ['project.read', 'history.undo', 'project.read', 'project.save', 'project.read'], ['project.read']]
    for index, (trace, expected) in enumerate(zip(traces, methods)):
        equal([row['request']['method'] for row in trace['requests']], expected, 'Fixed public authoring lifecycle')
        for at, row in enumerate(trace['requests']):
            reply = row['response']
            equal([row['request']['id'], reply['id']], [at + 1] * 2, 'RPC correlation')
            refused = index == 0 and at == 8
            require(('error' in reply) == refused and ('result' in reply) != refused, 'Only deliberate refusal')
    result = lambda i, n: traces[i]['requests'][n]['response']['result']
    before, selected, saved = result(0, 0), result(0, 2), result(0, 11)
    equal(result(0, 4)['project'], before['project'], 'Undo all fixture changes')
    equal(result(0, 6)['project'], selected['project'], 'Redo fixture exactly')
    equal(result(0, 9), result(0, 7), 'Invalid locale plus rename leaves revision/content unchanged')
    equal(saved['project'], selected['project'], 'Save exact selected project')
    equal(result(1, 0), saved, 'New journal reader')
    equal(result(2, 0), result(1, 4), 'TypeScript execution does not mutate authored project')
    equal(result(2, 2)['project'], saved['project'], 'Undo temporary TypeScript selection')
    equal(result(3, 0), result(2, 4), 'Final Wasm journal reopen')
    equal(result(3, 0), data(root, 'authored-read.json'), 'Final public read evidence')
    imported, identity = data(root, 'imports.json'), data(root, 'authored.json')
    command = traces[0]['requests'][1]['request']['params']
    equal(command['commands'], author.commands(identity['scene'], imported['first']['asset']['id'],
                                              imported['audio']['asset']['id']), 'Exact fixture through command bus')
    require(command['expected_revision'] == before['revision']
            and result(0, 1)['revision'] == before['revision'] + 1, 'One atomic authoring revision')
    source_command = traces[1]['requests'][1]['request']['params']['commands'][0]
    require(source_command['op'] == 'write_script' and source_command['expected_source'] == {'state': 'missing'},
            'Public create-only source transaction')
    require(oracle.sha(source_command['source'].encode()) == identity['reference_source']['sha256'], 'Actual TS source binding')
    equal(result(1, 2)['untrusted_source_data']['source'], source_command['source'], 'Read exact authored TS source')


def replay_records(root):
    root = Path(root)
    owner, authored = data(root, 'owner.json'), data(root, 'authored.json')
    require(owner['passed'] is True and owner['error'] is None and owner['seconds'] <= 5400, 'Successful bounded owner')
    for name, cap in [('target', 12 * 1024**3), ('scratch', 8 * 1024**3),
                      ('output', 2 * 1024**3), ('rss', 8 * 1024**3)]:
        require(type(owner['peaks'][name]) is int and 0 < owner['peaks'][name] <= cap, 'Resource ceiling: ' + name)
    managed_logs(root, owner['calls'])
    authoring(root)
    equal(data(root, 'expected-coverage.json'), [f.rows(phase) for phase in range(4)], 'Independent canonical coverage')
    outputs = {}
    for name, (start, end) in oracle.RUNS.items():
        equal(data(root, name + '-input.json'), f.recording(end - start), 'Actual positive input')
        equal(data(root, name + '-observe.json'), f.observation(start, end), 'Actual observation plan')
        outputs[name] = {'report': data(root, name + '.json'), 'pcm': read(root, name + '.pcm', 256000),
                         'saved': read(root, name + '.save')}
    reference = data(root, 'reference.json')
    oracle.joined(outputs, reference, read(root, 'reference.pcm', 256000))
    equal(data(root, 'reference-observe.json'), f.observation(0, f.TICKS), 'TS reference plan')
    reference_calls = [c for c in owner['calls'] if len(c['argv']) > 1 and c['argv'][1] == 'native-play']
    require(len(reference_calls) == 1 and reference_calls[0]['exit'] == 0, 'One actual TS native reference')
    equal(json.loads(read(root, reference_calls[0]['stdout']['path'])), reference, 'Reference bound to managed stdout')
    player_pids = set()
    for name in [*oracle.RUNS, *f.FAILURES.values()]:
        receipt = data(root, name + '-process.json')
        calls = [c for c in owner['calls'] if c['argv'] == receipt['argv']]
        require(len(calls) == 1 and calls[0]['exit'] == receipt['returncode'], 'One actual call for each control')
        call = calls[0]
        require(type(call['pid']) is int and call['pid'] > 0 and call['pid'] not in player_pids,
                'Distinct player process')
        player_pids.add(call['pid'])
        for stream in ('stdout', 'stderr'):
            equal(read(root, call[stream]['path']).decode(), receipt[stream], 'Actual retained process stream')
        require('--package' in receipt['argv'] and '--static-module' not in receipt['argv'], 'Ordinary package selection')
        if name in oracle.RUNS:
            equal(json.loads(receipt['stdout']), outputs[name]['report'], 'Actual positive report')
        else:
            key = next(key for key, value in f.FAILURES.items() if value == name)
            equal(data(root, name + '-input.json'), f.recording(1, key), 'Exact ignored-error input')
            oracle.refusal(receipt, name)
    exported = data(root, 'package/export.json')
    program = {'kind': 'wasm', 'module': authored['module'], 'fingerprint': authored['fingerprint'],
        'input_sha256': authored['guest_sha256'], 'prepared_sha256': authored['prepared_sha256'],
        'registry_sha256': authored['registry_sha256'], 'translator': 'wasm2c-1.0.42'}
    equal(exported['program'], program, 'Captured authored guest linkage')
    equal(data(root, 'package/project-build.json')['program'], program, 'Actual generated player linkage')
    equal(exported['project_sha256'], authored['project_sha256'], 'Actual authored package identity')
    equal(exported['player_sha256'], data(root, 'package-files.json')['incant_game_player']['sha256'], 'Copied executable')
    equal(data(root, 'guest.json')['artifact']['sha256'], authored['guest_sha256'], 'Compiled SDK guest import identity')
    require(exported['build_storage_removed_before_startup'] is True and exported['startup_source_parsing'] is False,
            'Actual ordinary source-free startup')
    audit_build(root, owner, exported)
    return {'passed': True, 'positive_player_processes': 4, 'callback_refusal_processes': 6,
            'actual_ts_native_reference': True, 'player_sha256': exported['player_sha256'],
            'callback_internal_rollback_retested_here': False, 'gpu_executed': False}


def replay(root):
    root = Path(root)
    equal(inventory(root / 'package/player'), data(root, 'package-files.json'), 'Immutable runtime bytes and modes')
    for name in ('authoring', 'trusted-wabt', 'guest-build'):
        require(not (root / name).exists() and not (root / name).is_symlink(), 'Owned source/build removed')
    require(not list((root / 'private-builds').iterdir()), 'Private exporter scratch removed')
    return replay_records(root)
