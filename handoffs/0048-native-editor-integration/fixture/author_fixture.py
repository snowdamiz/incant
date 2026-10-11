#!/usr/bin/env python3
"""Disposable0048 fixture: public init/import/incant_cmd/save and CPU observations only."""
import ast
import hashlib
import json
import math
import os
from pathlib import Path
import selectors
import signal
import struct
import subprocess

OUT = Path('/tmp/incant-native-0048-fixtures')
REPO = Path('/Users/andreyyurlov/.codex/worktrees/native-agent-play/incant')
SOURCE = '263cacc122760ced67efee4caeb2218ea13810f4'
BINARY = Path('/tmp/incant-native-review-0048-fixture-bin/incant_headless')
BINARY_SHA = '4c4cb80869e6b2ceb6c26d70217b0571dbec495701dfe39eb9e4ce09f1c5ae3e'
NODE = Path('/Users/andreyyurlov/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')
SAVED = OUT / 'saved'
PROJECT = SAVED / 'project.json'
JOURNAL = SAVED / 'project.journal.jsonl'
IDS = {name: '000000000000000000000000' + suffix for name, suffix in {
    'key_script':'11', 'pointer_script':'12', 'move_script':'13',
    'key_probe':'21', 'pointer_probe':'22', 'moving_mesh':'23', 'stationary_mesh':'24',
    'camera':'31', 'second_camera':'32', 'no_camera_scene':'41', 'no_camera_mesh':'42', 'rename_target':'51'}.items()}
ENV = os.environ.copy()
ENV['PATH'] = str(NODE.parent) + os.pathsep + ENV.get('PATH', '')
TRANSCRIPT = OUT / 'public-commands.jsonl'
COMMANDS = []
PROCESSES = []

def require(test, message):
    if not test: raise AssertionError(message)

def sha(data): return hashlib.sha256(data).hexdigest()
def digest(path): return sha(Path(path).read_bytes())
def frozen(path): return subprocess.check_output(['git', 'show', f'{SOURCE}:{path}'], cwd=REPO)
def record(value):
    with TRANSCRIPT.open('a') as handle: handle.write(json.dumps(value, sort_keys=True) + '\n')

def run(args, name):
    result = subprocess.run([str(BINARY), *map(str,args)], cwd=REPO, env=ENV, text=True, capture_output=True, timeout=60)
    (OUT / f'{name}.stdout').write_text(result.stdout)
    (OUT / f'{name}.stderr').write_text(result.stderr)
    item={'kind':'cli', 'argv':[str(BINARY), *map(str,args)], 'exit_code':result.returncode, 'stdout':result.stdout, 'stderr':result.stderr}
    COMMANDS.append(item);record(item)
    require(result.returncode == 0, f'{name} failed: {result.stderr}')
    return json.loads(result.stdout)

class Rpc:
    def __init__(self, index):
        self.err = (OUT / f'rpc-{index}.stderr').open('w')
        self.process = subprocess.Popen([str(BINARY), 'rpc', str(PROJECT), '--journal', str(JOURNAL)],
            cwd=REPO, env=ENV, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.err,
            text=True, encoding='utf-8', start_new_session=True)
        self.selection=selectors.DefaultSelector();self.selection.register(self.process.stdout,selectors.EVENT_READ)
        self.requests=0;self.index=index
        PROCESSES.append({'pid':self.process.pid,'argv':[str(BINARY),'rpc',str(PROJECT),'--journal',str(JOURNAL)]})
    def call(self, method, params=None, expected_error=False):
        self.requests += 1
        request={'id':self.requests,'method':method,'params':params or {}}
        self.process.stdin.write(json.dumps(request) + '\n');self.process.stdin.flush()
        require(self.selection.select(30), f'RPC timeout: {method}')
        line=self.process.stdout.readline(16*1024*1024+1)
        require(line and len(line)<=16*1024*1024, f'RPC ended or exceeded bound: {method}')
        response=json.loads(line)
        record({'kind':'rpc','process':self.index,'request':request,'response':response})
        require(('error' in response)==expected_error, f'{method}: {response}')
        return response['error'] if expected_error else response['result']
    def close(self):
        self.selection.close()
        self.process.stdin.close()
        try: self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            os.killpg(self.process.pid,signal.SIGKILL);self.process.wait();raise
        finally:
            self.process.stdout.close();self.err.close()
            PROCESSES[self.index-1]['reaped']=True
            PROCESSES[self.index-1]['exit_code']=self.process.returncode
        require(self.process.returncode == 0, 'RPC process failed')

def transform(translation=(0,0,0)):
    return {'translation':list(translation),'rotation':[0,0,0,1],'scale':[1,1,1]}
def entity(scene, identity, name, components):
    return {'op':'create_entity','scene_id':scene,'entity':{'id':identity,'name':name,'parent':None,'provenance':None,'components':components}}
def script(identity, name, path, source):
    return [{'op':'write_script','script_id':identity,'source':source,'expected_source':{'state':'missing'}},
            {'op':'upsert_script','script':{'id':identity,'name':name,'path':path}}]
def near(actual, expected):
    require(len(actual)==len(expected) and all(abs(a-b)<1e-9 for a,b in zip(actual,expected)), f'{actual} != {expected}')

def main():
    require(digest(BINARY)==BINARY_SHA, 'wrong preliminary binary')
    require(not SAVED.exists() and not TRANSCRIPT.exists(), 'use a fresh disposable fixture directory')
    SAVED.mkdir()
    provenance=json.loads(frozen('crates/incant_assets/tests/fixtures/fbx/provenance.json'))
    source_name='blender_272_cube_7400_binary.fbx'
    asset_source=frozen('crates/incant_assets/tests/fixtures/fbx/'+source_name)
    require(sha(asset_source)==provenance['files_sha256'][source_name], 'model fixture changed')
    (SAVED/source_name).write_bytes(asset_source)
    (SAVED/'MODEL-LICENSE').write_bytes(frozen('crates/incant_assets/tests/fixtures/fbx/LICENSE'))
    (OUT/'upstream-model-provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
    vetted=frozen('handoffs/0044-native-editor-play/tools/native/author-input-variant.py')
    values={node.targets[0].id:ast.literal_eval(node.value) for node in ast.parse(vetted).body
        if isinstance(node,ast.Assign) and isinstance(node.targets[0],ast.Name) and node.targets[0].id in ('KEY_SOURCE','POINTER_SOURCE')}
    move_source=("import type { NativeBehavior } from '@incant/runtime/native';\n"
        "// 0048 numerical probe: attached entities move +0.25 world units per second.\n"
        "export default { update({ instances, chunks, dt }) {\n"
        "  for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++) {\n"
        "    if (!instances.some(instance => instance.entity === chunk.ids[i])) continue;\n"
        "    chunk.transforms[i * chunk.transformStride] += 0.25 * dt;\n"
        "  }\n} } satisfies NativeBehavior;\n")
    run(['init',PROJECT,'--entities','0','--name','0048 Native Acceptance Fixture'],'init')
    imported=run(['import',PROJECT,source_name],'import')
    asset=imported['asset'];IDS['model_asset']=asset['id']
    require(asset['kind']=='model' and imported['details']['meshes']>0 and imported['details']['vertices']>0, 'no imported model geometry')
    rpc=Rpc(1)
    try:
        initial=rpc.call('project.read');scene=next(iter(initial['project']['scenes']));IDS['main_scene']=scene;IDS['project']=initial['project']['id']
        require(initial['revision']==1,'import not journaled once')
        mesh={'mesh':asset['id'],'materials':[],'cast_shadows':True}
        camera={'projection':{'kind':'perspective'},'fov_degrees':60,'near':0.1,'far':100}
        first=script(IDS['move_script'],'Numerical Movement','move.ts',move_source)+[
            entity(scene,IDS['moving_mesh'],'Moving Imported Mesh',{'Transform':transform((2,0,0)),'MeshRenderer':mesh,'Script':{'source':IDS['move_script'],'props':{}}}),
            entity(scene,IDS['stationary_mesh'],'Stationary Imported Mesh',{'Transform':transform(),'MeshRenderer':mesh}),
            entity(scene,IDS['camera'],'Stationary Camera',{'Transform':transform((0,2,8)),'Camera':camera}),
            entity(scene,IDS['second_camera'],'Second Stationary Camera',{'Transform':transform((0,2,12)),'Camera':camera}),
            entity(scene,IDS['rename_target'],'Allowance Rename Target',{}),
            {'op':'create_scene','scene':{'id':IDS['no_camera_scene'],'name':'Spatial Without Camera','entities':{}}},
            entity(IDS['no_camera_scene'],IDS['no_camera_mesh'],'No Camera Imported Mesh',{'Transform':transform(),'MeshRenderer':mesh})]
        tx=rpc.call('command.execute',{'expected_revision':1,'description':'Author0048 imported geometry and movement fixture','commands':first});require(tx['revision']==2,'movement transaction failed')
        second=script(IDS['key_script'],'Key Probe','key-probe.ts',values['KEY_SOURCE'])+[
            entity(scene,IDS['key_probe'],'Key Probe',{'Transform':transform(),'Script':{'source':IDS['key_script'],'props':{}}})]
        tx=rpc.call('command.execute',{'expected_revision':2,'description':'Add0048 instance-filtered key probe','commands':second});require(tx['revision']==3,'key transaction failed')
        third=script(IDS['pointer_script'],'Pointer Probe','pointer-probe.ts',values['POINTER_SOURCE'])+[
            entity(scene,IDS['pointer_probe'],'Pointer Probe',{'Transform':transform(),'Script':{'source':IDS['pointer_script'],'props':{}}})]
        tx=rpc.call('command.execute',{'expected_revision':3,'description':'Add0048 instance-filtered pointer probe','commands':third});require(tx['revision']==4,'pointer transaction failed')
        authored=rpc.call('project.read')
        require(rpc.call('project.save')['saved'],'save failed')
        history=rpc.call('history.read');require(len(history)==4,'wrong history length')
        require(history[0]['actor']['origin']=='import' and all(tx['actor']['origin']=='user' for tx in history[1:]),'wrong provenance')
    finally: rpc.close()
    baseline={str(p.relative_to(SAVED)):digest(p) for p in SAVED.rglob('*') if p.is_file()}
    rpc=Rpc(2)
    states=[]
    try:
        require(rpc.call('project.read')==authored,'fresh journal reopen differs')
        for name,identity,source in [('move',IDS['move_script'],move_source),('key',IDS['key_script'],values['KEY_SOURCE']),('pointer',IDS['pointer_script'],values['POINTER_SOURCE'])]:
            read=rpc.call('script.read',{'script_id':identity})['untrusted_source_data']
            require(read['source']==source and read['matches_history'], f'{name} source recovery differs')
        rejected=rpc.call('play.start',{'scene':IDS['no_camera_scene']},expected_error=True)
        require('spatial content requires an explicit --camera' in rejected['message'],'unexpected no-camera refusal')
        start=rpc.call('play.start',{'scene':scene,'camera':IDS['camera']})
        require(start['tick']==0 and start['cpu_render_input'] and not start['gpu_submitted'] and start['script_source_parsing'],'CPU startup did not load authored scene')
        selected=[IDS[k] for k in ['moving_mesh','stationary_mesh','key_probe','pointer_probe','camera','second_camera']]
        observation={'entities':selected,'include_input':True}
        def step(events):
            state=rpc.call('play.step',{'events':events,'observe':observation})
            require(not state['gpu_submitted'],'GPU work is forbidden')
            states.append(state)
            for key in ['stationary_mesh','camera','second_camera']:
                require(state['entities'][IDS[key]]['transform']==authored['project']['scenes'][scene]['entities'][IDS[key]]['components']['Transform'],f'{key} was touched by an unrelated probe')
            return state
        def value(state,key): return state['entities'][IDS[key]]['transform']
        s=step([{'type':'focus','focused':False},{'type':'key','code':'KeyD','down':True}])
        near(value(s,'key_probe')['translation'],[0,0,0]);require(value(s,'key_probe')['scale'][0]==1,'unfocused marker wrong')
        s=step([{'type':'focus','focused':True},{'type':'pointer_position','position':[123.25,45.5]},
            {'type':'key','code':'KeyD','down':True},{'type':'mouse_button','button':'left','down':True},
            {'type':'wheel','delta':[0,2.5],'unit':'pixels'},{'type':'wheel','delta':[0,1],'unit':'lines'}])
        near(value(s,'key_probe')['translation'],[1,1,3.5]);require(value(s,'key_probe')['scale'][0]==2,'focused marker wrong')
        near(value(s,'pointer_probe')['translation'][:2],[123.25,45.5])
        s=step([{'type':'focus','focused':False}])
        near(value(s,'key_probe')['translation'],[1,1,3.5]);require(value(s,'key_probe')['scale'][0]==1,'focus loss marker wrong')
        for _ in range(20): s=step([])
        near(value(s,'moving_mesh')['translation'],[2+0.25*23/60,0,0])
        near(value(s,'key_probe')['translation'],[1,1,3.5])
        stopped=rpc.call('play.stop');require(not stopped['playing'] and stopped['project']==authored['project'],'Stop changed authoring')
        require(rpc.call('project.read')==authored,'CPU Play changed authoring')
    finally: rpc.close()
    run(['validate',PROJECT],'validate')
    require(baseline=={str(p.relative_to(SAVED)):digest(p) for p in SAVED.rglob('*') if p.is_file()},'CPU verification changed authored/cooked bytes')
    cache_files=list((SAVED/'.incant/cache/models').glob('*'))
    model_path=next(p for p in cache_files if p.is_file() and p.read_bytes().startswith(b'INCMOD03'))
    data=model_path.read_bytes();require(sha(data[:-32])==data[-32:].hex(),'cooked model checksum differs')
    size=struct.unpack_from('<I',data,8)[0];metadata=json.loads(data[12:12+size]);count=struct.unpack_from('<I',data,12+size)[0]
    require(metadata['fingerprint']==asset['sha256'] and count==imported['details']['meshes'] and count>0,'cooked model catalog linkage differs')
    linked=[IDS[k] for k in ['moving_mesh','stationary_mesh','no_camera_mesh']]
    for identity in linked:
        entity_data=next(s['entities'][identity] for s in authored['project']['scenes'].values() if identity in s['entities'])
        require(entity_data['components']['MeshRenderer']['mesh']==asset['id'],'mesh did not reference imported asset')
    evidence={'passed':True,'fixture_root':str(SAVED),'project':str(PROJECT),'journal':str(JOURNAL),'ids':IDS,
        'source_commit':SOURCE,'headless_sha256':digest(BINARY),'headless_manifest_sha256':digest(BINARY.parent/'manifest.json'),
        'revision':4,'tick_rate':60,'history':history,'public_mutations_only':True,'saved_reopen_equal':True,
        'authoring_unchanged_by_cpu_play':True,'no_camera_refusal':rejected,'gpu_submitted':False,
        'cpu_ticks':23,'cpu_runtime_source_sha256':start['script_source_sha256'],
        'model_import':imported,'cooked_model':{'path':str(model_path),'sha256':digest(model_path),'magic':'INCMOD03','mesh_count':count,'metadata':metadata},
        'input_checks':{'key_d_ticks':1,'left_button_ticks':1,'wheel_sum':3.5,'pointer_logical_pixels':[123.25,45.5],'focus_scale_x':[1,2,1]},
        'moving_mesh_final_x':2+0.25*23/60,'stationary_camera_and_mesh_unchanged':True,
        'script_logic_source_sha256':sha(vetted),'fixture_files_sha256':baseline,'processes':PROCESSES,
        'limitations':['Preliminary fixture authoring/CPU evidence only; final native binary/graphics grant is root-owned.',
            'No appearance, rendered visibility, live account/provider, GPU/native window or phase gate was evaluated.',
            'Authoring probe logic uses existing0044 instance filters; camera is stationary with local-Z-forward convention checked from source only.']}
    (OUT/'numerical-states.json').write_text(json.dumps(states,indent=2)+'\n')
    (OUT/'fixture-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    print(json.dumps({'passed':True,'project':str(PROJECT),'ids':IDS,'evidence':str(OUT/'fixture-evidence.json')}))

if __name__=='__main__': main()
