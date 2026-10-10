#!/usr/bin/env python3
"""Exercise authored grid costs, door changes and saved gameplay through public APIs."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    binary_sha = digest(binary)

    def run(*argv):
        return json.loads(subprocess.check_output([str(binary), *map(str, argv)], cwd=ROOT, text=True))

    project, journal = out/'grid.incant.json', out/'grid.incant.journal.jsonl'
    run('init', project, '--name', 'Weighted grid doors', '--entities', 0)
    scene = next(iter(json.loads(project.read_text())['scenes']))
    grid_id, actor_id = f'{1:026}', f'{2:026}'
    costs = [1]*35
    for row in range(5):
        costs[row*7+3] = 0
    costs[10], costs[24] = 1, 4

    def rpc(requests):
        text = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
            cwd=ROOT, input=''.join(json.dumps(q)+'\n' for q in requests), text=True)
        return [json.loads(line) for line in text.splitlines()]

    def execute(commands, description):
        revision = rpc([{'id': 0, 'method': 'project.read'}])[0]['result']['revision']
        return rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': commands,
            'expected_revision': revision, 'description': description}}, {'id': 2, 'method': 'project.save'}])

    commands = []
    for ident, name, components in [
        (grid_id, 'Grid', {'NavigationGrid': {'dimensions': [7,5], 'costs': costs}}),
        (actor_id, 'Walker', {'Transform': {'translation': [0,0,2], 'rotation': [0,0,0,1], 'scale': [1,1,1]}}),
    ]:
        commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': ident, 'name': name, 'parent': None, 'provenance': None, 'components': components}})
    assert all('error' not in reply for reply in execute(commands, 'Create weighted grid and walker'))
    authored = json.loads(project.read_text())
    assert authored['scenes'][scene]['entities'][grid_id]['provenance'] is not None
    set_grid = {'op': 'set_component', 'scene_id': scene, 'entity_id': grid_id, 'component': 'NavigationGrid',
                'value': {'dimensions': [7,5], 'costs': [-1]*35}}
    before_invalid = project.read_bytes()
    assert 'error' in execute([set_grid], 'Reject negative costs')[0]
    assert project.read_bytes() == before_invalid
    # The transaction limit applies across the whole document, not per scene.
    extra_grids = [{'op': 'create_entity', 'scene_id': scene, 'entity': {
        'id': f'{i:026}', 'name': 'Extra grid', 'parent': None, 'provenance': None,
        'components': {'NavigationGrid': {'dimensions': [1,1], 'costs': [1]}}}}
        for i in range(3,11)]
    assert 'error' in execute(extra_grids, 'Reject nine grids')[0]
    assert project.read_bytes() == before_invalid
    # Mutate valid data, then undo and redo in independent RPC processes.
    changed = costs.copy()
    changed[10] = 0
    set_grid['value']['costs'] = changed
    assert 'error' not in execute([set_grid], 'Close upper door')[0]
    for method, expected in [('history.undo', costs), ('history.redo', changed), ('history.undo', costs)]:
        replies = rpc([{'id': 3, 'method': method}, {'id': 4, 'method': 'project.save'}])
        assert all('error' not in reply for reply in replies), replies
        assert json.loads(project.read_text())['scenes'][scene]['entities'][grid_id]['components']['NavigationGrid']['costs'] == expected

    source, compiled = out/'grid.ts', out/'grid.js'
    source.write_text('import type { ScriptApi, NavigationGrid } from '+json.dumps(str(ROOT/'sdk/ts/src/index'))+';\n'
        +'const scene='+json.dumps(scene)+',grid='+json.dumps(grid_id)+',walker='+json.dumps(actor_id)+';\n'+'''
export default defineBehavior<{tick:number,pos:[number,number],blocked:number[],costs:(number|null)[],arrived:boolean}>({
  initialState:{tick:0,pos:[0,2],blocked:[],costs:[],arrived:false},
  update(api:ScriptApi,dt,s){
    s.tick++;
    const path=(start:[number,number])=>api.findGridPath({scene_id:scene,grid_entity:grid,
      path:{start,end:[6,2],diagonal:true,max_expansions:35}});
    const fixed=path([0,2]), route=path(s.pos);
    if(s.tick<=10)s.costs.push(fixed?.cost??null);
    if(route===null)s.blocked.push(s.tick);
    if(s.tick%3===0 && route!==null && route.cells.length>1){
      s.pos=route.cells[1]!;
      api.command({op:'set_component',scene_id:scene,entity_id:walker,component:'Transform',
        value:{translation:[s.pos[0],0,s.pos[1]],rotation:[0,0,0,1],scale:[1,1,1]}});
    }
    s.arrived=s.pos[0]===6 && s.pos[1]===2;
    if([3,5,8].includes(s.tick)){
      const g=api.query('NavigationGrid').find(e=>e.id===grid)!.components.NavigationGrid as unknown as NavigationGrid;
      const costs=[...g.costs];
      if(s.tick===3)costs[10]=0;
      if(s.tick===5)costs[24]=0;
      if(s.tick===8)costs[10]=1;
      api.command({op:'set_component',scene_id:scene,entity_id:grid,component:'NavigationGrid',value:{dimensions:[7,5],costs}});
    }
  }
});
''')
    subprocess.run(['node', str(ROOT/'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
                    '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], cwd=ROOT, check=True)
    subprocess.run(['node', str(ROOT/'tools/build_script.mjs'), str(source), str(compiled)], cwd=ROOT, check=True)
    before = {p.name: digest(p) for p in [project, journal]}
    whole = run('play', project, '--ticks', 120, '--compiled-script', compiled)
    repeat = run('play', project, '--ticks', 120, '--compiled-script', compiled)
    first = run('play', project, '--ticks', 7, '--compiled-script', compiled, '--save-output', out/'blocked.save.json')
    resumed = run('play', project, '--ticks', 113, '--compiled-script', compiled, '--load-save', out/'blocked.save.json')
    state = whole['script_state']
    assert state == repeat['script_state'] == resumed['script_state']
    assert whole['state'] == repeat['state'] == resumed['state']
    assert first['script_state']['blocked'] == [6,7]
    assert state['blocked'] == [6,7,8] and state['arrived'] and state['pos'] == [6,2]
    assert state['costs'] == [6828,6828,6828,9828,9828,None,None,None,6828,6828], state
    assert before == {p.name: digest(p) for p in [project, journal]}
    assert binary_sha == digest(binary)
    result = {'passed': True, 'strict_typescript': True, 'shared_commands_and_undo_redo': True,
        'invalid_grid_rollback': True, 'project_grid_count_limit': True, 'blocked_door_save_resume_exact': True, 'repeat_exact': True,
        'state': state, 'author_files_unchanged': before, 'binary_sha256': binary_sha,
        'rendering_or_physics_gate': False}
    (out/'runs.json').write_text(json.dumps({'whole': whole, 'repeat': repeat, 'first': first, 'resumed': resumed}, indent=2)+'\n')
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
