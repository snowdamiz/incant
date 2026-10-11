"""Real retained RPC controls using the trusted, copied developer player.

Called only after the native-project-play coordinator removes compiler inputs.
No mocks, screenshot capture, arbitrary executable selection or project edits.
"""
import json


def exercise(run, project, digest, entity, rate, out):
    def require(condition, message):
        if not condition:
            raise ValueError(message)

    def call(method, **params):
        return {'method': method, 'params': params}

    def transcript(label, requests):
        for index, request in enumerate(requests):
            request['id'] = index
        result = run(['rpc', project], input=''.join(json.dumps(r)+'\n' for r in requests), check=True)
        responses = [json.loads(line) for line in result.stdout.splitlines()]
        require(len(responses) == len(requests), 'RPC response count')
        require(all(r['id'] == i for i, r in enumerate(responses)), 'RPC response identity')
        (out/f'rpc-{label}.json').write_text(json.dumps(responses, indent=2)+'\n')
        return responses

    def ok(response):
        require('error' not in response and 'result' in response, f'RPC failure: {response}')
        return response['result']

    def failed(response, reason):
        require('result' not in response and reason in response.get('error', {}).get('message', ''),
                f'RPC expected refusal ({reason}): {response}')

    right = {'type': 'key', 'code': 'KeyD', 'down': True}
    baseline = None
    for repeat in range(2):
        requests = [call('play.start', script_sha256=digest)]
        requests.extend(call('play.step', events=[right] if tick == 1 else []) for tick in range(1, 6))
        requests.extend([call('play.state', entities=[entity], include_input=True, include_logs=True),
                         call('play.stop'), call('project.read')])
        responses = transcript(f'success-{repeat}', requests)
        start = ok(responses[0])
        require(start['tick'] == 0 and start['runtime'] == 'native', 'RPC initial native state')
        require(start['script_source_sha256'] == digest and not start['script_source_parsing'], 'RPC behavior identity')
        for index in range(1, 6):
            step = ok(responses[index])
            require(step['tick'] == index and step['entities'] == {}, 'RPC default must not enumerate world')
        state = ok(responses[6])
        require(abs(state['entities'][entity]['transform']['translation'][0] - 14/rate) < 1e-10,
                'RPC native behavior motion')
        require(state['entities'][entity]['velocity']['linear'] == [3, 0, 0], 'RPC behavior velocity')
        require(len(state['logs']['entries']) == 5 and state['logs']['entries'][-1]['message'] == 'accepted 5 🚀',
                'RPC committed logs/Unicode')
        require(state['input']['actions']['right']['active'] and not state['input']['actions']['right']['pressed'],
                'RPC held input edges')
        require(not ok(responses[7])['playing'], 'RPC stop')
        authored = ok(responses[8])
        require(authored['revision'] == 0, 'RPC gameplay entered authoring journal')
        if baseline is not None:
            require(responses == baseline, 'RPC fresh-process drift')
        baseline = responses

    responses = transcript('failed-session', [
        call('play.start', script_sha256=digest),
        call('play.step', events=[right, {'type': 'key', 'code': 'Space', 'down': True}]),
        call('play.step'), call('play.step'),
        call('play.state', entities=[entity], include_input=True, include_logs=True),
        call('play.step'), call('play.stop'),
        call('play.start', script_sha256=digest),
        call('play.step', observe={'include_logs': True}),
    ])
    require(ok(responses[1])['tick'] == 1 and ok(responses[2])['tick'] == 2, 'RPC pre-failure ticks')
    failed(responses[3], 'later failure')
    failed(responses[4], 'failed')
    failed(responses[5], 'failed')
    require(not ok(responses[6])['playing'] and ok(responses[7])['tick'] == 0, 'RPC restart after failure')
    restarted = ok(responses[8])
    require(restarted['tick'] == 1 and len(restarted['logs']['entries']) == 1, 'RPC restart resets VM/log lifetime')

    responses = transcript('mismatched-script', [
        call('play.start', script_sha256='0'*64), call('play.state'),
    ])
    failed(responses[0], 'source hash')
    failed(responses[1], 'no play session')
    return {'successful_processes': 2, 'failed_session_processes': 1, 'mismatched_script_processes': 1,
            'source_parsing': False, 'full_world_serialization': False, 'gpu_submitted': False}
