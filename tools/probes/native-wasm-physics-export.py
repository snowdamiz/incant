#!/usr/bin/env python3
"""Public author/import/cook/export physics game, then fresh source-free players.

Only this probe's new private inputs/builds/debug maps are removed. No GPU,
window, checkpoint codec, device performance or phase acceptance is involved.
"""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TICKS = 20

DEV_ERRORS = {'bad-owner': 'Error: native play startup/runtime failure: development Wasm rejected or trapped: invalid physics query: character requires a nonsensor kinematic body with zero angular velocity', 'short-scratch': 'Error: native play startup/runtime failure: development Wasm rejected or trapped: wasm trap: wasm `unreachable` instruction executed', 'bad-number': 'Error: native play startup/runtime failure: development Wasm rejected or trapped: wasm trap: wasm `unreachable` instruction executed', 'late-command': 'Error: native play startup/runtime failure: runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 123]) does not exist'}
PACKAGE_ERRORS = {'bad-owner': 'Error: Session(Module(Boundary(Query(Physics(Query("character requires a nonsensor kinematic body with zero angular velocity"))))))', 'short-scratch': 'Error: Session(Module(Trap(1)))', 'bad-number': 'Error: Session(Module(Trap(1)))', 'late-command': 'Error: Session(Module(Boundary(Runtime(MissingEntity(StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 123]))))))'}

def ulid(n):
    alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
    return ''.join(alphabet[(n >> (5 * i)) & 31] for i in reversed(range(26)))


def entities():
    """Boxes include a planar floor gap, overlapping sensors and a character wall."""
    values = {}
    specs = [(1,[.5,-.5,7],[.5,.5],False,False),(4,[2.5,-.5,7],[.5,.5],False,False),
             (2,[.5,2,4],[.5,.5],False,True),(3,[.5,2,9],[.5,.5],True,False),
             (10,[0,-.5,0],[20,.5,20],False,False),(11,[0,.6,0],[.5,.5,.5],False,True),
             (12,[2.5,1,0],[.5,2,4],False,False),(13,[1,1,0],[1.5,2,4],True,False),
             (20,[0,0,0],None,False,False)]
    for n,at,half,sensor,kinematic in specs:
        components = {'Transform':{'translation':at,'rotation':[0,0,0,1],'scale':[1,1,1]}}
        if half:
            planar = len(half)==2
            components['Collider2D' if planar else 'Collider'] = {
                'shape':{'kind' if planar else 'type':'box','half_extents':half},
                'density':1000,'friction':.5,'restitution':0,'sensor':sensor,
                'memberships':4294967295,'filter':4294967295}
            if kinematic:
                components['Velocity']={'linear':[0,0,0]}
                components['RigidBody2D' if planar else 'RigidBody']={
                    'motion':'kinematic','gravity_scale':1,'linear_damping':0,
                    'angular_damping':0,'can_sleep':True,'ccd':True}
        values[ulid(n)]={'id':ulid(n),'name':f'Physics {n}','parent':None,'provenance':None,'components':components}
    return values


def observation(rows):
    checks=[]
    for row in rows:
        for field,n in [('marker',20),('character',11),('planar',2)]:
            checks.append({'name':f'{field}-{row["tick"]}','tick':row['tick'],
                'expect':{'type':'vector','entity':ulid(n),'property':'translation','value':row[field],'tolerance':1e-9}})
    return {'format':'incant-native-observation','version':1,'entities':[ulid(n) for n in [2,11,20]],'checks':checks}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--oracle',type=Path,required=True)
    args=parser.parse_args()
    sys.path.insert(0,str(ROOT/'tools'))
    import export_native
    import native_process
    import rust_guest
    from native_package import require,sha256,write_json
    provision=runpy.run_path(str(ROOT/'tools/probes/shipping-wasm2c.py'))['provision']
    Rpc=runpy.run_path(str(ROOT/'tools/probes/script-source-transactions.py'))['Rpc']
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    inputs=out/'authoring';inputs.mkdir()
    build=out/'guest-build';build.mkdir()
    trusted=out/'trusted-tools';trusted.mkdir()
    scratch=out/'private-builds';scratch.mkdir();tempfile.tempdir=str(scratch)
    project=inputs/'project.json';journal=inputs/'project.journal.jsonl'
    binary=args.binary.resolve(strict=True);oracle=args.oracle.resolve(strict=True)
    removed={}
    command_receipts=[]
    def hashes(path): return {str(p.relative_to(out)):sha256(p.read_bytes()) for p in sorted(path.rglob('*')) if p.is_file()}
    def remove_tree(path):
        require(path.is_relative_to(out) and path.is_dir() and not path.is_symlink(),'Unsafe proof cleanup')
        removed.update(hashes(path));shutil.rmtree(path)
    def execute(argv,**kwargs):
        value=native_process.run(list(map(str,argv)),text=True,encoding='utf-8',**kwargs)
        command_receipts.append({'argv':list(map(str,argv)),'exit_code':value.returncode,'stdout_sha256':sha256(value.stdout.encode()),'stderr_sha256':sha256(value.stderr.encode())});write_json(out/'commands.json',command_receipts)
        require(value.returncode==0,value.stderr[-8000:]);return value
    def value(argv,**kwargs):return json.loads(execute(argv,**kwargs).stdout)
    def refusal(name,argv,output,*,expected,**kwargs):
        p=native_process.run(list(map(str,argv)),text=True,encoding='utf-8',**kwargs)
        require(p.returncode==1 and not output.exists() and p.stderr.strip()==expected,f'{name} wrong runtime boundary: {p.returncode}: {p.stderr}')
        require(not p.stdout, f'{name} unexpectedly published stdout')
        require('failure prefix must not publish' not in p.stdout,'Failed log prefix published')
        record={'command':list(map(str,argv)),'exit_code':p.returncode,'stdout':p.stdout,'stderr':p.stderr,'output_absent':True,'expected_stderr':expected}
        command_receipts.append(record);write_json(out/'commands.json',command_receipts)
        write_json(out/f'{name}.json',record);return record
    execute([binary,'init',project,'--entities','0'])
    rustc=execute(['rustup','which','--toolchain','1.99.0','rustc']).stdout.strip()
    source=build/'behavior.rs';source.write_bytes((ROOT/'tools/probes/native-wasm-physics-export/behavior.rs').read_bytes())
    rust_guest.compile(rustc,source,build/'behavior.wasm')
    write_json(out/'guest-compile.json',{'compiler':execute([rustc,'-vV']).stdout,'wasm_sha256':sha256((build/'behavior.wasm').read_bytes()),'sdk':rust_guest.provenance()})
    shutil.copy2(build/'behavior.wasm',inputs/'behavior.wasm')
    imported=value([binary,'import',project,'behavior.wasm']);require(imported['asset']['kind']=='wasm','Public Wasm import')
    write_json(out/'import.json',imported)
    scene=ulid(99);rpc=Rpc(binary,project,journal)
    try:
        before=rpc.call('project.read')
        transaction_commands=[{'op':'delete_scene','scene_id':s} for s in before['project']['scenes']]
        transaction_commands += [{'op':'create_scene','scene':{'id':scene,'name':'Physics queries','entities':entities()}},
            {'op':'set_scene_program','scene_id':scene,'program':{'kind':'wasm','module':imported['asset']['id']}},
            {'op':'set_input_actions','actions':{name:{'kind':'button','bindings':[{'type':'key','code':key}]} for name,key in [('bad_owner','KeyA'),('short_scratch','KeyB'),('bad_number','KeyC'),('late_command','KeyD')]}}]
        receipt=rpc.call('command.execute',{'expected_revision':before['revision'],'description':'Author public physics query game','commands':transaction_commands})
        selected=rpc.call('project.read')['project'];rpc.call('history.undo');require(rpc.call('project.read')['project']==before['project'],'Atomic authoring Undo')
        rpc.call('history.redo');require(rpc.call('project.read')['project']==selected,'Atomic authoring Redo')
        rpc.call('project.save');saved=rpc.call('project.read');write_json(out/'authoring-receipt.json',receipt)
    finally:rpc.close()
    rpc=Rpc(binary,project,journal)
    try:require(rpc.call('project.read')==saved,'Public journal reopen')
    finally:rpc.close()
    execute([binary,'validate',project])
    removed[str((inputs/'behavior.wasm').relative_to(out))]=sha256((inputs/'behavior.wasm').read_bytes());(inputs/'behavior.wasm').unlink()
    remove_tree(build)
    # The ordinary exporter consumes only the captured cooked module.
    wabt,provenance=provision(trusted);write_json(out/'wabt.json',provenance)
    print('Public authoring, Undo/Redo and journal reopen passed; source removed. Starting ordinary export.',flush=True)
    report=export_native.export(project,out/'package',wabt=wabt)
    require(report['program']['kind']=='wasm' and report['build_storage_removed_before_startup'],'Ordinary static module export')
    player=out/'package/player'/('incant_game_player.exe' if os.name=='nt' else 'incant_game_player')
    data=player.parent/'data'
    raw=execute([oracle,data/'world.scene',TICKS]).stdout
    rows=[json.loads(line) for line in raw.splitlines()]
    require(len(rows)==TICKS and [r['tick'] for r in rows]==list(range(1,TICKS+1)),'Native oracle clock')
    require(all(sum(r['events'][i] for r in rows)>0 for i in range(4)),'Both dimensions must enter and exit')
    write_json(out/'oracle.json',rows);plan=out/'observe.json';write_json(plan,observation(rows))
    positive_replay=out/'positive-input.json';write_json(positive_replay,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[]})
    dev=value([binary,'native-play',project,'--ticks',TICKS,'--input-replay',positive_replay,'--observe',plan,'--pcm-output',out/'development.pcm'])
    require(dev['passed'] and dev['observations']['passed'] and not dev['script_source_parsing'],'Captured-only development/native oracle')
    require(dev['logs']=={'dropped':0,'entries':[{'level':'info','message':'physics queries accepted','tick':t} for t in range(1,TICKS+1)]},'Exact successful guest log/clock sequence')
    write_json(out/'development.json',dev)
    failures={}
    for name,key in [('bad-owner','KeyA'),('short-scratch','KeyB'),('bad-number','KeyC'),('late-command','KeyD')]:
        replay=out/f'{name}-input.json';write_json(replay,{'format':'incant-input','version':1,'tick_rate':60,'start_tick':0,'ticks':TICKS,'frames':[{'tick':5,'events':[{'type':'key','code':key,'down':True}]}]})
        pcm=out/f'dev-{name}.pcm';failures['dev-'+name]=refusal('dev-'+name,[binary,'native-play',project,'--ticks',TICKS,'--input-replay',replay,'--pcm-output',pcm],pcm,expected=DEV_ERRORS[name])
    cache=inputs/'.incant/cache/wasm'/(imported['asset']['sha256']+'.wasm.json');original=cache.read_bytes();corrupt=json.loads(original);corrupt['prepared_sha256']='0'*64;write_json(cache,corrupt)
    pcm=out/'dev-cache-mismatch.pcm';failures['dev-cache-mismatch']=refusal('dev-cache-mismatch',[binary,'native-play',project,'--ticks',1,'--pcm-output',pcm],pcm,expected='Error: native play startup/runtime failure: native play startup failure: static Wasm preparation: captured module identity mismatch');require('identity mismatch' in failures['dev-cache-mismatch']['stderr'],'Cache identity mismatch diagnostic');cache.write_bytes(original)
    print('Native oracle and captured development parity/refusals passed. Removing private project/cache/tools.',flush=True)
    remove_tree(inputs);remove_tree(trusted)
    for path in (out/'package/symbols').glob('*.map'):
        require(path.is_file() and not path.is_symlink(),'Unexpected private debug map');removed[str(path.relative_to(out))]=sha256(path.read_bytes());path.unlink()
    require(not any(scratch.iterdir()),'Exporter private build storage survived');scratch.rmdir()
    environment={'PATH':str(player.parent),'TMPDIR':str(out),'TEMP':str(out),'TMP':str(out)}
    environment.update({k:os.environ[k] for k in ('SystemRoot','SystemDrive','WINDIR') if k in os.environ})
    runs=[]
    for i in range(3):
        pcm=out/f'run-{i}.pcm';r=value([player,'--package',data,'--ticks',TICKS,'--input-recording',positive_replay,'--observe',plan,'--output',pcm],cwd=player.parent,env=environment)
        require(r['observations']['passed'] and r['logs']==dev['logs'] and r['tick']==dev['tick'] and r['frames']==dev['frames'] and r['pcm_sha256']==dev['pcm_sha256'],'Native oracle/development/package parity')
        require(r['observations']==dev['observations'],'Exact observable game-state parity')
        write_json(out/f'run-{i}.json',r);runs.append(r)
    for name in ['bad-owner','short-scratch','bad-number','late-command']:
        pcm=out/f'package-{name}.pcm';failures['package-'+name]=refusal('package-'+name,[player,'--package',data,'--ticks',TICKS,'--input-recording',out/f'{name}-input.json','--output',pcm],pcm,expected=PACKAGE_ERRORS[name],cwd=player.parent,env=environment)
    pcm=out/'checkpoint-refused.pcm';save=out/'checkpoint-refused.bin';failures['checkpoint']=refusal('checkpoint-refused',[player,'--package',data,'--ticks',1,'--output',pcm,'--save-checkpoint',save],pcm,expected='Error: Checkpoint(Unsupported("static module state ABI"))',cwd=player.parent,env=environment)
    require(not save.exists() and 'static module state ABI' in failures['checkpoint']['stderr'],'Explicit module checkpoint refusal')
    write_json(out/'result.json',{'passed':True,'target':report['target'],'player_sha256':report['player_sha256'],'program':report['program'],'ticks_each':TICKS,'fresh_ordinary_package_runs':len(runs),'native_oracle_checks':3*TICKS,'successful_same_input_replay_routes':True,'command_receipts_sha256':sha256((out/'commands.json').read_bytes()),'package_refusal_boundary':'exact runtime diagnostic and no PCM/stdout publication; not a session-step rollback claim','native_event_enter_exit_counts':[sum(r['events'][i] for r in rows) for i in range(4)],'public_import_atomic_select_undo_redo_journal_reopen':True,'source_deleted_before_development_and_export':True,'authoring_cache_build_maps_deleted_before_three_runs':True,'no_static_module_fixture_flag':True,'refusals':list(failures),'removed_private_files':removed,'retained_package_hashes':hashes(out/'package'),'oracle_binary_sha256':sha256(oracle.read_bytes()),'headless_binary_sha256':sha256(binary.read_bytes()),'proof_source_hashes':{str(p.relative_to(ROOT)):sha256(p.read_bytes()) for p in [Path(__file__),ROOT/'tools/probes/native-wasm-physics-export/behavior.rs',ROOT/'crates/incant_runtime/examples/physics_package_oracle.rs']},'limits':['local desktop CPU package proof, no GPU or pixel acceptance','no phone/performance/phase gate','whole guest and physics checkpoints remain unsupported','repository engine and SDK source intentionally retained; only private project inputs/build/maps removed']})
    print(json.dumps({'passed':True,'output':str(out),'runs':len(runs),'refusals':len(failures)},indent=2))

if __name__=='__main__':main()
