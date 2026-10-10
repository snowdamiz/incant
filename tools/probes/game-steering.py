#!/usr/bin/env python3
"""Drive 100 agents through the public TypeScript steering API and shared commands."""
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
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(*arguments):
        return json.loads(subprocess.check_output([str(binary), *map(str, arguments)], cwd=ROOT,
                                                 text=True, encoding='utf-8'))

    project, journal = out / 'game.incant.json', out / 'game.incant.journal.jsonl'
    run('init', project, '--name', 'One hundred steering agents', '--entities', 0)
    scene = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    commands, goals = [], {}
    for i in range(100):
        eid = f'{i:026d}'
        x, z = (-4 if i % 2 == 0 else 4), (i // 2) * 2 - 49
        goals[eid] = [-x, z]
        commands.append({'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': eid, 'name': f'Walker {i}', 'parent': None, 'provenance': None,
            'components': {'Transform': {'translation': [x, 0, z], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                           'Velocity': {'linear': [0, 0, 0]}}}})
    request = [{'id': 1, 'method': 'command.execute', 'params': {'commands': commands,
                'expected_revision': '', 'description': 'Create one hundred walkers'}},
               {'id': 2, 'method': 'project.save'}]
    # Read the revision through the public RPC envelope; validation is independent.
    read = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
        input=json.dumps({'id': 0, 'method': 'project.read'})+'\n', cwd=ROOT, text=True, encoding='utf-8')
    request[0]['params']['expected_revision'] = json.loads(read)['result']['revision']
    replies = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
        input=''.join(json.dumps(r)+'\n' for r in request), cwd=ROOT, text=True, encoding='utf-8')
    assert all('error' not in json.loads(line) for line in replies.splitlines()), replies
    source, compiled = out / 'steering.ts', out / 'steering.js'
    source.write_text('import type { ScriptApi, SteeringAgent, Transform } from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+';\n'
        + 'const goals:Record<string,[number,number]>='+json.dumps(goals)+';\n'+'''
export default defineBehavior<{tick:number,minimum:number,remaining:number,neighbors:number}>({
  initialState:{tick:0,minimum:1000,remaining:1000,neighbors:0},
  update(api:ScriptApi,dt,s){
    const entities=api.query('Velocity');
    const agents:SteeringAgent[]=entities.map(e=>{
      const p=(e.components.Transform as Transform).translation;
      const v=(e.components.Velocity as {linear:number[]}).linear;
      const g=goals[e.id]!,dx=g[0]-p[0],dz=g[1]-p[2],d=Math.hypot(dx,dz);
      const speed=Math.min(d/dt,2),factor=d<1e-6?0:speed/d;
      return {id:e.id,position:p,velocity:[v[0]!,v[2]!],preferred_velocity:[dx*factor,dz*factor],
        radius:.25,height:1.8,max_speed:2,responsibility:1};
    });
    for(let i=0;i<agents.length;i++)for(let j=i+1;j<agents.length;j++){
      const a=agents[i]!.position,b=agents[j]!.position;
      s.minimum=Math.min(s.minimum,Math.hypot(a[0]-b[0],a[2]-b[2]));
    }
    if(s.minimum<.498)throw Error('Agent clearance was violated');
    s.remaining=Math.max(...agents.map(a=>Math.hypot(goals[a.id]![0]-a.position[0],goals[a.id]![1]-a.position[2])));
    const velocities=api.steerAgents({agents,time_horizon:2,obstacle_time_horizon:2,margin:.05});
    s.neighbors=Math.max(s.neighbors,...velocities.map(v=>v.neighbors));
    const byId=new Map(entities.map(e=>[e.id,e]));
    for(const v of velocities){const e=byId.get(v.id)!;api.command({op:'set_component',scene_id:e.scene_id,entity_id:v.id,
      component:'Velocity',value:{linear:[v.velocity[0],0,v.velocity[1]]}});}
    s.tick++;
  }
});
''', encoding='utf-8')
    subprocess.run(['node', str(ROOT/'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
        '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], cwd=ROOT, check=True)
    subprocess.run(['node', str(ROOT/'tools/build_script.mjs'), str(source), str(compiled)], cwd=ROOT, check=True)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    whole = run('play', project, '--ticks', 600, '--compiled-script', compiled)
    repeat = run('play', project, '--ticks', 600, '--compiled-script', compiled)
    first = run('play', project, '--ticks', 300, '--compiled-script', compiled, '--save-output', out/'mid.save.json')
    resumed = run('play', project, '--ticks', 300, '--compiled-script', compiled, '--load-save', out/'mid.save.json')
    assert whole['script_state'] == resumed['script_state'] == repeat['script_state']
    assert whole['state'] == resumed['state'] == repeat['state']
    state = whole['script_state']
    assert state['tick'] == 600 and state['remaining'] < .1 and state['minimum'] >= .498 and state['neighbors'] > 0, state
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    assert binary_hash == hashlib.sha256(binary.read_bytes()).hexdigest(), 'binary changed during probe'
    result = {'passed': True, 'agents': 100, 'strict_typescript': True, 'shared_velocity_commands': True,
        'repeat_and_save_resume_exact': True, 'state': state, 'author_files_unchanged': before,
        'binary_sha256': binary_hash, 'rendering_or_physics_collision_gate': False}
    (out/'runs.json').write_text(json.dumps({'whole': whole, 'first': first, 'resumed': resumed, 'repeat': repeat}, indent=2)+'\n', encoding='utf-8')
    (out/'result.json').write_text(json.dumps(result, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
