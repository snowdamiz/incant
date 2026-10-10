#!/usr/bin/env python3
"""Verify compound authoring/recovery and real scripted physics through public APIs."""
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
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = str(args.binary.resolve())
    project, journal = out/'compound.incant.json', out/'history.jsonl'
    records = []

    def run(*arguments):
        return subprocess.check_output([binary, *map(str, arguments)], text=True, cwd=ROOT)

    @contextmanager
    def rpc():
        proc = subprocess.Popen([binary, 'rpc', str(project), '--journal', str(journal)],
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)
        def call(method, params=None, success=True):
            request = {'id':len(records), 'method':method, 'params':params or {}}
            proc.stdin.write(json.dumps(request)+'\n'); proc.stdin.flush()
            response = json.loads(proc.stdout.readline())
            records.append({'request':json.loads(json.dumps(request)), 'response':response})
            assert ('error' not in response)==success, response
            return response.get('result', response.get('error'))
        try:
            yield call
        finally:
            proc.stdin.close(); assert proc.wait(timeout=15)==0; proc.stdout.close()

    def transform(at):
        return {'translation':at, 'rotation':[0,0,0,1], 'scale':[1,1,1]}
    def part(n, at, shape):
        return {'id':f'{n:026d}', 'translation':at, 'rotation':[0,0,0,1], 'shape':shape}
    def box(half):
        return {'type':'box','half_extents':half}
    def collider(shape):
        return {'shape':shape,'density':1000,'friction':0.5,'restitution':0,'sensor':False,'memberships':4294967295,'filter':4294967295}
    def compound(parts):
        return {'type':'compound','parts':parts}

    run('init',project,'--entities',0,'--name','Compound CLI verification')
    sid=next(iter(json.loads(project.read_text())['scenes']))
    ground,arch,body=[f'{n:026d}' for n in [10,11,12]]
    arch_collider=collider(compound([part(100,[-1,1,0],box([.2,1,.2])),part(101,[1,1,0],box([.2,1,.2])),part(102,[0,2,0],box([1.2,.2,.2]))]))
    body_collider=collider(compound([part(110,[-.7,0,0],{'type':'sphere','radius':.3}),part(111,[.7,0,0],{'type':'sphere','radius':.3}),part(112,[0,0,0],box([.7,.1,.1]))]))
    commands=[]
    for eid,name,components in [
        (ground,'Floor',{'Transform':transform([0,-.5,0]),'Collider':collider(box([10,.5,10]))}),
        (arch,'Arch',{'Transform':transform([0,0,0]),'Collider':arch_collider}),
        (body,'Dumbbell',{'Transform':transform([3,4,0]),'Collider':body_collider,'RigidBody':{'motion':'dynamic','gravity_scale':1,'linear_damping':0,'angular_damping':0,'can_sleep':True,'ccd':True}}),
    ]:
        commands.append({'op':'create_entity','scene_id':sid,'entity':{'id':eid,'name':name,'parent':None,'components':components,'provenance':None}})
    def set_collider(value):
        return {'op':'set_component','scene_id':sid,'entity_id':arch,'component':'Collider','value':value}
    with rpc() as call:
        call('command.execute',{'commands':commands,'expected_revision':0,'description':'Create compound scene'})
        original=call('project.read')
        invalid=json.loads(json.dumps(arch_collider))
        invalid['shape']['parts'][1]['id']=invalid['shape']['parts'][0]['id']
        call('command.execute',{'commands':[set_collider(invalid)],'expected_revision':1},success=False)
        assert call('project.read')==original
        arch_collider['shape']['parts'][1]['translation'][0]=1.4
        call('command.execute',{'commands':[set_collider(arch_collider)],'expected_revision':1,'description':'Move stable right leg'})
        edited=call('project.read')['project']
        assert call('history.undo')['project']==original['project']
        assert call('history.redo')['project']==edited
        call('project.save')
    with rpc() as call:
        assert call('project.read')['project']==edited
        assert len(call('history.read'))==2
    baseline={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    source=out/'compound.ts'
    source.write_text('import type { ScriptApi } from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+''';
export default defineBehavior<{tick:number;hole:boolean;leg:boolean;height:number}>({
  initialState:{tick:0,hole:false,leg:false,height:0},
  update(api:ScriptApi,dt,state){
    state.tick++;
    const arch=api.query('Collider').find(e=>e.name==='Arch')!;
    const body=api.query('RigidBody')[0]!;
    const cast=(x:number)=>api.raycast({scene_id:arch.scene_id,origin:[x,1,3],direction:[0,0,-1],max_distance:6,
      include_sensors:false,exclude_entity:body.id,memberships:4294967295,filter:4294967295});
    state.hole=cast(0)===null;
    state.leg=cast(-1)?.entity_id===arch.id;
    state.height=(body.components.Transform as {translation:number[]}).translation[1]!;
    if(!state.hole||!state.leg)throw new Error('Compound opening or stable hit identity failed');
  }
});
''')
    compiled=out/'compound.js'
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit','--target','ES2022','--module','ESNext','--moduleResolution','bundler',str(source)],check=True,cwd=ROOT)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],check=True,cwd=ROOT)
    first=json.loads(run('play',project,'--ticks',360,'--compiled-script',compiled))
    second=json.loads(run('play',project,'--ticks',360,'--compiled-script',compiled))
    assert first['completed'] and first['state']==second['state'] and first['script_state']==second['script_state']
    assert first['script_state']['hole'] and first['script_state']['leg'] and abs(first['script_state']['height']-.3)<.02
    assert baseline=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    (out/'rpc.json').write_text(json.dumps(records,indent=2)+'\n')
    (out/'play.json').write_text(json.dumps(first,indent=2)+'\n')
    result={'passed':True,'atomic_duplicate_rejection':True,'stable_part_edit_undo_redo_reopen':True,
            'strict_typescript':True,'hollow_arch_query':True,'parent_hit_identity':True,
            'dynamic_compound_height':first['script_state']['height'],'repeated_final_state_exact':True,
            'author_files_unchanged':baseline,'physical_device_gate':False}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))


if __name__=='__main__':
    main()
