#!/usr/bin/env python3
"""Replay saved build/CPU evidence; no compiler, player or GPU execution."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess
import threading


def sha(data):return hashlib.sha256(data).hexdigest()

def require(value,message):
    if not value:raise ValueError(message)

def file_sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def git_check(root,revision,source):
    # Stream the full1.57GB producing tree instead of capturing it in memory.
    process=subprocess.Popen(['git','cat-file','--batch'],cwd=root,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
    timer=threading.Timer(120,process.kill);timer.start()
    try:
        for path,expected in source.items():
            process.stdin.write((revision+':'+path+'\n').encode());process.stdin.flush()
            header=process.stdout.readline().split()
            require(len(header)==3 and header[1]==b'blob','Missing Git input: '+path)
            remaining=int(header[2]);digest=hashlib.sha256()
            while remaining:
                value=process.stdout.read(min(remaining,65536));require(value,'Truncated Git blob')
                digest.update(value);remaining-=len(value)
            require(process.stdout.read(1)==b'\n' and digest.hexdigest()==expected,'Git source differs: '+path)
        process.stdin.close();require(process.wait(timeout=10)==0,'Git verifier failed')
    finally:
        timer.cancel()
        if process.poll() is None:process.kill();process.wait(timeout=10)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals',action='store_true')
    parser.add_argument('--binaries',action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root',type=Path,help='Strict full producing tree only; later docs/source compositions may differ')
    args=parser.parse_args();here=Path(__file__).resolve().parent
    root=args.source_root.resolve() if args.source_root else here.parents[3]
    manifest=json.loads((here/'manifest.json').read_text())
    zipped=(here/'records.json.gz').read_bytes();require(sha(zipped)==manifest['archive_sha256'],'Archive changed')
    with gzip.open(here/'records.json.gz','rb') as stream:payload=stream.read(32*1024**2+1)
    require(len(payload)==manifest['decoded_archive_bytes']<=32*1024**2,'Archive bound')
    encoded=json.loads(payload);require(set(encoded)=={r['key'] for r in manifest['records']},'Record membership')
    raw={}
    for row in manifest['records']:
        value=base64.b64decode(encoded[row['key']],validate=True)
        require(len(value)==row['bytes'] and sha(value)==row['sha256'],'Record changed: '+row['key'])
        raw[row['key']]=value
        if args.originals:require(Path(row['original']).read_bytes()==value,'Original record changed: '+row['key'])
    def data(key):return json.loads(raw[key])
    producer=data('producer-summary.json');session=data('build-006/session.json')
    require(producer['source']==manifest['producer']==session['source_head']=='50b5408b1d6be66dbb1dcbca501d4380add9d656','Producer changed')
    sources=session['source_sha256'];require(len(sources)==producer['full_source_files']==22091,'Source inventory')
    require(producer['tree']==manifest['producer_tree'],'Producing tree')
    for row in manifest['records']:
        if 'source_path' in row:require(sha(raw[row['key']])==sources[row['source_path']],'Changed-source bytes')
    if args.source_root:
        for path,value in sources.items():require(file_sha(root/path)==value,'Live producing source differs: '+path)
    if args.git_source:git_check(root,args.git_source,sources)
    for enabled,key in [(args.originals,'original_files'),(args.binaries,'binary_and_data_files')]:
        if enabled:
            for row in manifest[key]:
                path=Path(row['path']);require(path.stat().st_size==row['bytes'] and file_sha(path)==row['sha256'],'External file changed: '+str(path))
    require(re.search(rb'Ran 57 tests .*\n\nOK',raw['portable-controls-001.log']),'57portable source controls')
    failed_messages=['sdk_version','Pinned compiler','Linked package input','Single log cap','Combined log cap']
    for i,message in enumerate(failed_messages,1):
        result=data(f'managed-build-{i:03}/result.json');calls=data(f'managed-build-{i:03}/managed-calls.json')
        require(result['passed'] is False and result['cleanup_confirmed'] is True and message in result['error'],'Historical failure missing')
        require(all(r['cleanup_confirmed'] for r in calls),'Historical cleanup unconfirmed')
        require(not any('--package' in c['argv'] for c in calls),'Failed attempt had unexpected player startup')
        if i<=4:require(not any('build' in c['argv'] and '--profile' in c['argv'] for c in calls),'Early preflight unexpectedly built')
    negative=data('managed-build-005/result.json');storage=data('managed-build-005/storage.json')
    require(max(r['log_bytes'] for r in storage)==177561451>128*1024**2 and not negative['passed'],'Original aggregate refusal lost')
    plan=data('build-proposal-008.json');result=data('managed-build-006/result.json');calls=data('managed-build-006/managed-calls.json')
    require(result['passed'] is True and result['cleanup_confirmed'] is True and result['CPU_startups']==4 and len(calls)==result['managed_calls']==91,'Actual build/startup result')
    require(result['source']==producer['source'] and result['proposal_sha256']==sha(raw['build-proposal-008.json'])
            and result['supervisor_sha256']==sha(raw['build-supervisor-008.py']),'Execution binding')
    require(plan['bounds']=={'seconds':2860,'floor':6*1024**3,'scratch_cap':8*1024**3,'output_cap':3*1024**3,'log_cap':1024**3,'file_cap':64*1024**2}
            and plan['prior_usage']['charged_seconds']==740,'Cumulative allocation changed')
    require(all(c['exit_code']==0 and c['call_finished'] and c['native_process_returned'] and c['cleanup_confirmed'] for c in calls),'Managed lifecycle')
    close=data('build-closeout-006.json');require(close['all_owned_PID_groups_absent'] and close['wrapper_exit']==0 and close['wrapper_terminal_observed'],'Wrapper closure')
    require(close['actual_ps_matches']==[] and close['CPU_startups']==4 and close['GPU_window_capture_executions']==0,'Actual absence/scope')
    require(session['build_storage_removed_before_any_player'] and session['authoring_inputs_removed_before_any_player']
            and len(session['builds'])==4 and len(session['retired_player_units'])==6,'Deletion/retired history')
    batch=data('build-006/manifest.json');require(batch['complete'] is True and batch['GPU_window_capture_executions']==0,'Complete CPU batch')
    all_units=session['units']+session['retired_player_units'];origins={}
    for unit in all_units:
        identity=sha(json.dumps(unit['identity'],sort_keys=True).encode());key=(identity,unit['build'])
        require(key not in origins and unit['hashes'] and unit['command'].startswith('Running `'),'Immutable origin')
        origins[key]=unit
    names=['Core-en','zh-Hans','ar','en-XA']
    for i,name in enumerate(names):
        receipt=data(f'build-006/{name}/export.json');build=session['builds'][i];rows=build['target_units']
        require(receipt['shipping_session_target_units']==rows and len(rows)==266,'Actual target-unit rows')
        require(sum(not r['reused'] for r in rows)==[266,32,2,2][i],'Fresh/reused units')
        for row in rows:
            require((row['identity_sha256'],row['fresh_build']) in origins and row['fresh_build']<=i,'Unproven reuse')
            require((row['fresh_build']<i)==row['reused'],'Reuse chronology')
        player=[r for r in rows if r['crate']=='incant_game_player']
        require(len(player)==2 and all(not r['reused'] and r['fresh_build']==i for r in player),'Fresh scene player missing')
        require(receipt['startup']['tick']==1 and receipt['startup']['frames']==800 and receipt['startup']['render_input']['gpu_submitted'] is False
                and receipt['build_storage_removed_before_startup'] and receipt['startup_source_parsing'] is False,'Standalone CPU startup')
        expected='project,native-window'+(',physics-deterministic' if i else '')
        command=build['command'];require(command[command.index('--features')+1]==expected and command[command.index('--profile')+1]=='shipping','Actual selected graph/profile')
        require(receipt['physics_mode']==('deterministic' if i else 'fast'),'Authored physics mode')
        require(receipt['project_sha256']==batch['jobs'][i]['project_sha256'] and receipt['player_sha256']==batch['jobs'][i]['player_sha256'],'Separate package binding')
    cpu=data('baselines-001/result.json');life=data('baselines-001/lifecycle.json');cpu_close=data('baseline-closeout-001.json')
    require(cpu['passed'] is True and len(cpu['cases'])==5 and len(life['calls'])==5 and cpu_close['positive_owned_group_absence'],'Full CPU closure')
    require(cpu['proposal_sha256']==sha(raw['baseline-proposal-001.json']) and cpu_close['full_PCM_retained_bytes']==92032000,'Full CPU protocol/PCM')
    require(all(c['stop_returned'] and c['call_returned'] and c['pid_group_absent'] and c['returncode']==0 for c in life['calls']),'CPU child cleanup')
    for case in cpu['cases']:
        name=case['name'];report=data(f'baselines-001/{name}/report.json')
        require(report['tick']==2156 and report['frames']==(2156 if name=='Core-en' else 3056)*800
                and 'frame_capture' not in report and report['render_input']['gpu_submitted'] is False,'CPU-only full-route clocks')
        require(report['pcm_sha256']==case['check']['pcm_sha256'] and report['render_input']['ui']['widgets']==13,'CPU PCM/HUD')
        if name!='Core-en':require(report['session']['generation']==2 and report['session']['restarts'][0]['host_step']==900 and report['observations']['passed'] is True,'Restart/observations')
    print(f'PASS {len(raw)}records;22091sourceinputs;57portablecontrols;5retained failures;4freshsceneplayers/4source-deletedCPUstartups;5fullCPUroutes;noGPU/window/appearance/performanceclaim')


if __name__=='__main__':main()
