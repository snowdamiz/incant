#!/usr/bin/env python3
"""Public-CLI physics behavior, persistence and sandbox probe; no pixel claims."""
import argparse
from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='new output directory')
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = str(args.binary.resolve())
    project, journal = out / 'project.incant.json', out / 'history.jsonl'
    records = []

    def run(*arguments):
        return subprocess.check_output([binary, *map(str, arguments)], text=True, cwd=ROOT)

    @contextmanager
    def rpc():
        proc = subprocess.Popen([binary, 'rpc', str(project), '--journal', str(journal)],
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)
        def call(method, params=None, success=True):
            request = {'id': len(records), 'method': method, 'params': params or {}}
            proc.stdin.write(json.dumps(request) + '\n')
            proc.stdin.flush()
            response = json.loads(proc.stdout.readline())
            records.append({'request': request, 'response': response})
            assert ('error' not in response) == success, response
            return response.get('result', response.get('error'))
        try:
            yield call
        finally:
            proc.stdin.close()
            assert proc.wait(timeout=15) == 0
            proc.stdout.close()

    run('init', project, '--entities', 0, '--name', 'Physics CLI verification')
    sid = next(iter(json.loads(project.read_text())['scenes']))
    ground, body = '00000000000000000000000010', '00000000000000000000000011'
    def transform(y):
        return {'translation': [0, y, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
    def collider(extents):
        return {'shape': {'type': 'box', 'half_extents': extents}, 'density': 1000,
                'friction': 0.5, 'restitution': 0, 'sensor': False,
                'memberships': 4294967295, 'filter': 4294967295}
    rigid = {'motion': 'dynamic', 'gravity_scale': 1, 'linear_damping': 0,
             'angular_damping': 0, 'can_sleep': True, 'ccd': True}
    with rpc() as call:
        initial = call('project.read')
        commands = []
        for eid, name, components in [
            (ground, 'Floor', {'Transform': transform(-0.5), 'Collider': collider([4, 0.5, 4])}),
            (body, 'Falling body', {'Transform': transform(4), 'Collider': collider([0.5]*3), 'RigidBody': rigid}),
        ]:
            commands.append({'op': 'create_entity', 'scene_id': sid, 'entity': {
                'id': eid, 'name': name, 'parent': None, 'components': components, 'provenance': None}})
        created = call('command.execute', {'commands': commands, 'expected_revision': initial['revision'], 'description': 'Create physics scene'})
        assert created['revision'] == initial['revision'] + 1
        before = call('project.read')
        invalid = {'op':'set_component', 'scene_id':sid, 'entity_id':body, 'component':'Transform', 'value':{**transform(4),'scale':[2,1,1]}}
        call('command.execute', {'commands':[invalid], 'expected_revision':before['revision']}, success=False)
        assert call('project.read') == before
        undone = call('history.undo')
        assert body not in undone['project']['scenes'][sid]['entities']
        redone = call('history.redo')
        assert redone['project'] == before['project']
        call('project.save')
    with rpc() as call:
        reopened = call('project.read')
        assert reopened['project'] == before['project']
        assert len(call('history.read')) == 1
        call('play.start')
        for _ in range(120):
            state = call('play.step')
        assert abs(state['entities'][body]['translation'][1] - 0.5) < 0.02
        stopped = call('play.stop')
        assert stopped['project'] == before['project']
    baseline = {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in [project, journal]}
    plain = json.loads(run('run', project, '--ticks', 360))['state']
    repeated = json.loads(run('run', project, '--ticks', 360))['state']
    assert plain == repeated
    source = out / 'behavior.ts'
    source.write_text('''export default defineBehavior({
  initialState: { tick: 0, speed: 0, hit: '' },
  update(api, dt, state) {
    state.tick++;
    const body = api.query('RigidBody')[0];
    state.speed = body.components.Velocity.linear[1];
    const hit = api.raycast({ scene_id: body.scene_id, origin:[0,10,0], direction:[0,-1,0],
      max_distance:20, include_sensors:false, exclude_entity:body.id, memberships:4294967295, filter:4294967295 });
    state.hit = hit.entity_id;
    if(state.tick === 121) api.command({ op:'set_component', scene_id:body.scene_id,
      entity_id:body.id, component:'Velocity', value:{linear:[0,5,0]} });
    if(state.tick === 122) api.log('physics jump and raycast passed');
  }
});
''')
    compiled = out / 'behavior.js'
    subprocess.run(['node', str(ROOT/'tools/build_script.mjs'), str(source), str(compiled)], check=True, cwd=ROOT)
    report = json.loads(run('play', project, '--ticks', 122, '--compiled-script', compiled))
    assert report['completed'] and report['script_commands'] == 1
    assert report['script_state']['hit'] == ground and report['script_state']['speed'] > 4
    assert report['state']['entities'][body]['translation'][1] > 0.55
    assert len(report['logs']) == 1
    assert baseline == {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in [project, journal]}
    (out/'rpc.json').write_text(json.dumps(records, indent=2)+'\n')
    (out/'play.json').write_text(json.dumps(report, indent=2)+'\n')
    (out/'direct.json').write_text(json.dumps(plain, indent=2)+'\n')
    result = {'passed': True, 'public_rpc': True, 'atomic_invalid_rejection': True,
              'undo_redo_and_persistent_reopen': True, 'repeated_runtime_exact': True,
              'sandbox_query_and_velocity_command': True, 'author_files_unchanged': baseline,
              'settled_y': plain['entities'][body]['translation'][1],
              'jump_y': report['state']['entities'][body]['translation'][1],
              'live_platform_gate': False}
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
