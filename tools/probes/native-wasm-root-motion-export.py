#!/usr/bin/env python3
"""Actual root-motion gameplay through public authoring and ordinary native export.

Complete per-tick native-oracle operands/clocks/IK, exact development/static reports,
private source/cache/build/map deletion and typed no-output refusals. CPU only.
"""
import argparse
import datetime
import json
import math
import struct
import os
from pathlib import Path
import runpy
import shutil
import sys
import tempfile
import time
ROOT=Path(__file__).resolve().parents[2]
POSE=runpy.run_path(str(ROOT/'tools/probes/native-wasm-animation-pose-export.py'))
BASE=POSE['BASE'];ulid=POSE['ulid'];TICKS=32
COMPOUND=runpy.run_path(str(ROOT/'tools/probes/native-wasm-compound-export.py'))
ACTIONS=[(name,name.replace('-','_'),'Key'+chr(65+i)) for i,name in enumerate(['old-v5','old-ik-v2','short-root','late-command','bad-root'])]
OLD='animation snapshots cannot represent root stripping'
DEV_ERRORS={n:'Error: native play startup/runtime failure: development Wasm rejected or trapped: Wasm ABI: '+OLD for n,_,_ in ACTIONS[:2]}
PACKAGE_ERRORS={n:'Error: Session(Module(Boundary(Abi("'+OLD+'"))))' for n,_,_ in ACTIONS[:2]}
DEV_ERRORS['short-root']='Error: native play startup/runtime failure: development Wasm rejected or trapped: wasm trap: wasm `unreachable` instruction executed'
PACKAGE_ERRORS['short-root']='Error: Session(Module(Trap(1)))'
DEV_ERRORS['late-command']='Error: native play startup/runtime failure: runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist'
PACKAGE_ERRORS['late-command']='Error: Session(Module(Boundary(Runtime(MissingEntity(StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]))))))'
DEV_ERRORS['bad-root']='Error: native play startup/runtime failure: invalid scene data: animation control requires final animation'
PACKAGE_ERRORS['bad-root']='Error: Session(Module(Boundary(Runtime(Scene(Invalid("animation control requires final animation"))))))'

def fixture(path,cubic=False):
    POSE['fixture'](path,True)
    model=json.loads((path/'rig.gltf').read_text());data=bytearray((path/'rig.bin').read_bytes())
    def add(values,kind,count):
        while len(data)%4:data.append(0)
        view=len(model['bufferViews']);accessor=len(model['accessors']);payload=struct.pack('<'+'f'*len(values),*values)
        model['bufferViews'].append({'buffer':0,'byteOffset':len(data),'byteLength':len(payload)});data.extend(payload)
        model['accessors'].append({'bufferView':view,'componentType':5126,'type':kind,'count':count});return accessor
    model['animations']=[]
    for i in range(8):
        duration=(i+1)/2 if i else .125;times=add([0,duration],'SCALAR',2);model['accessors'][times].update(min=[0],max=[duration])
        mode='CUBICSPLINE' if cubic and i==2 else ['LINEAR','STEP'][i%2];end=[(i+1)/8,0,(i%2)/16]
        translation=add(([0]*9+[0]*3+end+[0]*3) if mode=='CUBICSPLINE' else [0,0,0]+end,'VEC3',6 if mode=='CUBICSPLINE' else 2)
        angle=(i+1)/64;sign=-1 if i%2 else 1
        rotation=add([0,0,0,1,0,sign*math.sin(angle),0,sign*math.cos(angle)],'VEC4',2)
        model['animations'].append({'name':f'root-{i}','samplers':[{'input':times,'output':translation,'interpolation':mode},{'input':times,'output':rotation,'interpolation':'LINEAR'}],'channels':[{'sampler':0,'target':{'node':0,'path':'translation'}},{'sampler':1,'target':{'node':0,'path':'rotation'}}]})
    model['buffers'][0]['byteLength']=len(data);(path/'rig.bin').write_bytes(data);(path/'rig.gltf').write_text(json.dumps(model,indent=2)+'\n')

def entities(asset):
    result=COMPOUND['entities'](asset['id'])
    extra=json.loads(json.dumps(result[ulid(2)]));extra.update(id=ulid(5),name='Kinematic root owner');extra['components'].pop('AnimationIk',None);extra['components']['Animation']['speed']=-16;result[ulid(5)]=extra
    for n,policy,at in [(1,'transform',[0,0,0]),(2,'extract_only',[0,0,0]),(3,'character3d',[0,0,-3]),(5,'kinematic3d',[0,0,3])]:
        c=result[ulid(n)]['components'];c['Transform']=BASE['transform'](at)
        c['AnimationRootMotion']={'model':asset['id'],'fingerprint':asset['sha256'],'root':0,'application':policy,'applying':n!=2}
        if n in (3,5):
            c['Velocity']={'linear':[0,0,0]};c['RigidBody']={'motion':'kinematic','gravity_scale':0,'linear_damping':0,'angular_damping':0,'can_sleep':False,'ccd':True}
            c['Collider']={'shape':{'type':'sphere','radius':.1},'density':1000,'friction':0,'restitution':0,'sensor':False,'memberships':4294967295,'filter':4294967295}
    result[ulid(6)]={'id':ulid(6),'name':'Character blocking wall','parent':None,'provenance':None,'components':{'Transform':BASE['transform']([.18,0,-3]),'Collider':{'shape':{'type':'box','half_extents':[.04,2,1]},'density':1000,'friction':0,'restitution':0,'sensor':False,'memberships':4294967295,'filter':4294967295}}}
    for n in [*range(400,486),*range(500,520)]:result[ulid(n)]={'id':ulid(n),'name':f'Root observation {n}','parent':None,'provenance':None,'components':{'Transform':BASE['transform']([0,0,0])}}
    return result

def observation(rows):
    selected=[*range(100,164),*range(400,444),*range(500,520)]
    def offset(n):return (n-100)*3 if n<400 else 384+(n-400)*3 if n<500 else 642+(n-500)*3
    return {'format':'incant-native-observation','version':1,'entities':[ulid(n) for n in [1,2,3,5,*selected]],'checks':[{'name':f'{row["tick"]}-{n}','tick':row['tick'],'expect':{'type':'vector','entity':ulid(n),'property':'translation','value':row['values'][offset(n):offset(n)+3],'tolerance':1e-12}} for row in rows if row['tick'] in (14,TICKS) for n in selected]}

def check_logs(logs,rows,require):
    require(logs['dropped']==0 and len(logs['entries'])==TICKS*26,'Complete per-field record history')
    for i,entry in enumerate(logs['entries']):
        row=rows[i//26];part=i%26;actual=json.loads(entry['message'])
        require(set(actual)=={'tick','part','values'} and entry['level']=='info' and entry['tick']==actual['tick']==row['tick'] and actual['part']==part,'Field record identity')
        require(len(actual['values'])==27 and all(type(a) in (int,float) and math.isfinite(a) and abs(a-b)<=1e-12 for a,b in zip(actual['values'],row['values'][part*27:part*27+27])),'Every pose/IK/root identity, exact clock word, delta and body scalar against typed native oracle')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path);parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--oracle',type=Path,required=True);parser.add_argument('--stage',choices=('author','export','all'),default='all')
    args=parser.parse_args();sys.path.insert(0,str(ROOT/'tools'))
    import native_process
    import export_native
    from native_package import require,sha256,write_json
    out=args.output.resolve();binary=args.binary.resolve(strict=True);oracle=args.oracle.resolve(strict=True)
    if args.stage!='export':out.mkdir(parents=True,exist_ok=False)
    require(out.is_dir() and not out.is_symlink(),'Missing private proof directory')
    actions=ACTIONS;dev_errors=DEV_ERRORS;package_errors=PACKAGE_ERRORS
    sources=[Path(__file__),ROOT/'tools/probes/native-wasm-root-motion-export/behavior.rs',ROOT/'crates/incant_cook/examples/root_motion_package_oracle.rs',ROOT/'crates/incant_wasm_runtime/tests/support/animation_compound.rs',ROOT/'crates/incant_wasm_runtime/tests/support/root_motion.rs',ROOT/'tools/probes/native-wasm-compound-export.py',ROOT/'tools/probes/native-wasm-animation-pose-export.py',ROOT/'tools/probes/native-animation-graph-export.py',ROOT/'tools/export_native.py']
    source_hashes={str(p.relative_to(ROOT)):sha256(p.read_bytes()) for p in sources}
    command_path=out/'commands.json';commands=json.loads(command_path.read_text()) if command_path.exists() else []
    run=native_process.run
    def recorded(argv,**kwargs):
        index=len(commands);record={'argv':list(map(str,argv)),'at':datetime.datetime.now(datetime.timezone.utc).isoformat()};commands.append(record)
        start=time.monotonic();value=run(list(map(str,argv)),**kwargs)
        record.update(exit_code=value.returncode,seconds=time.monotonic()-start)
        for stream in ('stdout','stderr'):
            payload=getattr(value,stream)
            if payload is None and hasattr(kwargs.get(stream),'name'):
                kwargs[stream].flush();payload=Path(kwargs[stream].name).read_bytes()
            if isinstance(payload,(str,bytes)):
                path=out/f'command-{index:03}-{stream}.txt';path.write_bytes(payload.encode() if isinstance(payload,str) else payload)
                record[stream]={'file':path.name,'sha256':sha256(path.read_bytes())}
        write_json(command_path,commands);return value
    native_process.run=recorded
    def execute(argv,**kwargs):
        value=recorded(argv,text=True,encoding='utf-8',**kwargs);require(value.returncode==0,value.stderr[-8000:]);return value
    def value(argv,**kwargs):return json.loads(execute(argv,**kwargs).stdout)
    def hashes(path):return {str(p.relative_to(out)):sha256(p.read_bytes()) for p in sorted(path.rglob('*')) if p.is_file()}
    removed_path=out/'removed.json';removed=json.loads(removed_path.read_text()) if removed_path.exists() else {}
    def remove_file(path):
        require(path.is_relative_to(out) and path.is_file() and not path.is_symlink(),'Unsafe proof file cleanup')
        removed[str(path.relative_to(out))]=sha256(path.read_bytes());path.unlink();write_json(removed_path,removed)
    def remove_tree(path):
        require(path.is_relative_to(out) and path.is_dir() and not path.is_symlink(),'Unsafe proof tree cleanup')
        removed.update(hashes(path));shutil.rmtree(path);write_json(removed_path,removed)
    def refusal(name,argv,output,expected,**kwargs):
        p=recorded(argv,text=True,encoding='utf-8',**kwargs)
        receipt={'command':list(map(str,argv)),'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'output_absent':not output.exists(),'expected_stderr':expected}
        write_json(out/f'{name}.json',receipt)
        require(p.returncode==1 and not output.exists() and p.stderr.strip()==expected and not p.stdout,f'{name} unexpected boundary: {p.returncode}: {p.stderr}')
        return receipt
    inputs=out/'authoring';project=inputs/'project.json';journal=project.with_suffix('.journal.jsonl');plan=out/'observe.json';positive=out/'positive-input.json'
    if args.stage!='export':
        inputs.mkdir();build=out/'guest-build';build.mkdir();execute([binary,'init',project,'--entities',0]);fixture(inputs)
        cubic=inputs/'cubic';cubic.mkdir();fixture(cubic,True)
        before_cubic=sha256(project.read_bytes());attempt=recorded([binary,'import',project,'cubic/rig.gltf'],text=True,encoding='utf-8')
        expected='Error: could not cook cubic/rig.gltf: unsupported asset feature: animation output count/interpolation'
        require(attempt.returncode==1 and not attempt.stdout and attempt.stderr.strip()==expected and sha256(project.read_bytes())==before_cubic,'CUBICSPLINE typed import refusal preserves project')
        write_json(out/'cubic-import-refused.json',{'exit_code':attempt.returncode,'stdout':attempt.stdout,'stderr':attempt.stderr,'expected':expected,'project_unchanged':True})
        remove_tree(cubic)
        model=value([binary,'import',project,'rig.gltf']);write_json(out/'model-import.json',model)
        source=build/'behavior.rs';source.write_bytes(sources[1].read_bytes())
        compiled=value([sys.executable,ROOT/'tools/rust_guest.py',source,build/'behavior.wasm','--rustup','/opt/homebrew/bin/rustup']);write_json(out/'guest-compile.json',compiled)
        shutil.copy2(build/'behavior.wasm',inputs/'behavior.wasm');module=value([binary,'import',project,'behavior.wasm']);write_json(out/'module-import.json',module)
        Rpc=runpy.run_path(str(ROOT/'tools/probes/script-source-transactions.py'))['Rpc'];rpc=Rpc(binary,project,journal);calls=[]
        def call(method,params=None):
            result=rpc.call(method,params);calls.append({'method':method,'params':params,'result':result});write_json(out/'public-commands.json',calls);return result
        try:
            before=call('project.read');batch=[{'op':'delete_scene','scene_id':s} for s in before['project']['scenes']]
            batch += [{'op':'create_scene','scene':{'id':ulid(99),'name':'Complete native root motion','entities':entities(model['asset'])}},
                      {'op':'set_scene_program','scene_id':ulid(99),'program':{'kind':'wasm','module':module['asset']['id']}},
                      {'op':'set_input_actions','actions':{name:{'kind':'button','bindings':[{'type':'key','code':key}]} for _,name,key in actions}}]
            call('command.execute',{'expected_revision':before['revision'],'description':'Author complete root-motion game','commands':batch})
            selected=call('project.read')['project'];call('history.undo');require(call('project.read')['project']==before['project'],'Atomic Undo')
            call('history.redo');require(call('project.read')['project']==selected,'Atomic Redo')
            call('project.save');saved=call('project.read')
        finally:rpc.close()
        rpc=Rpc(binary,project,journal)
        try:require(call('project.read')==saved,'Editor-default journal path reopened through public RPC with --journal')
        finally:rpc.close()
        execute([binary,'validate',project])
        for name in ('rig.gltf','rig.bin','behavior.wasm'):remove_file(inputs/name)
        remove_tree(build)
        raw=execute([oracle,project,ulid(99),out/'oracle.scene',TICKS]).stdout;rows=[json.loads(line) for line in raw.splitlines()]
        require(len(rows)==TICKS and [r['tick'] for r in rows]==list(range(1,TICKS+1)),'Native oracle clock')
        require(any(row['max_samples']==16 for row in rows) and rows[13]['compound'],'Full compound and refusal boundary exercised')
        require(any(row['reverse'] for row in rows) and any(row['wrapped'] for row in rows) and any(row['collision_limited'] for row in rows),'Reverse/wrap/collision-limited root policies observed')
        write_json(out/'oracle.json',rows);write_json(plan,observation(rows));require(len(observation(rows)['checks'])==256,'Bounded public vector assertions')
        write_json(positive,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[]})
        dev=value([binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',positive,'--observe',plan,'--pcm-output',out/'development.pcm'])
        require(dev['passed'] and dev['observations']['passed'] and not dev['script_source_parsing'] and dev['cpu_render_input'] and not dev['gpu_submitted'],'Captured-only development path')
        check_logs(dev['logs'],rows,require);write_json(out/'development.json',dev)
        for name,_,key in actions:
            replay=out/f'{name}-input.json';write_json(replay,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[{'tick':14,'events':[{'type':'key','code':key,'down':True}]}]})
            pcm=out/f'dev-{name}.pcm';refusal('dev-'+name,[binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',replay,'--pcm-output',pcm],pcm,dev_errors[name])
        write_json(out/'preflight.json',{'passed':True,'sources':source_hashes,'headless_sha256':sha256(binary.read_bytes()),'oracle_binary_sha256':sha256(oracle.read_bytes()),'oracle_scene_sha256':sha256((out/'oracle.scene').read_bytes()),'project_sha256':sha256(project.read_bytes()),'ticks':TICKS,'complete_records':TICKS*26,'record_scalars':27,'bounded_vector_checks':256,'source_deleted':True,'journal':str(journal)})
        print('Public authoring/Undo/reopen and source-deleted development/native oracle/refusals passed.',flush=True)
        if args.stage=='author':return
    pre=json.loads((out/'preflight.json').read_text());require(pre['passed'] and pre['sources']==source_hashes,'Frozen preflight inputs changed')
    require(pre['headless_sha256']==sha256(binary.read_bytes()) and pre['oracle_binary_sha256']==sha256(oracle.read_bytes()),'Preflight binaries changed')
    require(pre['project_sha256']==sha256(project.read_bytes()),'Authored project changed')
    rows=json.loads((out/'oracle.json').read_text());dev=json.loads((out/'development.json').read_text())
    trusted=out/'trusted-tools';trusted.mkdir();scratch=out/'private-builds';scratch.mkdir();tempfile.tempdir=str(scratch)
    provision=runpy.run_path(str(ROOT/'tools/probes/shipping-wasm2c.py'))['provision'];wabt,provenance=provision(trusted);write_json(out/'wabt.json',provenance)
    print('Starting fresh ordinary wasm2c export from captured-only project.',flush=True)
    report=export_native.export(project,out/'package',camera=ulid(300),wabt=wabt)
    require(report['program']['kind']=='wasm' and report['build_storage_removed_before_startup'],'Ordinary compiled scene module selection')
    player=out/'package/player'/('incant_game_player.exe' if os.name=='nt' else 'incant_game_player');data=player.parent/'data'
    require(sha256((data/'world.scene').read_bytes())==pre['oracle_scene_sha256'],'Preflight/export cooked scene identity')
    remove_tree(inputs);remove_tree(trusted)
    for path in (out/'package/symbols').glob('*.map'):remove_file(path)
    require(not any(scratch.iterdir()),'Exporter private builds survived');scratch.rmdir()
    environment={'PATH':str(player.parent),'TMPDIR':str(out),'TEMP':str(out),'TMP':str(out)}
    environment.update({k:os.environ[k] for k in ('SystemRoot','SystemDrive','WINDIR') if k in os.environ})
    runs=[]
    for i in range(3):
        pcm=out/f'run-{i}.pcm';r=value([player,'--package',data,'--ticks',TICKS,'--input-recording',positive,'--observe',plan,'--output',pcm],cwd=player.parent,env=environment)
        require(r['observations']==dev['observations'] and r['logs']==dev['logs'] and r['tick']==dev['tick'] and r['frames']==dev['frames'] and r['pcm_sha256']==dev['pcm_sha256'],'Exact development/package observable parity')
        check_logs(r['logs'],rows,require);write_json(out/f'run-{i}.json',r);runs.append(r)
    for name,_,_ in actions:
        pcm=out/f'package-{name}.pcm';refusal('package-'+name,[player,'--package',data,'--ticks',TICKS,'--input-recording',out/f'{name}-input.json','--output',pcm],pcm,package_errors[name],cwd=player.parent,env=environment)
    pcm=out/'checkpoint-refused.pcm';save=out/'checkpoint-refused.bin'
    refusal('checkpoint-refused',[player,'--package',data,'--ticks',1,'--output',pcm,'--save-checkpoint',save],pcm,'Error: Checkpoint(Unsupported("static module state ABI"))',cwd=player.parent,env=environment)
    require(not save.exists(),'Unsupported checkpoint published output')
    write_json(out/'result.json',{'passed':True,'target':report['target'],'player_sha256':report['player_sha256'],'program':report['program'],'fresh_ordinary_package_runs':3,'ticks_each':TICKS,'per_tick_records':26,'record_scalars_checked_per_run':TICKS*702,'public_vector_checks_per_run':256,'source_deleted_before_development_and_export':True,'project_cache_build_maps_deleted_before_runs':True,'public_import_atomic_select_undo_redo_journal_reopen':True,'no_static_module_fixture_flag':True,'refusals':[f'{host}-{name}' for host in ['dev','package'] for name,_,_ in actions]+['checkpoint-refused'],'refusal_scope':'Exact diagnostic, no stdout/PCM/save; full staged rollback is covered by actual-host conformance separately.','removed_private_files':removed,'package_hashes':hashes(out/'package'),'proof_source_hashes':source_hashes,'commands_sha256':sha256(command_path.read_bytes()),'limits':['Local desktop CPU package only; no GPU/rendered/device/performance or phase gate.','Four native root policies, exact clock words, LINEAR/STEP tracks, compound interpolation and ordered IK; CUBICSPLINE import explicitly refuses.','Whole guest checkpoints explicitly unsupported.','Only this new proof source/cache/build/maps removed; shared engine build storage and repository retained.']})
    print(json.dumps({'passed':True,'output':str(out),'fresh_runs':len(runs),'full_scalar_observations_each':TICKS*702},indent=2))

if __name__=='__main__':main()
