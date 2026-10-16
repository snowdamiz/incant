"""Fixed-result data oracle. No subprocesses, native loading or archive execution."""
import hashlib
import json

HEAD = 'c81eb7a95ce5fa9801a41e85f46f73d3d693dfdc'
TREE = 'a3b399a185132bba341728f8823d049ced987dde'
ENTITY = '00000000000000000000000002'
GUEST = 'f46df26fad21d80ed817a5a63ae77f88d8920adcbeaae46fd8b343cd75cede70'
PREPARED = '568f8dc69d95d25ebbd1b62b266f7b865764b3d25db4e261ce5a8691a3ceaf0d'
REGISTRY = '0326deb18341102f253a7ba8e3e0c1fa2ab3ab74c51510fbc773dbd63e6cfaf9'
PLAYER = '9deb15477ad87957cc2ddfed882aef4d237e11913ef6a71eb6d05436eade5a6b'
HEADLESS = '29efb6c0dab4c994b91ae11d04eb6172a40c4dd4bf84da932d863e9c850079eb'
FEATURES = ['physics-deterministic', 'project', 'render-input', 'static-module']
REFUSALS = {
    4: 'Error: native play startup/runtime failure: development Wasm rejected or trapped: '
       'Wasm ABI: host does not accept session restart requests',
    26: 'Error: Checkpoint(Unsupported("static module state ABI"))',
}


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def equal(actual, expected, reason):
    # Canonical JSON keeps booleans distinct from numbers.
    def normalized(value):
        if type(value) is float and value.is_integer():
            return int(value)
        if isinstance(value, list):
            return [normalized(item) for item in value]
        if isinstance(value, dict):
            return {key: normalized(item) for key, item in value.items()}
        return value
    def canonical(value):
        return json.dumps(normalized(value), sort_keys=True, allow_nan=False)
    require(canonical(actual) == canonical(expected), reason)


def session(ticks):
    return {'generation': ticks + 1, 'host_tick': ticks, 'simulation_tick': 0,
            'restarts': [{'host_step': n, 'completed_generation': n, 'simulation_tick': 1,
                          'output_frames': str(n * 800)} for n in range(1, ticks + 1)]}


def run(report, pcm):
    equal(report['session'], session(3), 'Three actual replacement boundaries')
    equal([report['tick'], report['frames']], [0, 2400], 'Reset tick/output chronology')
    require(pcm == bytes(19200) and report['pcm_sha256'] == sha(pcm), 'Exact silent PCM')
    obs = report['observations']
    equal([obs['passed'], obs['tick'], obs['tick_rate']], [True, 0, 60], 'Observation clock')
    require(len(obs['checks']) == 3, 'Complete accepted-step checks')
    for n, check in enumerate(obs['checks'], 1):
        equal(check, {'name': f'fresh-vm-{n}', 'tick': n, 'passed': True, 'actual': [1, 0, 0],
                      'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                                 'value': [1, 0, 0], 'tolerance': 0}}, 'Fresh VM observation')
    equal(obs['entities'], {ENTITY: {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1],
                                   'scale': [1, 1, 1], 'parent': None, 'velocity': None,
                                   'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0],
                                                       [0, 0, 1, 0], [0, 0, 0, 1]]}},
          'Fresh final world')
    equal(report['logs'], {'entries': [], 'dropped': 0}, 'No retained VM logs')


def authoring(data):
    first, repeated = data('evidence/002.stdout.log'), data('evidence/003.stdout.log')
    equal(data('evidence/imports.json'), {'first': first, 'repeated': repeated}, 'Import receipts')
    equal([first['changed'], first['cache_hit'], repeated['changed'], repeated['cache_hit']],
          [True, False, False, True], 'Public import/reimport behavior')
    equal([first['asset'], first['revision'], first['details']],
          [repeated['asset'], repeated['revision'], repeated['details']], 'Idempotent reimport')
    equal([first['details']['input_sha256'], first['details']['prepared_sha256'],
           first['details']['registry_sha256']], [GUEST, PREPARED, REGISTRY], 'Qualified guest')
    rpc = data('evidence/rpc-4.stderr.json')
    equal(rpc['exit'], 0, 'Authoring RPC exit')
    requests = rpc['requests']
    equal([r['request']['method'] for r in requests],
          ['project.read', 'project.read', 'command.execute', 'project.read', 'history.undo',
           'project.read', 'history.redo', 'project.read', 'project.read', 'command.execute',
           'project.read', 'history.undo', 'project.read', 'project.save', 'project.read'],
          'Public transaction/history/save sequence')
    for n, request in enumerate(requests, 1):
        equal([request['request']['id'], request['response']['id']], [n, n], 'RPC correlation')
        require('error' not in request['response'], 'RPC refusal')
    results = [r['response']['result'] for r in requests]
    before, selected = results[0]['project'], results[3]['project']
    scene = data('evidence/authored-identity.json')['scene']
    commands = requests[2]['request']['params']['commands']
    equal([c['op'] for c in commands], ['create_entity', 'set_scene_program'], 'Atomic authoring')
    equal(commands[0]['entity']['id'], ENTITY, 'Authored witness')
    program = {'kind': 'wasm', 'module': first['asset']['id']}
    equal(commands[1], {'op': 'set_scene_program', 'scene_id': scene, 'program': program},
          'Public module selection')
    equal(selected['scenes'][scene]['program'], program, 'Selected module')
    equal(results[5]['project'], before, 'Atomic Undo')
    equal(results[7]['project'], selected, 'Atomic Redo')
    require('program' not in results[10]['project']['scenes'][scene], 'Clear selection')
    equal(results[12]['project'], selected, 'Clear Undo')
    equal(results[13]['saved'], True, 'Public save')
    equal(results[13]['receipt']['candidate_sha256'],
          data('evidence/authored-identity.json')['project_sha256'], 'Saved project hash')
    reopened = data('evidence/rpc-5.stderr.json')
    equal(reopened['exit'], 0, 'Reopened RPC exit')
    require(len(reopened['requests']) == 1, 'Single reopen observation')
    equal(reopened['requests'][0]['response']['result'], results[14], 'Journal reopen')
    equal(results[14]['project'], selected, 'Saved selection')
    return first
