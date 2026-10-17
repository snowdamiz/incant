"""Data-only editor CPU/window replay. Archived source is never imported/executed."""
import gzip
import hashlib
import json
import math
from pathlib import Path

HEAD='b3da3b6d9bbb6a8870480ae54d97a2884d20edde'
PACKET='d07454d7c3c92ef17cab007d78ff895def4805e93ffd7b4a6ad4336d91ded4bc'
STAGES=['manual_import_original','watched_vertices','undo_original','redo_vertices','incompatible_topology_retained_replacement','source_recovered_vertices']


def need(value,message):
    if not value:raise ValueError(message)


def sha(raw):return hashlib.sha256(raw).hexdigest()


def load(root):
    root=Path(root);manifest=json.loads((root/'manifest.json').read_text());path=root/'records.json.gz'
    need(path.stat().st_size<=4*1024**2,'Compressed bound')
    raw=path.read_bytes();need(len(raw)==manifest['archive']['bytes'] and sha(raw)==manifest['archive']['sha256'],'Archive binding')
    with gzip.open(path,'rb') as stream:raw=stream.read(16*1024**2+1)
    need(len(raw)<=16*1024**2 and len(raw)==manifest['archive']['decoded_bytes'],'Decoded bound')
    bundle=json.loads(raw);need(bundle['format']=='incant-animated-vertex-editor-records-v1','Data format')
    need(set(bundle['records'])==set(manifest['records']),'Exact portable inventory')
    records={}
    for name,row in bundle['records'].items():
        need(not Path(name).is_absolute() and '..' not in Path(name).parts,'Safe inert record name')
        data=row['text'].encode();need(row['bytes']==len(data) and row['sha256']==sha(data),'Original raw record hash')
        need(manifest['records'][name]=={'bytes':len(data),'sha256':sha(data)},'Manifest record binding')
        records[name]=data
    return records,manifest


def proof_check(p):
    need(p['passed'] is True and p['platform']=='macos' and p['asset_kind']=='animated-vertices','Actual Mac vertex workflow')
    for field in ['actual_window_presentations','manual_import_execute_history_handlers','real_source_watcher','undo_not_overwritten','source_recovery','stale_generation_refused','stop_releases_world','world_identity_unchanged','module_counter_continued','animation_continued','committed_incompatible_refused']:
        need(p[field] is True,'Required observed behavior: '+field)
    for field in ['pixel_capture','appearance_review','performance_or_phase_gate']:need(p[field] is False,'Scope boundary')
    need('backend: Metal' in p['adapter'] and type(p['play_generation']) is int and p['play_generation']>0,'Actual adapter/generation')
    tick=p['paused_tick'];need(type(tick) is int and tick>=0 and p['final_tick']==tick+1,'One continuing tick')
    need(p['vm_counter']=={'before_translation':tick*(tick+1),'expected_delta':2*(tick+1),'observed_delta':2*(tick+1)},'Continued VM arithmetic')
    stages=p['stages'];need([s['stage'] for s in stages]==STAGES,'Exact six handler stages')
    need(all(s['tick']==tick and s['recipe_matches'] is True for s in stages),'Paused live recipe')
    frames=[s['frames'] for s in stages];need(all(type(f) is int and f>0 for f in frames) and all(a<b for a,b in zip(frames,frames[1:])),'Actual later presentations')
    revisions=[s['history']['revision'] for s in stages]
    need(all(type(r) is int for r in revisions) and revisions==list(range(revisions[0],revisions[0]+6)),'Committed edit/refusal/recovery revisions')
    hashes=[s['accepted_fingerprint'] for s in stages]
    need(all(isinstance(h,str) and len(h)==64 and all(c in '0123456789abcdef' for c in h) for h in hashes),'Recipe identities')
    need(hashes[0]==hashes[2] and hashes[1]==hashes[3]==hashes[4]==hashes[5] and hashes[0]!=hashes[1],'Actual original/replacement UndoRedo and refusal retention')
    before=p['animation_step']['before'];after=p['animation_step']['after']
    for state in (before,after):
        need(set(state)=={'playback','completed','transition'} and state['completed'] is False and state['transition'] is None,'Complete animation state')
        play=state['playback'];need(set(play)=={'clip','seconds','speed','playing','looping'},'Complete playback')
        need(type(play['clip']) is int and play['clip']==0 and play['speed']==1 and play['playing'] is True and play['looping'] is True,'Fixed playback semantics')
        need(type(play['seconds']) in (int,float) and math.isfinite(play['seconds']) and 0<=play['seconds']<2,'Bounded animation clock')
    need(abs(before['playback']['seconds']-(.5+tick/60)%2)<1e-9 and abs(after['playback']['seconds']-(before['playback']['seconds']+1/60)%2)<1e-9,'Preserved and advanced animation clock')
    need(all(s['animation']==before for s in stages),'Complete paused animation in every stage')
    refusal=p['incompatible_refresh_refusal'];event=refusal['event'];authored=refusal['authored_fingerprint']
    need(len(authored)==64 and authored!=hashes[3] and refusal['accepted_fingerprint']==hashes[3] and refusal['committed_revision']==revisions[4],'Committed authored/live split')
    need(event['level']=='error' and isinstance(event['id'],str) and event['message'].startswith('Native Play asset refresh:') and 'animated model topology, rest or skin bindings changed' in event['message'],'Exact observed topology refusal')
    need(refusal['native_code_available'] is False and 'code' not in refusal,'No fabricated native error code')
    need(stages[4]['authored_fingerprint']==authored and stages[4]['refresh_error']==event,'Refused authored recipe retains accepted live catalog')
    return revisions[-1],hashes[-1]


def validate(records,manifest):
    js=lambda n:json.loads(records[n])
    need(manifest['producer_head']==HEAD and manifest['producer_packet_sha256']==PACKET,'Actual frozen producer')
    need(sha(records['packet.json'])==PACKET,'Frozen original packet')
    packet=js('packet.json');plan=js('plan.json');source=js('source.json')
    need(source['head']==plan['source_head']==HEAD,'Original source identity')
    for name,identity in packet['files'].items():need(sha(records[name])==identity['sha256'] and len(records[name])==identity['bytes'],'Frozen input '+name)
    for name in manifest['source_blobs']:need(sha(records['source-blobs/'+name])==source['files'][name]['sha256'],'Exact git source blob')
    excluded=manifest['excluded_binary_records'];cpu=js('cpu-result.json');window=js('window-result.json')
    review=js('root-review.json');need(review['producer']==HEAD,'Independent review producer')
    for field,name in [('cpu_result_sha256','cpu-result.json'),('cpu_terminal_closeout_sha256','cpu-terminal-closeout.json'),('window_result_sha256','window-result.json'),('window_terminal_closeout_sha256','window-terminal-closeout.json'),('actual_workflow_sha256','window-output/workflow/result.json')]:
        need(review[field]==sha(records[name]),'Independent review raw binding')
    all_retained={phase+'-output/'+n:i for phase,r in [('cpu',cpu),('window',window)] for n,i in r['retained'].items()}
    need(set(excluded)=={n for n in all_retained if '/private-builds/' in n or n.endswith('.incmodel')},'Exact explicitly omitted binary-cache inventory')
    for name,identity in all_retained.items():
        if name in excluded:need(excluded[name]==identity and name not in records,'Excluded binary metadata only')
        else:need(name in records and sha(records[name])==identity['sha256'] and len(records[name])==identity['bytes'],'Exact retained data '+name)
    need(len(cpu['artifacts'])==3 and {k:v['identity'] for k,v in cpu['artifacts'].items()}==manifest['external_artifacts'],'External artifact identities')
    cargo_rows=[json.loads(line) for line in records['cpu-output/003.stdout.log'].decode().splitlines()]+[json.loads(line) for line in records['cpu-output/004.stdout.log'].decode().splitlines()]
    for name,artifact in cpu['artifacts'].items():
        cargo=artifact['cargo'];package='incant_editor' if name=='editor_tests' else name
        need(cargo in cargo_rows and cargo['fresh'] is False and cargo['features']==plan['features'][package],'Actual fresh feature-bound artifacts')
        need(cargo['manifest_path'].startswith(plan['source_root']+'/') and cargo['profile']['test'] is (name=='editor_tests'),'Current path/normal-versus-test producer')
    for phase,r,count in [('cpu',cpu,7),('window',window,6)]:
        owner=js(phase+'-output/owner.json');close=js(phase+'-terminal-closeout.json')
        need(r['passed'] is True and owner['passed'] is True and close['passed'] is True and close['exec_terminal_exit']==0,'Actual successful terminal chain')
        need(r['source_head']==close['source_head']==HEAD and r['packet_sha256']==close['packet_sha256']==PACKET,'Terminal producer')
        need(close['result_sha256']==sha(records[phase+'-result.json']) and r['owner_sha256']==close['owner_sha256']==sha(records[phase+'-output/owner.json']),'Owner/result closeout hashes')
        need(r['source_tools_predecessors_protected_unchanged'] is True,'Recorded final source/tool/protected preservation')
        ids=[g['pid'] for g in owner['owned']]
        need(len(ids)==len(set(ids))==count and all(type(pid) is int and pid>0 for pid in ids),'Exact positive managed group inventory')
        need(r['groups']==close['groups']==ids and all(g['exit']==0 and g['cleanup_confirmed'] is True for g in owner['owned']),'Owned cleanup')
        observed=r['observed_pids'];wrapper=r['wrapper_pid']
        need(observed and len(observed)==len(set(observed)) and all(type(pid) is int and pid>0 for pid in observed) and set(ids)<=set(observed),'Nonempty observed identities')
        need(type(wrapper) is int and wrapper>0 and wrapper not in observed and wrapper==owner['wrapper_pid']==close['wrapper_pid'],'Distinct wrapper identity')
        need([a['pid'] for a in r['absence']]==sorted(observed) and all(a['pid_and_group_absent'] is True for a in r['absence']),'Exact observed absence')
        expected=set(observed)|{wrapper};need(len(close['absence'])==len(expected) and {a['pid'] for a in close['absence']}==expected and all(a['pid_and_group_absent'] is True for a in close['absence']),'Exact terminal absence including wrapper')
        for call in owner['calls']:
            need(call['exit']==0 and call['finished'] is True and call['cleanup_confirmed'] is True,'Managed call success')
            need(any(g['pid']==call['pid'] and g['argv']==call['argv'] for g in owner['owned']),'Managed call ownership')
            for stream in ('stdout','stderr'):need(call[stream]['sha256']==sha(records[phase+'-output/'+call[stream]['path']]),'Captured stream hash')
        need(r['target_peak']<=plan['target_cap'] and owner['peaks']['rss']<=plan['rss_cap'],'Original resource bounds')
    cpu_owner=js('cpu-output/owner.json');need(len(cpu_owner['calls'])==7,'Seven actual CPU commands')
    for index,key in [(0,'npm_install_argv'),(1,'frontend_build_argv'),(2,'typescript_smoke_argv'),(3,'build_argv'),(4,'test_build_argv'),(6,'lint_argv')]:need(cpu_owner['calls'][index]['argv']==plan[key],'Exact CPU route')
    tests=records['cpu-output/005.stdout.log'].decode();need('test result: ok. 8 passed; 0 failed; 0 ignored;' in tests,'Eight actual probe controls')
    for name in plan['test_names']:need(f'test {name} ... ok' in tests or f'test {name} - should panic ... ok' in tests,'Exact executed test')
    need(sha(records['cpu-output/frontend.json'])==cpu['frontend_sha256'],'Recorded installed/frontend identity list')
    proof=js('window-output/workflow/native-window.json');outcome=js('window-output/workflow/result.json')
    need(outcome['window']==proof and outcome['passed'] is True and outcome['actual_app'] is True and outcome['durable_journal_recovery'] is True and outcome['unrelated_authored_bytes_preserved'] is True,'Actual workflow completion')
    need(sha(records['window-output/workflow/result.json'])==window['proof_sha256'],'Workflow result binding')
    final_revision,final_asset=proof_check(proof)
    processes=js('window-output/workflow/processes.json');owned=js('window-output/owner.json')['owned']
    need(len(processes)==window['native_processes']==outcome['native_processes']==6,'Exactly six native processes')
    for row,group in zip(processes,owned):need(row['argv']==group['argv'] and row['exit_code']==0 and row['reaped'] is True,'Actual process/ledger ownership')
    need(processes[4]['argv'][0]==cpu['artifacts']['incant_editor']['copy'] and processes[4]['argv'][2]=='--animated-vertices-probe','Actual opted-in editor handler')
    need(all(processes[i]['argv'][0]==cpu['artifacts']['incant_headless']['copy'] for i in [0,1,2,3,5]),'Same normal headless author/reopen artifact')
    requests=[json.loads(processes[i]['input']) for i in [1,2,3,5]]
    need([r['method'] for r in requests]==['project.read','command.execute','project.save','project.read'],'Public authored route and fresh reopen')
    commands=requests[1]['params']['commands'];need([c['op'] for c in commands]==['write_script','upsert_script','create_entity','create_entity'],'Public fixture transactions')
    need('updates += 1' in commands[0]['source'] and '+=2*updates' in commands[0]['source'],'Persistent VM sentinel fixture')
    replies=[json.loads(processes[i]['stdout']) for i in [1,2,3,5]]
    need(all(reply['id']==request['id'] and 'error' not in reply for request,reply in zip(requests,replies)),'Exact RPC success identity')
    recovered=replies[-1]['result'];need(recovered['revision']==outcome['recovered_revision']==final_revision,'Fresh durable journal revision')
    assets=[a for a in recovered['project']['assets'].values() if a['path']=='refresh.gltf'];need(len(assets)==1 and assets[0]['sha256']==final_asset,'Fresh reopened accepted asset')
    fixture=js('window-output/workflow/fixture.json');need(fixture['fixture_authored_through_cmd'] is True,'Public authored fixture')
    for name,digest in fixture['inputs'].items():need(sha(records['window-output/workflow/'+name])==digest,'Unrelated authored bytes preserved')
    need(records['window-output/workflow/refresh.gltf']==records['window-output/workflow/refresh-replacement.gltf'],'Source recovery bytes')
    need(len(records['window-output/workflow/project.journal.jsonl'])>fixture['journal_bytes_before'],'Actual durable journal growth')
    need(abs(cpu['cumulative_seconds']-plan['prior_seconds']-cpu['seconds'])<1e-8 and abs(window['cumulative_seconds']-cpu['cumulative_seconds']-window['seconds'])<1e-8 and window['cumulative_seconds']<=6000,'Preserved cumulative account')
    failure=js('window-preflight-001-failure.json');need(failure['native_window_started'] is False,'Metadata-only preparation failure remains distinct')
    return {'passed':True,'records':len(records),'producer':HEAD,'cpu_tests':8,'actual_handler_stages':6,'durable_revision':final_revision,'excluded_binary_records':len(excluded),'native_reexecuted':False,'appearance_or_timing_claim':False}


if __name__=='__main__':
    records,manifest=load(Path(__file__).resolve().parent)
    print(json.dumps(validate(records,manifest),indent=2))
