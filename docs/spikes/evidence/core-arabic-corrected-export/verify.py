#!/usr/bin/env python3
"""Replay retained Arabic export/CPU data. Never launches an engine or UI."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import math

HERE=Path(__file__).resolve().parent


def require(value,message):
    if not value:raise ValueError(message)


def sha(data):return hashlib.sha256(data).hexdigest()


def file_sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def exact_file(row):
    path=Path(row['path'])
    require(path.stat().st_size==row['bytes'] and file_sha(path)==row['sha256'],'Retained file changed: '+str(path))


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals',action='store_true')
    parser.add_argument('--binaries',action='store_true')
    parser.add_argument('--git-source')
    parser.add_argument('--source-root',type=Path)
    args=parser.parse_args()
    manifest=json.loads((HERE/'manifest.json').read_text())
    require(manifest['format']=='incant-core-arabic-corrected-export-v1','Known evidence format')
    packed=(HERE/'records.json.gz').read_bytes();require(sha(packed)==manifest['archive_sha256'],'Archive hash')
    with gzip.open(HERE/'records.json.gz','rb') as stream:raw=stream.read(8*1024**2+1)
    require(len(raw)<=8*1024**2,'Finite archive')
    data={}
    for row in json.loads(raw)['records']:
        name=row['name'];path=Path(name)
        require(not path.is_absolute() and '..' not in path.parts and name not in data,'Unique relative record')
        content=base64.b64decode(row['base64'],validate=True)
        require(len(content)==row['bytes'] and sha(content)==row['sha256'],'Record hash')
        data[name]=content
    require(len(data)==manifest['records'],'Record count')
    read=lambda name:json.loads(data[name])
    source=manifest['producing_source'];require(source=='069bbe54dc529511138867dfd4fbe50507a38b6b','Producing source remains historical')
    author=read('author/receipt.json');document=read('author/core.json')
    require(author['locale']=='ar' and author['state']=='playing' and author['revision']==32 and author['rpc_calls']==46
            and author['undo_redo_reopen'] is True and author['validation']['valid'] is True,'Public authoring/history/reopen')
    require(sha(data['author/core.json'])==manifest['project_sha256']==author['project_sha256']
            and document['settings']['localization']=={'locale':'ar','fallbacks':[],'pseudo':False},'Whole-scene Arabic identity')
    for health in range(4):
        require(author['localized_text']['ar'][f'health.{health}']=='\u200f'+' '.join(['●']*health+['○']*(3-health))+'\u200f','Exact resolved RLM health labels')
    exported=read('export/export.json');close=read('export-closeout-001.json');life=read('export/lifecycle.json')
    require(read('export/result.json')['passed'] is True and close['source']==source
            and exported['project_sha256']==manifest['project_sha256'] and exported['player_sha256']==manifest['player_sha256']
            and exported['mode']=='window' and exported['target']=='aarch64-apple-darwin'
            and exported['physics_mode']=='deterministic' and 'profiling' not in exported and 'shaders' not in exported,'Actual ordinary export')
    require(exported['startup_source_parsing'] is False and exported['build_storage_removed_before_startup'] is True
            and exported['startup']['tick']==1 and exported['startup']['frames']==800
            and exported['startup']['render_input']['gpu_submitted'] is False,'CPU-only one-tick startup/800PCMframes')
    require(len(life['calls'])==len(life['cleanup_records'])==14 and life['cleanup_confirmed'] is True
            and not life['cleanup_errors'] and all(row['exit']==0 for row in life['calls']),'Fourteen owned export calls')
    require(close['private_parent_now_empty'] is True and close['original_PIDs_independently_absent'] is True
            and close['actual_target_features']['incant_script']==['audio','physics-deterministic','ui']
            and close['separate_build_host_features']['incant_script']==['compiler','physics-deterministic'],'Target/runtime and host/compiler distinction')
    require(close['actual_player_features']==['native-window','physics-deterministic','project','render-gpu','render-input']
            and close['actual_target_features']['incant_profile']==[],'No selected profiling/observer/cooked mode')
    graph=data['export/normal-dependencies.txt'].decode()
    forbidden={'incant_cmd','incant_doc','incant_core','incant_cook','incant_agent','incant_agent_native','wasmtime'}
    require(not set(exported['normal_dependencies'])&forbidden,'Shipped normal dependency boundary')
    for line in graph.splitlines():
        require('|' in line,'Selected graph syntax')
        flags=line.removesuffix(' (*)').partition('|')[2].split(',')
        require(not any(f.startswith(('profile','diagnostic-','native-presentations','native-compilation','cooked-')) for f in flags),'Ordinary selected target graph')
    for name,flags in exported['codegen'].items():
        require(all(flags[k]==v for k,v in {'opt-level':'3','codegen-units':'1','panic':'abort','debuginfo':'2','split-debuginfo':'packed'}.items()),'Actual shipping codegen')
    require(exported['codegen']['incant_game_player']['lto']=='fat','Actual player LTO')
    require(read('baseline/result.json')['passed'] is True,'CPU trio result')
    manifest_routes=read('recordings/recordings.json');reports={}
    require(manifest_routes['locale']=='ar' and manifest_routes['route_changes'] is False,'Unchanged routes')
    for case,(name,host,tick) in zip(manifest_routes['cases'],[('lose-short',900,900),('win',2156,2156),('lose-restart-win',3056,2156)],strict=True):
        require((case['name'],case['host_steps'],case['simulation_tick'])==(name,host,tick),'Exact route clocks')
        report=read('baseline/'+name+'.json');reports[name]=report
        require(report==read('baseline/'+name+'.stdout.raw.log') and type(report['tick']) is int and report['tick']==tick
                and type(report['frames']) is int and report['frames']==host*800,'Raw CPU clocks')
        require(report['render_input']['gpu_submitted'] is False and report['render_input']['mode']=='cpu-only'
                and report['render_input']['ui']['widgets']==13 and report['render_input']['ui']['sources']==case['labels'],'Exact localized logical HUD')
        observed=report['observations'];plan=read('recordings/'+name+'-observe.json')
        require(observed['passed'] is True and set(observed['entities'])==set(plan['entities'])
                and len(observed['checks'])==len(plan['checks'])>0,'Complete observation membership')
        for actual,expected in zip(observed['checks'],sorted(plan['checks'],key=lambda c:(c['tick'],c['name'])),strict=True):
            require(all(actual[k]==expected[k] for k in ('name','tick','expect')) and actual['passed'] is True
                    and actual['actual'] is expected['expect']['value'],'Exact successful check outcomes')
        require(report['logs']['dropped']==0,'No dropped gameplay events')
        pickups=[r['tick'] for r in report['logs']['entries'] if r['message'].startswith('core.pickup ')]
        require(pickups==([] if name=='lose-short' else [127,508,1266,1842]),'Unchanged actual pickups')
    reset=reports['lose-restart-win'];win=reports['win']
    require(reset['session']=={'generation':2,'host_tick':3056,'simulation_tick':2156,'restarts':[{'host_step':900,'completed_generation':1,'simulation_tick':900,'output_frames':'720000'}]}
            and all(reset[k]==win[k] for k in ('logs','render_input','flipbooks','sources'))
            and reset['observations']['entities']==win['observations']['entities'],'Restart/fresh-win parity')
    cpu=read('baseline/lifecycle.json');require(cpu['cleanup_confirmed'] is True and len(cpu['cleanup_records'])==len(cpu['calls'])==3 and not cpu['cleanup_errors'],'Three observed CPU cleanups')
    locked=read('locked-pseudo/closeout.json');failed=read('locked-pseudo/supervisor-result.json')
    require(failed['passed'] is False and 'TimeoutExpired' in failed['error'],'Latest actual pseudo timeout remains failed')
    require(all(v['value'] is True for v in read('locked-pseudo/os-lock-metadata.json')['observed']),'Actual OS-lock limitation')
    dormant=read('dormant-window/window-proposal-001.json')
    require(dormant['state'].startswith('dormant-unexecuted') and dormant['stop_after']==3056
            and dormant['native_child_seconds']==120 and dormant['bounds']['seconds']==180
            and dormant['appearance_or_gate_already_passed'] is False,'Dormant window only')
    require(b'Ran 5 tests' in data['dormant-window/lock-controls-final.raw.log'] and data['dormant-window/lock-controls-final.raw.log'].rstrip().endswith(b'OK'),'Five pure lock-metadata controls')
    if args.originals:
        for row in manifest['originals']:exact_file(row)
        for row in manifest['retained_external']:
            if row['kind']=='large-log':exact_file(row)
    if args.binaries:
        for row in manifest['retained_external']:
            if row['kind']=='runtime-or-PCM':exact_file(row)
        pcms={name:Path(next(r['path'] for r in manifest['retained_external'] if r['path'].endswith('/'+name+'.pcm'))).read_bytes() for name in reports}
        for name,host in [('lose-short',900),('win',2156),('lose-restart-win',3056)]:
            require(len(pcms[name])==host*6400 and sha(pcms[name])==reports[name]['pcm_sha256']
                    and all(math.isfinite(v[0]) for v in struct.iter_unpack('<f',pcms[name])),'Exact finite PCM')
        require(pcms['lose-restart-win']==pcms['lose-short']+pcms['win'],'Actual PCM concatenation')
    if args.git_source or args.source_root:
        require(args.source_root is not None,'Git checks require a local repository --source-root')
        ref=args.git_source or source
        for row in read('source-freeze-001.json')['records']:
            require(not Path(row['path']).is_absolute() and '..' not in Path(row['path']).parts,'Relative source identity')
            actual=subprocess.check_output(['git','-C',str(args.source_root),'show',ref+':'+row['path']])
            require(sha(actual)==row['sha256'],'Producing Git source differs: '+row['path'])
            if args.source_root and args.git_source is None:
                require(file_sha(args.source_root/row['path'])==row['sha256'],'Live producing input differs: '+row['path'])
    print(f"PASS {len(data)} records; ordinary export;900/2156/3056CPU; locale appearance remains open")


if __name__=='__main__':main()
