#!/usr/bin/env python3
"""Replay saved host data only; no archived program, native player or capture runs."""
import argparse
import gzip
import hashlib
import importlib.util
import json
import math
from pathlib import Path
from analyze import calculate, require

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
BASE = '/Users/andreyyurlov/.codex/incant-evidence/native-host-stages-player-001/'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def file_digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for k,v in items:
            require(k not in result,'Duplicate JSON key')
            result[k]=v
        return result
    def invalid(_):
        raise ValueError('Nonfinite JSON')
    def finite(text):
        value=float(text);require(math.isfinite(value),'Nonfinite JSON number');return value
    return json.loads(raw,object_pairs_hook=pairs,parse_constant=invalid,parse_float=finite)


def load():
    manifest=decode((HERE/'manifest.json').read_bytes())
    compressed=(HERE/'records.json.gz').read_bytes()
    require(len(compressed)==manifest['archive_bytes']<=2*1024**2
            and digest(compressed)==manifest['archive_sha256'],'Archive identity')
    with gzip.open(HERE/'records.json.gz','rb') as stream:
        expanded=stream.read(16*1024**2+1)
    require(len(expanded)==manifest['expanded_bytes']<=16*1024**2,'Expanded archive bound')
    packet=decode(expanded)
    require(packet['format']=='incant-host-stage-complete-records-v1','Record format')
    records={}
    for row in packet['records']:
        require(row['path'] not in records,'Duplicate record')
        raw=row['text'].encode()
        require(len(raw)==row['bytes'] and digest(raw)==row['sha256'],'Saved record byte identity')
        records[row['path']]=row['text']
    require(len(records)==manifest['record_count']==42,'Exact record inventory')
    return manifest,records


def validate(manifest,records):
    text=lambda name:records[BASE+name]
    get=lambda name:decode(text(name))
    plan,result,child,close,grant=[get(n) for n in ['live-controller-003/plan.json','window-002/result.json',
        'window-002/child-result.json','window-002/agent-closeout.json','live-controller-003/root-grant.json']]
    require(manifest['source']==plan['source']==result['source']==close['source']
            =='e93ac30a9cce9deadd4013ac5fa7abbc3e60a2ac','Historical producing source')
    require(plan['player_sha256']==manifest['player_sha256']==close['player_sha256']
            =='45c5374583b7543af7e4b5d77a6776d79f7a5eeb3c945c9b640adf5b3a970ca6','Player identity')
    require(result['plan_sha256']==grant['plan_sha256']==digest(text('live-controller-003/plan.json').encode())
            and result['runner_sha256']==grant['runner_sha256']==digest(text('live-controller-003/run.py').encode()),'Execution grant binding')
    require(result['status']=='complete' and result['exit_code']==child['exit_code']==close['actual_wrapper_exit']==0
            and child['error'] is None and result['retries']==0,'Actual complete native result')
    require(child['cleanup_confirmed'] and not child['cleanup_errors'] and close['actual_exec_terminal_observed']
            and close['actual_exec_session']==4777 and close['positive_cleanup_confirmed']
            and close['owned_PIDs_and_groups']==[18203,18211] and close['matching_ps_rows']==[]
            and all(v=='ESRCH' for row in close['signal_zero_results'].values() for v in row.values()),'Positive process closure')
    require(close['result_sha256']==digest(text('window-002/result.json').encode())
            and close['child_result_sha256']==digest(text('window-002/child-result.json').encode()),'Closeout binding')
    require(child['elapsed_seconds']<=plan['child_seconds']==120
            and result['elapsed_seconds']<=plan['stage_seconds']==150,'Observed time bounds')
    require(not result['performance_acceptance'] and not result['display_acceptance']
            and not result['live_device_pcm_captured'] and not manifest['performance_gate_passed']
            and not manifest['display_cadence_measured'],'Unsupported gate/audio claim')
    old=get('live-controller-002/plan.json')
    require(text('live-controller-003/run.py')==text('live-controller-002/run.py')
            and [v.replace(plan['output'],old['output']) for v in plan['argv']]==old['argv'],'Same native payload/controller')
    for k in ['source','player_sha256','bundle_files','data_files','recording_sha256','child_seconds','stage_seconds',
              'output_caps','output_bytes','minimum_free_bytes','child_rss_bytes','retries']:
        require(plan[k]==old[k],'Unchanged scope/bounds: '+k)
    failed=get('window-001/result.json')
    require(failed['status']=='failed' and failed['exit_code']==1 and not failed['analysis']['host_timing_complete']
            and failed['analysis']['window_state_final']['host_steps']==263,'Prior partial failure preserved')
    for name,row in result['artifacts'].items():
        raw=text('window-002/'+name).encode()
        require(len(raw)==row['bytes'] and digest(raw)==row['sha256'],'Original native output identity: '+name)
    require(text('window-002/stderr.log')=='','Actual stderr')
    report=get('window-002/stdout.json')
    require(report['observations']==decode(records[plan['baseline_report']])['observations']
            and len(report['observations']['checks'])==8 and len(report['observations']['entities'])==17,
            'Full exact CPU gameplay observation parity')
    source=REPO/'tools/presentation_host_stages.py'
    require(file_digest(source)==manifest['host_analyzer_sha256']
            and file_digest(HERE/'analyze.py')==manifest['independent_analyzer_sha256'],'Reviewed analyzer identities')
    for path,expected in manifest['previous_packets'].items():
        require(file_digest(REPO/path)==expected,'Previous evidence packet changed')
    spec=importlib.util.spec_from_file_location('reviewed_host_analyzer',source)
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    capture=get('window-002/presentations.json');reported=get('window-002/host-stages.json')
    actual=module.analyze(capture);actual['input_sha256']=digest(text('window-002/presentations.json').encode())
    require(actual==reported,'Complete original analyzer replay')
    return calculate(capture,reported,report,get('window-002/callback-coverage.json'),get('window-002/compilations.json'),
                     [decode(line) for line in text('window-002/window-state.jsonl').splitlines()])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals',action='store_true')
    parser.add_argument('--binaries',action='store_true')
    args=parser.parse_args();manifest,records=load()
    summary=validate(manifest,records)
    require(summary==decode((HERE/'summary.json').read_bytes()),'Independent derived arithmetic')
    plan=decode(records[BASE+'live-controller-003/plan.json'])
    if args.originals:
        for path,text in records.items():
            require(Path(path).read_bytes()==text.encode(),'Original record differs: '+path)
        for row in plan['files']:
            path=Path(row['path'])
            require(path.stat().st_size==row['bytes'] and file_digest(path)==row['sha256'],'Frozen external input differs: '+str(path))
    if args.binaries:
        for root,key in [('bundle','bundle_files'),('data_root','data_files')]:
            for relative,row in plan[key].items():
                path=Path(plan[root])/relative
                require(path.stat().st_size==row['bytes'] and file_digest(path)==row['sha256'],'Retained native artifact differs')
    print('PASS: 42 exact records; complete 2156 historical host-stage route; independent arithmetic; prior263-step failure retained; no FPS/display/phase acceptance')


if __name__=='__main__':main()
