#!/usr/bin/env python3
"""Public captured weighted-pose Rust guest: development and ordinary native export.

Author/dev preflight precedes cold shipping. Only this new proof's private project,
cache, guest build, trusted tool copies and debug maps are deleted. CPU only.
"""
import argparse
import datetime
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import struct
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
BASE = runpy.run_path(str(ROOT/'tools/probes/native-animation-graph-export.py'))
TICKS = 20
ACTIONS = [('old-v1','old_v1','KeyA'),('old-v2','old_v2','KeyB'),('old-v3','old_v3','KeyC'),
           ('short-scratch','short_scratch','KeyD'),('late-command','late_command','KeyE')]
OLD = 'animation graph blend requires graph snapshot ABI'
V3 = 'animation v3 cannot represent weighted tree poses'
DEV_ERRORS = {name:'Error: native play startup/runtime failure: development Wasm rejected or trapped: Wasm ABI: '+message for name,message in [('old-v1',OLD),('old-v2',OLD),('old-v3',V3)]}
DEV_ERRORS.update({'short-scratch':'Error: native play startup/runtime failure: development Wasm rejected or trapped: wasm trap: wasm `unreachable` instruction executed',
                  'late-command':'Error: native play startup/runtime failure: runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist'})
PACKAGE_ERRORS = {name:'Error: Session(Module(Boundary(Abi("'+message+'"))))' for name,message in [('old-v1',OLD),('old-v2',OLD),('old-v3',V3)]}
PACKAGE_ERRORS.update({'short-scratch':'Error: Session(Module(Trap(1)))',
                      'late-command':'Error: Session(Module(Boundary(Runtime(MissingEntity(StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]))))))'})


def ulid(n):
    alphabet='0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    return ''.join(alphabet[(n>>(5*i))&31] for i in reversed(range(26)))


def fixture(root):
    BASE['fixture'](root)
    model=json.loads((root/'rig.gltf').read_text());data=bytearray((root/'rig.bin').read_bytes())
    originals=model['animations'];model['animations']=[]
    for n in range(8):
        view,accessor=len(model['bufferViews']),len(model['accessors'])
        model['bufferViews'].append({'buffer':0,'byteOffset':len(data),'byteLength':8})
        data.extend(struct.pack('<2f',0,n+1))
        model['accessors'].append({'bufferView':view,'componentType':5126,'type':'SCALAR','count':2,'min':[0],'max':[n+1]})
        animation=json.loads(json.dumps(originals[n%2]));animation['name']=f'clip-{n}'
        animation['samplers'][0]['input']=accessor;model['animations'].append(animation)
    model['buffers'][0]['byteLength']=len(data);(root/'rig.bin').write_bytes(data)
    (root/'rig.gltf').write_text(json.dumps(model,indent=2)+'\n')


def graph(pair=False):
    params={ulid(10+i):{'initial':[.25,.5,.75][i],'min':0,'max':1} for i in range(1 if pair else 3)}
    if pair:
        motion={'kind':'blend1d','parameter':ulid(10),'cycle_seconds':1,'points':[{'threshold':0,'clip':0},{'threshold':1,'clip':1}]}
        states={ulid(20):{'looping':True,'motion':motion}}
    else:
        nodes={ulid(1000+i):{'kind':'clip','clip':i} for i in range(8)}
        for i in range(2):
            nodes[ulid(1008+i)]={'kind':'blend2d','parameters':[ulid(10),ulid(11)],'x':[0,1],'y':[0,1],'children':[ulid(1000+i*4+j) for j in range(4)]}
        nodes[ulid(1010)]={'kind':'blend1d','parameter':ulid(12),'points':[{'threshold':0,'node':ulid(1008)},{'threshold':1,'node':ulid(1009)}]}
        states={ulid(20):{'looping':True,'motion':{'kind':'tree','cycle_seconds':2,'tree':{'root':ulid(1010),'nodes':nodes}}},
                ulid(21):{'looping':False,'motion':{'kind':'tree','cycle_seconds':.04,'tree':{'root':ulid(1100),'nodes':{ulid(1100):{'kind':'clip','clip':None}}}}}}
    return {'initial':ulid(20),'parameters':params,'states':states,'transitions':{}}


def entities(model):
    result={}
    for n in [1,2,3,4,*range(100,148),*range(200,209),300]:
        components={'Transform':BASE['transform']([0,0,8] if n==300 else [0,0,0])}
        if n in (1,2,3):
            components['MeshRenderer']={'mesh':model,'materials':[],'cast_shadows':False}
            components['Animation']={'clip':0 if n==2 else None,'seconds':0,'speed':1,'playing':True,'looping':True}
            if n!=2:components['AnimationGraph']=graph(n==3)
        if n==300:components['Camera']={'fov_degrees':60,'near':.1,'far':200,'projection':{'kind':'perspective'}}
        result[ulid(n)]={'id':ulid(n),'name':f'Pose game {n}','parent':None,'provenance':None,'components':components}
    return result


def fields(row):
    return {**{ulid(100+i):v for i,v in enumerate(row['samples'])},**{ulid(200+i):v for i,v in enumerate(row['metadata'])}}


def observation(rows):
    return {'format':'incant-native-observation','version':1,'entities':list(fields(rows[0])),
            'checks':[{'name':f'{row["tick"]}-{entity}','tick':row['tick'],'expect':{'type':'vector','entity':entity,'property':'translation','value':value,'tolerance':1e-12}}
                      for row in rows if row['tick'] in (1,6,13,20) for entity,value in fields(row).items()]}


def check_logs(logs,expected,require):
    require(logs['dropped']==0 and len(logs['entries'])==TICKS*3,'Full per-entity record count')
    for offset,entry in enumerate(logs['entries']):
        row=expected[offset//3];index=offset%3;actual=json.loads(entry['message'])
        require(entry['level']=='info' and entry['tick']==row['tick']==actual['tick'] and actual['entity']==index+1,'Record tick/level/entity')
        require(set(actual)=={'tick','entity','samples','metadata'} and len(actual['samples'])==16 and len(actual['metadata'])==3,'Complete per-field record layout')
        for kind,size in [('samples',16),('metadata',3)]:
            require(all(len(v)==3 for v in actual[kind]),'Record vector shape')
            wanted=row[kind][index*size:(index+1)*size]
            require(all(type(a) in (int,float) and math.isfinite(a) and abs(a-b)<=1e-12 for arow,brow in zip(actual[kind],wanted) for a,b in zip(arow,brow)),f'Native oracle {kind}@{row["tick"]}/{index+1}')


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
    sources=[Path(__file__),ROOT/'tools/probes/native-wasm-animation-pose-export/behavior.rs',ROOT/'crates/incant_cook/examples/animation_pose_package_oracle.rs',ROOT/'tools/probes/native-animation-graph-export.py',ROOT/'tools/export_native.py']
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
        model=value([binary,'import',project,'rig.gltf']);write_json(out/'model-import.json',model)
        source=build/'behavior.rs';source.write_bytes(sources[1].read_bytes())
        compiled=value([sys.executable,ROOT/'tools/rust_guest.py',source,build/'behavior.wasm','--rustup','/opt/homebrew/bin/rustup']);write_json(out/'guest-compile.json',compiled)
        shutil.copy2(build/'behavior.wasm',inputs/'behavior.wasm');module=value([binary,'import',project,'behavior.wasm']);write_json(out/'module-import.json',module)
        Rpc=runpy.run_path(str(ROOT/'tools/probes/script-source-transactions.py'))['Rpc'];rpc=Rpc(binary,project,journal);calls=[]
        def call(method,params=None):
            result=rpc.call(method,params);calls.append({'method':method,'params':params,'result':result});write_json(out/'public-commands.json',calls);return result
        try:
            before=call('project.read');batch=[{'op':'delete_scene','scene_id':s} for s in before['project']['scenes']]
            batch += [{'op':'create_scene','scene':{'id':ulid(99),'name':'Complete native poses','entities':entities(model['asset']['id'])}},
                      {'op':'set_scene_program','scene_id':ulid(99),'program':{'kind':'wasm','module':module['asset']['id']}},
                      {'op':'set_input_actions','actions':{name:{'kind':'button','bindings':[{'type':'key','code':key}]} for _,name,key in ACTIONS}}]
            call('command.execute',{'expected_revision':before['revision'],'description':'Author complete pose game','commands':batch})
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
        require(rows[0]['metadata'][0][0]==8 and rows[12]['metadata'][1][2]==1,'Eight active samples and completed rest exercised')
        write_json(out/'oracle.json',rows);write_json(plan,observation(rows));require(len(observation(rows)['checks'])==228,'Bounded public vector assertions')
        write_json(positive,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[]})
        dev=value([binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',positive,'--observe',plan,'--pcm-output',out/'development.pcm'])
        require(dev['passed'] and dev['observations']['passed'] and not dev['script_source_parsing'] and dev['cpu_render_input'] and not dev['gpu_submitted'],'Captured-only development path')
        check_logs(dev['logs'],rows,require);write_json(out/'development.json',dev)
        for name,_,key in ACTIONS:
            replay=out/f'{name}-input.json';write_json(replay,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[{'tick':5,'events':[{'type':'key','code':key,'down':True}]}]})
            pcm=out/f'dev-{name}.pcm';refusal('dev-'+name,[binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',replay,'--pcm-output',pcm],pcm,DEV_ERRORS[name])
        write_json(out/'preflight.json',{'passed':True,'sources':source_hashes,'headless_sha256':sha256(binary.read_bytes()),'oracle_binary_sha256':sha256(oracle.read_bytes()),'oracle_scene_sha256':sha256((out/'oracle.scene').read_bytes()),'project_sha256':sha256(project.read_bytes()),'ticks':TICKS,'complete_records':60,'records_per_tick':3,'sample_vectors_per_record':16,'metadata_vectors_per_record':3,'sample_vectors_per_tick':48,'metadata_vectors_per_tick':9,'bounded_vector_checks':228,'source_deleted':True,'journal':str(journal)})
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
    for name,_,_ in ACTIONS:
        pcm=out/f'package-{name}.pcm';refusal('package-'+name,[player,'--package',data,'--ticks',TICKS,'--input-recording',out/f'{name}-input.json','--output',pcm],pcm,PACKAGE_ERRORS[name],cwd=player.parent,env=environment)
    pcm=out/'checkpoint-refused.pcm';save=out/'checkpoint-refused.bin'
    refusal('checkpoint-refused',[player,'--package',data,'--ticks',1,'--output',pcm,'--save-checkpoint',save],pcm,'Error: Checkpoint(Unsupported("static module state ABI"))',cwd=player.parent,env=environment)
    require(not save.exists(),'Unsupported checkpoint published output')
    write_json(out/'result.json',{'passed':True,'target':report['target'],'player_sha256':report['player_sha256'],'program':report['program'],'fresh_ordinary_package_runs':3,'ticks_each':TICKS,'per_tick_records':3,'per_tick_full_sample_vectors':48,'per_tick_full_metadata_vectors':9,'public_vector_checks_per_run':228,'record_scalars_checked_per_run':3420,'source_deleted_before_development_and_export':True,'project_cache_build_maps_deleted_before_runs':True,'public_import_atomic_select_undo_redo_journal_reopen':True,'no_static_module_fixture_flag':True,'refusals':[f'{host}-{name}' for host in ['dev','package'] for name,_,_ in ACTIONS]+['checkpoint-refused'],'refusal_scope':'Exact runtime diagnostic and no stdout/PCM/save publication; not a fresh proof of per-step rollback. Host suite separately tests atomic prefixes.','removed_private_files':removed,'package_hashes':hashes(out/'package'),'proof_source_hashes':source_hashes,'commands_sha256':sha256(command_path.read_bytes()),'limits':['Actual local desktop CPU package; no GPU/rendered/device/performance or phase gate.','Eight active weighted samples; nonzero tree fades/composite operands remain unsupported.','Whole guest checkpoints explicitly unsupported; gameplay counter lives in native ECS, not module global state.','Only newly created proof source/cache/build/maps are removed; repository and shared engine build storage are retained.']})
    print(json.dumps({'passed':True,'output':str(out),'fresh_runs':len(runs),'full_scalar_observations_each':3420},indent=2))


if __name__=='__main__':main()
