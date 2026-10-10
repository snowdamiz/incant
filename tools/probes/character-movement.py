#!/usr/bin/env python3
"""Exercise character sweeps and jumping through public CLI/RPC and real TypeScript."""
import argparse
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
    project, journal = out / 'game.incant.json', out / 'history.jsonl'

    def run(*arguments):
        return subprocess.check_output([binary, *map(str, arguments)], text=True, cwd=ROOT)

    run('init', project, '--entities', 0, '--name', 'Character CLI verification')
    sid = next(iter(json.loads(project.read_text())['scenes']))
    player = '00000000000000000000000011'
    def transform(at):
        return {'translation': at, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
    def collider(shape):
        return {'shape': shape, 'density': 1000, 'friction': 0.5, 'restitution': 0,
                'sensor': False, 'memberships': 4294967295, 'filter': 4294967295}
    commands = []
    for eid, name, components in [
        ('00000000000000000000000010', 'Floor', {'Transform': transform([0,-0.5,0]), 'Collider': collider({'type':'box','half_extents':[10,0.5,10]})}),
        (player, 'Player', {'Transform': transform([0,0.91,0]), 'Collider': collider({'type':'capsule','half_height':0.6,'radius':0.3}),
            'RigidBody': {'motion':'kinematic','gravity_scale':1,'linear_damping':0,'angular_damping':0,'can_sleep':True,'ccd':True}}),
        ('00000000000000000000000012', 'Wall', {'Transform': transform([2,2,0]), 'Collider': collider({'type':'box','half_extents':[0.1,2,5]})}),
    ]:
        commands.append({'op':'create_entity','scene_id':sid,'entity':{'id':eid,'name':name,'parent':None,'components':components,'provenance':None}})
    requests = [
        {'id':1,'method':'command.execute','params':{'commands':commands,'expected_revision':0,'description':'Create character test scene'}},
        {'id':2,'method':'project.save','params':{}},
    ]
    result = subprocess.run([binary,'rpc',str(project),'--journal',str(journal)], input=''.join(json.dumps(x)+'\n' for x in requests),
                            text=True,capture_output=True,check=True,cwd=ROOT)
    responses = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(responses)==2 and all('result' in item for item in responses), responses
    baseline = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    source = out/'walk.ts'
    source.write_text('import type { ScriptApi } from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+''';
export default defineBehavior<{tick:number;vertical:number;grounded:boolean;maxY:number;jumped:boolean;landed:boolean}>({
  initialState: { tick:0, vertical:0, grounded:false, maxY:0, jumped:false, landed:false },
  update(api: ScriptApi, dt, state) {
    state.tick++;
    const player=api.query('RigidBody')[0]!;
    const transform=player.components.Transform as { translation:number[] };
    state.maxY=Math.max(state.maxY,transform.translation[1]!);
    state.vertical-=9.81*dt;
    if(state.tick===181 && state.grounded){state.vertical=5;state.jumped=true;}
    const movement=api.computeCharacterMotion({scene_id:player.scene_id,entity_id:player.id,
      translation:[(state.tick<=180?2:-1)*dt,state.vertical*dt,0]});
    state.grounded=movement.grounded;
    if(state.grounded && state.vertical<0){state.vertical=0;if(state.jumped)state.landed=true;}
    api.command({op:'set_component',scene_id:player.scene_id,entity_id:player.id,
      component:'Velocity',value:{linear:movement.translation.map(v=>v/dt)}});
    if(state.tick===360)api.log('walk, wall contact, jump and landing complete');
  }
});
''')
    compiled = out/'walk.js'
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit','--target','ES2022','--module','ESNext','--moduleResolution','bundler',str(source)],check=True,cwd=ROOT)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],check=True,cwd=ROOT)
    def play(ticks):
        return json.loads(run('play',project,'--ticks',ticks,'--compiled-script',compiled))
    wall = play(180)
    pos = wall['state']['entities'][player]['translation']
    assert 1.55<pos[0]<1.61 and 0.9<pos[1]<0.93, pos
    assert wall['script_state']['grounded']
    first, second = play(360), play(360)
    assert first['state']==second['state'] and first['script_state']==second['script_state']
    state=first['script_state']
    assert state['jumped'] and state['landed'] and state['grounded'] and state['maxY']>2, state
    assert first['state']['entities'][player]['translation'][0]<-1
    assert first['script_commands']==360 and len(first['logs'])==1
    assert baseline=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    (out/'rpc.json').write_text(json.dumps(responses,indent=2)+'\n')
    (out/'wall.json').write_text(json.dumps(wall,indent=2)+'\n')
    (out/'play.json').write_text(json.dumps(first,indent=2)+'\n')
    result={'passed':True,'public_rpc':True,'strict_typescript':True,'wall_position':pos,
        'script_state':state,'repeated_final_state_exact':True,'author_files_unchanged':baseline,
        'physical_device_gate':False}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))


if __name__=='__main__':
    main()
