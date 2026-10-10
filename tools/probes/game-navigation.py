#!/usr/bin/env python3
"""Import a floor, assemble rooms through RPC, and navigate a real TS character."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(*arguments):
        return json.loads(subprocess.check_output([str(binary), *map(str, arguments)],
                          cwd=ROOT, text=True, encoding='utf-8'))

    # Original mathematical fixture, no borrowed art or external URI dependency.
    positions = [-10., 0., -6., 10., 0., -6., 10., 0., 6., -10., 0., 6.]
    data = struct.pack('<12f6H', *positions, 0, 2, 1, 0, 3, 2)
    model = {'asset': {'version': '2.0'}, 'scene': 0, 'scenes': [{'nodes': [0]}],
        'nodes': [{'mesh': 0}], 'meshes': [{'primitives': [{'attributes': {'POSITION': 0}, 'indices': 1}]}],
        'buffers': [{'byteLength': len(data), 'uri': 'data:application/octet-stream;base64,' + base64.b64encode(data).decode('ascii')}],
        'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 48}, {'buffer': 0, 'byteOffset': 48, 'byteLength': 12}],
        'accessors': [{'bufferView': 0, 'componentType': 5126, 'count': 4, 'type': 'VEC3', 'min': [-10, 0, -6], 'max': [10, 0, 6]},
                      {'bufferView': 1, 'componentType': 5123, 'count': 6, 'type': 'SCALAR'}]}
    (out / 'floor.gltf').write_text(json.dumps(model), encoding='utf-8')
    project, journal = out / 'game.incant.json', out / 'game.incant.journal.jsonl'
    run('init', project, '--name', 'Navigation gameplay', '--entities', 0)
    imported = run('import', project, 'floor.gltf')
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    floor, wall, nav, agent, collision_floor = [f'{i:026d}' for i in range(10, 15)]

    def transform(at):
        return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}

    def collider(shape):
        return {'shape': shape, 'density': 1000, 'friction': .5, 'restitution': 0,
                'sensor': False, 'memberships': 4294967295, 'filter': 4294967295}

    navigation = {'settings': {'min': [-10, -2, -6], 'max': [10, 5, 6], 'cell_size': .25,
        'cell_height': .1, 'tile_cells': 16, 'agent_radius': .5, 'agent_height': 1.8,
        'max_climb': .3, 'max_slope_degrees': 45},
        'sources': [{'entity': floor, 'geometry': 'mesh'}, {'entity': wall, 'geometry': 'collider'}]}
    entities = [
        (floor, 'Imported floor', {'Transform': transform([0, 0, 0]), 'MeshRenderer': {'mesh': imported['asset']['id'], 'materials': [], 'cast_shadows': True}}),
        (wall, 'Room wall', {'Transform': transform([0, 1.5, 0]), 'Collider': collider({'type': 'box', 'half_extents': [1, 1.5, 2]})}),
        (nav, 'Navigation', {'NavigationMesh': navigation}),
        (collision_floor, 'Collision floor', {'Transform': transform([0, -.5, 0]), 'Collider': collider({'type': 'box', 'half_extents': [10, .5, 6]})}),
        (agent, 'Chaser', {'Transform': transform([-8, .91, 0]), 'Collider': collider({'type': 'capsule', 'half_height': .6, 'radius': .3}),
            'RigidBody': {'motion': 'kinematic', 'gravity_scale': 1, 'linear_damping': 0, 'angular_damping': 0, 'can_sleep': True, 'ccd': True}}),
    ]
    commands = [{'op': 'create_entity', 'scene_id': scene, 'entity': {'id': eid, 'name': name,
                'parent': None, 'provenance': None, 'components': components}} for eid, name, components in entities]

    def rpc(requests, errors=False):
        output = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
            input=''.join(json.dumps(item) + '\n' for item in requests), cwd=ROOT, text=True, encoding='utf-8')
        replies = [json.loads(line) for line in output.splitlines()]
        assert len(replies) == len(requests)
        if not errors:
            assert all('error' not in reply for reply in replies), replies
        return replies

    authored = rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': commands,
        'expected_revision': imported['revision'], 'description': 'Assemble navigation room'}}, {'id': 2, 'method': 'project.save'}])
    text = project.read_bytes()
    rpc([{'id': 1, 'method': 'history.undo'}, {'id': 2, 'method': 'history.redo'}, {'id': 3, 'method': 'project.save'}])
    assert project.read_bytes() == text
    before_invalid = journal.read_bytes()
    current = rpc([{'id': 1, 'method': 'project.read'}])[0]['result']
    invalid = json.loads(json.dumps(navigation))
    invalid['settings']['cell_size'] = 0
    rejected = rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': [{'op': 'set_component',
        'scene_id': scene, 'entity_id': nav, 'component': 'NavigationMesh', 'value': invalid}],
        'expected_revision': current['revision'], 'description': 'Reject invalid bake settings'}}], errors=True)
    assert 'error' in rejected[0] and journal.read_bytes() == before_invalid
    source = out / 'chase.ts'
    source.write_text('import type { ScriptApi } from ' + json.dumps((ROOT / 'sdk/ts/src/index').as_posix()) + ';\n' +
        'const ids=' + json.dumps({'scene': scene, 'wall': wall, 'nav': nav, 'agent': agent}) + ';\n' + '''
export default defineBehavior<{tick:number,route:number[][],index:number,firstCorners:number,secondCorners:number,arrived:boolean}>({
  initialState:{tick:0,route:[],index:0,firstCorners:0,secondCorners:0,arrived:false},
  update(api:ScriptApi,dt,s){
    s.tick++;
    const body=api.query('RigidBody').find(e=>e.id===ids.agent)!;
    const p=(body.components.Transform as {translation:number[]}).translation;
    if(s.tick===1||s.tick===90){
      const path=api.findPath({scene_id:ids.scene,mesh_entity:ids.nav,path:{start:[p[0]!,0,p[2]!],end:[8,0,0],snap_distance:1,max_visited:1000}});
      if(!path)throw Error('Expected a reachable path');
      s.route=path.points;s.index=1;
      if(s.tick===1)s.firstCorners=path.points.length;else s.secondCorners=path.points.length;
    }
    while(s.index<s.route.length && Math.hypot(s.route[s.index]![0]!-p[0]!,s.route[s.index]![2]!-p[2]!)<.08)s.index++;
    s.arrived=s.index===s.route.length;
    const target=s.route[Math.min(s.index,s.route.length-1)]!;
    const dx=target[0]!-p[0]!,dz=target[2]!-p[2]!,distance=Math.hypot(dx,dz);
    const step=s.arrived?0:Math.min(distance,4*dt)/Math.max(distance,1e-6);
    const movement=api.computeCharacterMotion({scene_id:ids.scene,entity_id:ids.agent,translation:[dx*step,-.05,dz*step]});
    api.command({op:'set_component',scene_id:ids.scene,entity_id:ids.agent,component:'Velocity',value:{linear:movement.translation.map(v=>v/dt)}});
    if(s.tick===60)api.command({op:'set_component',scene_id:ids.scene,entity_id:ids.wall,component:'Transform',value:{translation:[0,1.5,20],rotation:[0,0,0,1],scale:[1,1,1]}});
  }
});
''', encoding='utf-8')
    subprocess.run(['node', str(ROOT / 'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
        '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], check=True, cwd=ROOT)
    compiled = out / 'chase.js'
    subprocess.run(['node', str(ROOT / 'tools/build_script.mjs'), str(source), str(compiled)], check=True, cwd=ROOT)
    (out / 'floor.gltf').unlink()
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    whole = run('play', project, '--ticks', 360, '--compiled-script', compiled)
    first = run('play', project, '--ticks', 180, '--compiled-script', compiled, '--save-output', out / 'mid.save.json')
    resumed = run('play', project, '--ticks', 180, '--compiled-script', compiled, '--load-save', out / 'mid.save.json')
    repeat = run('play', project, '--ticks', 360, '--compiled-script', compiled)
    state = whole['script_state']
    assert state['firstCorners'] > 2 and state['secondCorners'] == 2 and state['arrived'], state
    assert state == resumed['script_state'] == repeat['script_state']
    # Mesh generations and last-build reports are session-local diagnostics.
    assert whole['state']['entities'] == resumed['state']['entities'] == repeat['state']['entities']
    position = whole['state']['entities'][agent]['translation']
    assert abs(position[0] - 8) < .1 and abs(position[2]) < .1 and .89 < position[1] < .94, position
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    assert binary_hash == hashlib.sha256(binary.read_bytes()).hexdigest(), 'binary changed during probe'
    result = {'passed': True, 'strict_typescript': True, 'public_import_rpc': True, 'durable_undo_redo': True,
        'invalid_command_rollback': True, 'cooked_mesh_without_source': True, 'runtime_room_rebuild': True,
        'navmesh_character_chase': True, 'saved_path_following': True, 'repeated_gameplay_exact': True,
        'position': position, 'first_corners': state['firstCorners'], 'second_corners': state['secondCorners'],
        'author_files_unchanged': before, 'binary_sha256': binary_hash, 'visual_or_device_gate': False}
    (out / 'runs.json').write_text(json.dumps({'whole': whole, 'first': first, 'resumed': resumed, 'repeat': repeat}, indent=2) + '\n', encoding='utf-8')
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
