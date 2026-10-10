#!/usr/bin/env python3
"""Author an explicit gap connection and save/reopen a scripted mid-link traversal."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/incant_headless')
    args = parser.parse_args()
    out, binary = args.output.resolve(), args.binary.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(*arguments):
        return json.loads(subprocess.check_output([str(binary), *map(str, arguments)], cwd=ROOT,
                                                 text=True, encoding='utf-8'))

    project, journal = out / 'game.incant.json', out / 'game.incant.journal.jsonl'
    run('init', project, '--name', 'Off-mesh traversal', '--entities', 0)
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    west, east, nav, walker, link = [f'{i:026d}' for i in range(10, 15)]

    def rpc(requests, errors=False):
        text = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
            input=''.join(json.dumps(r) + '\n' for r in requests), cwd=ROOT, text=True, encoding='utf-8')
        replies = [json.loads(line) for line in text.splitlines()]
        assert len(replies) == len(requests)
        assert errors or all('error' not in r for r in replies), replies
        return replies

    def transform(position):
        return {'translation': position, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}

    navigation = {'settings': {'min': [-6, -1, -2], 'max': [6, 3, 2], 'cell_size': .1,
        'cell_height': .05, 'tile_cells': 32, 'agent_radius': .2, 'agent_height': 1.7,
        'max_climb': .2, 'max_slope_degrees': 45},
        'sources': [{'entity': eid, 'geometry': 'collider'} for eid in [west, east]],
        'links': [{'id': link, 'start': [-2.6, 0, 0], 'end': [2.6, 0, 0], 'snap_distance': .2,
                   'bidirectional': False, 'enabled': True, 'extra_cost': 0}]}
    entities = [(nav, 'Navigation', {'NavigationMesh': navigation}),
                (walker, 'Walker', {'Transform': transform([-5, .05, 0])})]
    for eid, x in [(west, -4), (east, 4)]:
        entities.append((eid, 'Island', {'Transform': transform([x, -.1, 0]), 'Collider': {
            'shape': {'type': 'box', 'half_extents': [2, .1, 2]}, 'density': 1000, 'friction': .5,
            'restitution': 0, 'sensor': False, 'memberships': 4294967295, 'filter': 4294967295}}))
    commands = [{'op': 'create_entity', 'scene_id': scene, 'entity': {'id': eid, 'name': name,
        'parent': None, 'provenance': None, 'components': components}} for eid, name, components in entities]
    revision = rpc([{'id': 1, 'method': 'project.read'}])[0]['result']['revision']
    rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': commands,
        'expected_revision': revision, 'description': 'Connect separated islands'}}, {'id': 2, 'method': 'project.save'}])
    authored = project.read_bytes()
    rpc([{'id': 1, 'method': 'history.undo'}, {'id': 2, 'method': 'history.redo'}, {'id': 3, 'method': 'project.save'}])
    assert project.read_bytes() == authored
    revision = rpc([{'id': 1, 'method': 'project.read'}])[0]['result']['revision']
    invalid = json.loads(json.dumps(navigation))
    invalid['links'][0]['extra_cost'] = -1
    before_invalid = journal.read_bytes()
    rejected = rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': [{'op': 'set_component',
        'scene_id': scene, 'entity_id': nav, 'component': 'NavigationMesh', 'value': invalid}],
        'expected_revision': revision, 'description': 'Reject invalid connection cost'}}], errors=True)
    assert 'error' in rejected[0] and journal.read_bytes() == before_invalid
    source = out / 'traverse.ts'
    source.write_text('import type { ScriptApi } from ' + json.dumps((ROOT / 'sdk/ts/src/index').as_posix()) + ';\n'
        + 'const ids=' + json.dumps({'scene': scene, 'nav': nav, 'walker': walker, 'link': link}) + ';\n' + '''
export default defineBehavior<{tick:number,points:number[][],from:number,to:number,index:number,progress:number,arrived:boolean,jumped:boolean,reverseNull:boolean}>({
  initialState:{tick:0,points:[],from:0,to:0,index:1,progress:0,arrived:false,jumped:false,reverseNull:false},
  update(api:ScriptApi,dt,s){
    s.tick++;
    if(s.tick===1){
      const request={scene_id:ids.scene,mesh_entity:ids.nav,path:{start:[-5,0,0] as [number,number,number],end:[5,0,0] as [number,number,number],snap_distance:.2,max_visited:1000}};
      const path=api.findPath(request);
      if(!path||path.traversals.length!==1||path.traversals[0]!.link_id!==ids.link)throw Error('Missing explicit traversal');
      const t=path.traversals[0]!;s.points=path.points;s.from=t.from_index;s.to=t.to_index;
      request.path.start=[5,0,0];request.path.end=[-5,0,0];s.reverseNull=api.findPath(request)===null;
    }
    const entity=api.query('Transform').find(e=>e.id===ids.walker)!;
    const p=(entity.components.Transform as {translation:number[]}).translation;
    if(s.arrived)return;
    const target=s.points[s.index]!;let next:number[];
    if(s.index===s.to){
      s.progress=Math.min(1,s.progress+dt/.6);s.jumped=true;
      const a=s.points[s.from]!,b=s.points[s.to]!,t=s.progress;
      next=a.map((v,i)=>v+(b[i]!-v)*t+(i===1?4*t*(1-t):0));
      if(t===1)s.index++;
    }else{
      const delta=target.map((v,i)=>v-p[i]!),distance=Math.hypot(...delta),step=Math.min(distance,3*dt);
      next=p.map((v,i)=>v+delta[i]!*step/Math.max(distance,1e-9));
      if(distance<=3*dt+1e-8)s.index++;
    }
    s.arrived=s.index>=s.points.length;
    api.command({op:'set_component',scene_id:ids.scene,entity_id:ids.walker,component:'Transform',value:{translation:next,rotation:[0,0,0,1],scale:[1,1,1]}});
  }
});
''', encoding='utf-8')
    subprocess.run(['node', str(ROOT / 'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
        '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], check=True, cwd=ROOT)
    compiled = out / 'traverse.js'
    subprocess.run(['node', str(ROOT / 'tools/build_script.mjs'), str(source), str(compiled)], check=True, cwd=ROOT)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    whole = run('play', project, '--ticks', 180, '--compiled-script', compiled)
    first = run('play', project, '--ticks', 60, '--compiled-script', compiled, '--save-output', out / 'mid.save.json')
    resumed = run('play', project, '--ticks', 120, '--compiled-script', compiled, '--load-save', out / 'mid.save.json')
    repeat = run('play', project, '--ticks', 180, '--compiled-script', compiled)
    assert 0 < first['script_state']['progress'] < 1, first['script_state']
    assert first['state']['entities'][walker]['translation'][1] > .1
    state = whole['script_state']
    assert state['arrived'] and state['jumped'] and state['reverseNull'], state
    assert state == resumed['script_state'] == repeat['script_state']
    assert whole['state']['entities'] == resumed['state']['entities'] == repeat['state']['entities']
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    assert binary_hash == hashlib.sha256(binary.read_bytes()).hexdigest()
    result = {'passed': True, 'strict_typescript': True, 'public_commands': True, 'durable_undo_redo': True,
        'invalid_cost_rollback': True, 'explicit_link_traversal': True, 'one_way_reverse_null': True,
        'save_in_flight_exact': True, 'repeat_exact': True, 'mid_link_progress': first['script_state']['progress'],
        'author_files_unchanged': before, 'binary_sha256': binary_hash, 'physics_or_visual_gate': False}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    (out / 'runs.json').write_text(json.dumps({'whole': whole, 'first': first, 'resumed': resumed, 'repeat': repeat}, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
