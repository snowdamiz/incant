#!/usr/bin/env python3
"""Public captured Rust audio gameplay, actual mixer oracle and ordinary wasm2c package.

CPU only. Copied records are desired declarations, not mixer cursor observations.
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
import wave
ROOT = Path(__file__).resolve().parents[2]
TICKS = 96
ACTIONS = [(n, n.replace('-', '_'), 'Key'+chr(65+i)) for i,n in enumerate(
    ['short-snapshot','short-command','bad-gain','late-owner','bad-role','seek-clip'])]
DEV_ERRORS = {n: 'Error: native play startup/runtime failure: development Wasm rejected or trapped: wasm trap: wasm `unreachable` instruction executed' for n,_,_ in ACTIONS[:3]}
PACKAGE_ERRORS = {n: 'Error: Session(Module(Trap(1)))' for n,_,_ in ACTIONS[:3]}
DEV_ERRORS['late-owner'] = 'Error: native play startup/runtime failure: runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist'
PACKAGE_ERRORS['late-owner'] = 'Error: Session(Module(Boundary(Runtime(MissingEntity(StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]))))))'
DEV_ERRORS['bad-role'] = 'Error: native play startup/runtime failure: invalid scene data: audio control requires final source'
PACKAGE_ERRORS['bad-role'] = 'Error: Session(Module(Boundary(Runtime(Scene(Invalid("audio control requires final source"))))))'
DEV_ERRORS['seek-clip'] = 'Error: native play startup/runtime failure: invalid audio setting: pan, rate or start time outside supported range'
PACKAGE_ERRORS['seek-clip'] = 'Error: Audio(Invalid("pan, rate or start time outside supported range"))'

def ulid(n):
    alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    return ''.join(alphabet[(n >> (5*i)) & 31] for i in reversed(range(26)))

def transform(at=(0,0,0)):
    return {'translation':list(at),'rotation':[0,0,0,1],'scale':[1,1,1]}

def entities(clips):
    result = {ulid(n): {'id':ulid(n),'name':f'Audio numeric owner {n}','parent':None,'provenance':None,
        'components':{'Transform':transform(),'Velocity':{'linear':[0,0,0]}}} for n in [*range(1,7),*range(100,162)]}
    result[ulid(1)]['components']['AudioBus'] = {'parent':ulid(4),'gain_db':-3}
    result[ulid(4)]['components']['AudioBus'] = {'parent':None,'gain_db':-1}
    result[ulid(5)]['components']['AudioListener'] = {}
    for index,n in enumerate((2,3)):
        result[ulid(n)]['components']['AudioSource'] = {'clip':clips[index], 'bus':ulid(1), 'gain_db':-4,
            'pan':0 if index else .25,'rate':.87 if index else 1.13,'start_seconds':.00037,
            'playing':True,'looping':bool(index),'streaming':bool(index),
            'spatial':{'min_distance':1,'max_distance':30} if index else None}
    result[ulid(300)] = {'id':ulid(300),'name':'CPU input camera','parent':None,'provenance':None,
        'components':{'Transform':transform((0,0,10)),'Camera':{'fov_degrees':60,'near':.1,'far':100,'projection':{'kind':'perspective'}}}}
    return result

def observation(rows):
    return {'format':'incant-native-observation','version':1,'entities':[ulid(n) for n in range(100,162)],
        'checks':[{'name':f'{r["tick"]}-{n}','tick':r['tick'],'expect':{'type':'vector','entity':ulid(n),
            'property':'translation','value':r['values'][(n-100)*3:(n-100)*3+3],'tolerance':1e-12}}
            for r in rows if r['tick'] in (2,10,24,TICKS) for n in range(100,162)]}

def check_logs(logs, rows, require):
    require(logs['dropped']==0 and len(logs['entries'])==TICKS*6,'Complete per-field record history')
    for i,entry in enumerate(logs['entries']):
        row=rows[i//6];part=i%6;actual=json.loads(entry['message'])
        require(set(actual)=={'tick','part','values'} and entry['level']=='info' and
                entry['tick']==actual['tick']==row['tick'] and actual['part']==part,'Field record identity')
        expected=row['values'][part*31:part*31+31]
        require(len(actual['values'])==31 and all(type(a) in (int,float) and math.isfinite(a) and a==b
                for a,b in zip(actual['values'],expected)), 'Every declaration/identity/transport-epoch word exactly matches native oracle')

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
    sources=[Path(__file__),ROOT/'tools/probes/native-wasm-audio-export/behavior.rs',ROOT/'crates/incant_cook/examples/audio_package_oracle.rs',
             ROOT/'crates/incant_wasm_runtime/tests/support/audio_transport.rs',ROOT/'tools/export_native.py',ROOT/'tools/rust_guest.py',ROOT/'tools/probes/script-source-transactions.py']
    source_hashes={str(p.relative_to(ROOT)):sha256(p.read_bytes()) for p in sources}
    command_path=out/'commands.json';commands=json.loads(command_path.read_text()) if command_path.exists() else []
    run=native_process.run
    def recorded(argv,**kwargs):
        index=len(commands);record={'argv':list(map(str,argv)),'at':datetime.datetime.now(datetime.timezone.utc).isoformat()};commands.append(record)
        start=time.monotonic();value=run(list(map(str,argv)),**kwargs);record.update(exit_code=value.returncode,seconds=time.monotonic()-start)
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
        r=recorded(argv,text=True,encoding='utf-8',**kwargs);require(r.returncode==0,r.stderr[-8000:]);return r
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
        r=recorded(argv,text=True,encoding='utf-8',**kwargs)
        write_json(out/f'{name}.json',{'command':list(map(str,argv)),'exit_code':r.returncode,'stdout':r.stdout,
            'stderr':r.stderr,'output_absent':not output.exists(),'expected_stderr':expected})
        require(r.returncode==1 and not output.exists() and r.stderr.strip()==expected and not r.stdout,
                f'{name} unexpected boundary: {r.returncode}: {r.stderr}')
    inputs=out/'authoring';project=inputs/'project.json';journal=project.with_suffix('.journal.jsonl')
    plan=out/'observe.json';positive=out/'positive-input.json'
    if args.stage!='export':
        inputs.mkdir();build=out/'guest-build';build.mkdir();execute([binary,'init',project,'--entities',0])
        clips=[]
        for name,rate in [('static',22050),('stream',32000)]:
            path=inputs/(name+'.wav')
            with wave.open(str(path),'wb') as audio:
                audio.setnchannels(2);audio.setsampwidth(2);audio.setframerate(rate)
                audio.writeframes(b''.join(struct.pack('<hh',int(7000*math.sin(i*.13)),int(6000*math.cos(i*.17))) for i in range(2309)))
            imported=value([binary,'import',project,path.name]);clips.append(imported['asset']['id']);write_json(out/f'{name}-import.json',imported)
        source=build/'behavior.rs';source.write_bytes(sources[1].read_bytes())
        compiled=value([sys.executable,ROOT/'tools/rust_guest.py',source,build/'behavior.wasm','--rustup','/opt/homebrew/bin/rustup']);write_json(out/'guest-compile.json',compiled)
        shutil.copy2(build/'behavior.wasm',inputs/'behavior.wasm');module=value([binary,'import',project,'behavior.wasm']);write_json(out/'module-import.json',module)
        Rpc=runpy.run_path(str(ROOT/'tools/probes/script-source-transactions.py'))['Rpc'];rpc=Rpc(binary,project,journal);calls=[]
        def call(method,params=None):
            r=rpc.call(method,params);calls.append({'method':method,'params':params,'result':r});write_json(out/'public-commands.json',calls);return r
        try:
            before=call('project.read');transaction_commands=[{'op':'delete_scene','scene_id':s} for s in before['project']['scenes']]
            transaction_commands += [{'op':'create_scene','scene':{'id':ulid(99),'name':'Captured native audio game','entities':entities(clips)}},
                {'op':'set_scene_program','scene_id':ulid(99),'program':{'kind':'wasm','module':module['asset']['id']}},
                {'op':'set_input_actions','actions':{name:{'kind':'button','bindings':[{'type':'key','code':key}]} for _,name,key in ACTIONS}}]
            call('command.execute',{'expected_revision':before['revision'],'description':'Author captured Rust audio gameplay','commands':transaction_commands})
            selected=call('project.read')['project'];call('history.undo');require(call('project.read')['project']==before['project'],'Atomic Undo')
            call('history.redo');require(call('project.read')['project']==selected,'Atomic Redo');call('project.save');saved=call('project.read')
        finally:rpc.close()
        rpc=Rpc(binary,project,journal)
        try:require(call('project.read')==saved,'Editor-default journal path reopened through public RPC with --journal')
        finally:rpc.close()
        execute([binary,'validate',project])
        authored_sha256=sha256(project.read_bytes())
        for name in ('static.wav','stream.wav','behavior.wasm'):remove_file(inputs/name)
        remove_tree(build)
        raw=execute([oracle,project,ulid(99),out/'oracle.scene',out/'oracle.pcm',TICKS]).stdout
        rows=[json.loads(line) for line in raw.splitlines()]
        require(len(rows)==TICKS and [r['tick'] for r in rows]==list(range(1,TICKS+1)),'Native oracle clock')
        require(sum(r['pcm_nonzero'] for r in rows)>40 and any(r['pcm_silent'] for r in rows),'Actual mixer nonzero and paused output exercised')
        write_json(out/'oracle.json',rows);write_json(plan,observation(rows));require(len(observation(rows)['checks'])==248,'Bounded public vector assertions')
        write_json(positive,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[]})
        dev=value([binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',positive,'--observe',plan,'--pcm-output',out/'development.pcm'])
        require(dev['passed'] and dev['observations']['passed'] and not dev['script_source_parsing'] and dev['cpu_render_input'] and not dev['gpu_submitted'],'Captured-only development path')
        check_logs(dev['logs'],rows,require);require((out/'development.pcm').read_bytes()==(out/'oracle.pcm').read_bytes(),'Exact actual native-mixer PCM oracle');write_json(out/'development.json',dev)
        for name,_,key in ACTIONS:
            replay=out/f'{name}-input.json';write_json(replay,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,
                'frames':[{'tick':14,'events':[{'type':'key','code':key,'down':True}]}]})
            pcm=out/f'dev-{name}.pcm';refusal('dev-'+name,[binary,'native-play',project,'--camera',ulid(300),'--ticks',TICKS,'--input-replay',replay,'--pcm-output',pcm],pcm,DEV_ERRORS[name])
        require(sha256(project.read_bytes())==authored_sha256,'Development changed the authored project')
        write_json(out/'preflight.json',{'passed':True,'sources':source_hashes,'headless_sha256':sha256(binary.read_bytes()),
            'oracle_binary_sha256':sha256(oracle.read_bytes()),'oracle_scene_sha256':sha256((out/'oracle.scene').read_bytes()),
            'project_sha256':authored_sha256,'authoring_unchanged':True,'ticks':TICKS,'complete_records':TICKS*6,'record_scalars':31,
            'bounded_vector_checks':248,'source_deleted':True,'journal':str(journal)})
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
    for path in (out/'package/symbols').rglob('*'):
        if path.is_file() and (path.name.endswith('.map') or path.name=='script-symbols.json'):remove_file(path)
    require(not any(scratch.iterdir()),'Exporter private builds survived');scratch.rmdir()
    package_before=hashes(out/'package')
    environment={'PATH':str(player.parent),'TMPDIR':str(out),'TEMP':str(out),'TMP':str(out)}
    environment.update({k:os.environ[k] for k in ('SystemRoot','SystemDrive','WINDIR') if k in os.environ})
    for i in range(3):
        pcm=out/f'run-{i}.pcm';r=value([player,'--package',data,'--ticks',TICKS,'--input-recording',positive,'--observe',plan,'--output',pcm],cwd=player.parent,env=environment)
        require(r['observations']==dev['observations'] and r['logs']==dev['logs'] and r['tick']==dev['tick'] and
                r['frames']==dev['frames'] and r['pcm_sha256']==dev['pcm_sha256'],'Exact development/package observable parity')
        require(pcm.read_bytes()==(out/'oracle.pcm').read_bytes(),'Exact native mixer oracle PCM after source/cache/build deletion')
        check_logs(r['logs'],rows,require);write_json(out/f'run-{i}.json',r)
    for name,_,_ in ACTIONS:
        pcm=out/f'package-{name}.pcm';refusal('package-'+name,[player,'--package',data,'--ticks',TICKS,'--input-recording',out/f'{name}-input.json','--output',pcm],pcm,PACKAGE_ERRORS[name],cwd=player.parent,env=environment)
    pcm=out/'checkpoint-refused.pcm';save=out/'checkpoint-refused.bin'
    refusal('checkpoint-refused',[player,'--package',data,'--ticks',1,'--output',pcm,'--save-checkpoint',save],pcm,
        'Error: Checkpoint(Unsupported("static module state ABI"))',cwd=player.parent,env=environment)
    require(not save.exists(),'Unsupported checkpoint published output')
    require(package_before==hashes(out/'package'),'Packaged inputs changed during fresh-process verification')
    require(source_hashes=={str(p.relative_to(ROOT)):sha256(p.read_bytes()) for p in sources},'Proof sources changed during execution')
    write_json(out/'result.json',{'passed':True,'target':report['target'],'player_sha256':report['player_sha256'],'program':report['program'],
        'fresh_ordinary_package_runs':3,'ticks_each':TICKS,'per_tick_records':6,'record_scalars_checked_per_run':TICKS*186,
        'public_vector_checks_per_run':248,'source_deleted_before_development_and_export':True,'project_cache_build_maps_deleted_before_runs':True,
        'public_import_atomic_select_undo_redo_journal_reopen':True,'no_static_module_fixture_flag':True,'exact_native_mixer_pcm':True,
        'refusals':[f'{host}-{name}' for host in ['dev','package'] for name,_,_ in ACTIONS]+['checkpoint-refused'],
        'refusal_scope':'Exact diagnostic, no stdout/PCM/save; callback prefix rollback separately covered by actual-host conformance. Seek beyond captured duration is an audio-host failure after native command acceptance.',
        'removed_private_files':removed,'package_hashes':package_before,'package_unchanged':True,'proof_source_hashes':source_hashes,
        'commands_sha256':sha256(command_path.read_bytes()),'limits':['Local desktop CPU package only; no GPU/rendered/device/performance or phase gate.',
        'Declarations are desired committed state, never actual mixer cursors. Actual audio behavior is measured through native/Wasmtime/compiled PCM.',
        'Whole guest checkpoints explicitly unsupported.','Only this proof source/cache/build/maps removed; shared engine/toolchain retained.']})
    print(json.dumps({'passed':True,'output':str(out),'fresh_runs':3,'full_scalar_observations_each':TICKS*186},indent=2))

if __name__=='__main__':main()
