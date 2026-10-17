#!/usr/bin/env python3
"""Bounded saved-data replay. Never executes archived source, player or native code."""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
SOURCE = 'a621ef024c6012e3ab9e43be39afc501ea57f771'
MIB = 1024**2


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for k, v in items:
            require(k not in result, 'Duplicate JSON key')
            result[k] = v
        return result
    def invalid(_):
        raise ValueError('Nonfinite JSON')
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def bounded(path, cap):
    with Path(path).open('rb') as f:
        raw = f.read(cap + 1)
    require(len(raw) <= cap, 'Input byte cap')
    return raw


def file_identity(row):
    path = Path(row['path'])
    require(path.stat().st_size == row['bytes'], 'Original file size: ' + str(path))
    with path.open('rb') as f:
        require(hashlib.file_digest(f, 'sha256').hexdigest() == row['sha256'], 'Original file hash: ' + str(path))


def load():
    manifest = decode(bounded(HERE / 'manifest.json', 256 * 1024))
    packed = bounded(HERE / 'records.json.gz', 2 * MIB)
    require(len(packed) == manifest['archive_bytes'] and digest(packed) == manifest['archive_sha256'], 'Archive identity')
    with gzip.GzipFile(fileobj=io.BytesIO(packed), mode='rb') as f:
        raw = f.read(8 * MIB + 1)
    require(len(raw) == manifest['expanded_bytes'] <= 8 * MIB, 'Expanded byte cap')
    packet = decode(raw)
    require(packet['format'] == 'incant-wake-acquisition-live-records-v1', 'Packet format')
    records, originals = {}, {}
    for row in packet['records']:
        name = row['name'];body = row['text'].encode()
        require(name not in records and len(body) == row['bytes'] and digest(body) == row['sha256'], 'Record identity')
        records[name] = body;originals[row['original']] = body
    require(len(records) == manifest['record_count'], 'Record count')
    return manifest, records, originals


def audio(value):
    require(type(value) is dict and set(value) == {'scope', 'diagnostics', 'glitch_free_output_verified'}, 'Audio report fields')
    require(value['scope'] == 'current-session-owner' and value['glitch_free_output_verified'] is False, 'Audio scope')
    d = value['diagnostics']
    require(type(d) is dict and set(d) == {'xrun_notifications', 'route_change_notifications', 'poll_limit_hits', 'counters_saturated'}, 'Audio diagnostics fields')
    for key in ('xrun_notifications', 'route_change_notifications', 'poll_limit_hits'):
        require(type(d[key]) is int and 0 <= d[key] < 2**64, 'Audio counter')
    require(type(d['counters_saturated']) is bool, 'Audio saturation type')


def validate(m, records):
    get = lambda name: decode(records[name])
    require(m['source'] == SOURCE and m['source_tree'] == '612fd7d39d469c59278ec52e878f21805c7e98c8', 'Producing source')
    require(m['native_runs'] == 2 and m['actual_native_child_exit'] == 0 and m['original_native_wrapper_status'] == 'failed'
            and m['saved_data_replay'] == 'passed', 'Native failure/saved replay distinction')
    require(all(m[k] is False for k in ('continuous_display_qualified', 'performance_gate_passed', 'pacing_cause_determined', 'audible_output_verified')), 'Unsupported gate claim')
    plan = get('packet-001/plan.json');build = get('build-001/metadata/result.json');close = get('build001-closeout.json')
    require(plan['source'] == build['source'] == close['source'] == SOURCE and build['status'] == 'complete'
            and build['exit'] == 0 and build['supervisor_terminal'], 'Fresh build result')
    require(build['plan_sha256'] == digest(records['packet-001/plan.json'])
            and build['supervisor_sha256'] == digest(records['packet-001/build.py']), 'Build protocol')
    require(close['all_owned_absent'] and close['all_managed_exit0_and_cleanup_confirmed']
            and close['managed_calls'] == 71 and close['scratch_absent']
            and close['owned_build_input_storage_deleted_before_startup'], 'Build lifecycle')
    calls = get('build-001/metadata/managed-calls.json')
    require(len(calls) == 71 and all(r['exit_code'] == 0 and r['cleanup_confirmed'] and r['call_finished']
            and not r['cleanup_failures'] for r in calls), 'Managed build calls')
    require(close['public_imports'] == 14 and close['expected_cooked_hashes_verified'] == 23
            and close['project_bytes_unchanged'], 'Real import regeneration')
    imports = get('build-001/metadata/import-results.json')
    require(len(imports) == 14 and all(r['changed'] is False for r in imports), 'Importer publication invariants')
    for peak, cap in [('scratch_bytes', 'scratch_cap'), ('package_bytes', 'output_cap'), ('log_bytes', 'log_cap')]:
        require(close['storage_peaks'][peak] <= close['storage_cap'][cap], 'Build storage cap')
    require(close['cumulative_new_sequence_charge_seconds'] <= close['storage_cap']['seconds'] == 3000
            and close['startup_tick'] == 1 and close['startup_PCM_sample_frames'] == 800
            and close['startup_gpu_submitted'] is False, 'Build/startup bounds')
    export = get('build-001/package/diagnostic-build.json')
    require(export['source'] == SOURCE and export['player_sha256'] == m['player_sha256']
            and close['new_shipping_player_sha256'] == m['player_sha256']
            and export['startup_source_parsing'] is False and export['diagnostic_only']
            and export['eligible_for_performance_gate'] is False and export['eligible_for_distribution'] is False
            and export['build_and_input_storage_deleted_before_startup'], 'Shipping diagnostic identity')
    require(set(export['observer_feature_delta']) == {'incant_game_player', 'incant_render', 'wgpu-hal', 'objc2-metal'}
            and export['compatibility']['compiler_and_payload_equal'], 'Actual observer/compiler audit')
    for row in export['codegen'].values():
        require(row['opt-level'] == '3' and row['codegen-units'] == '1' and row['panic'] == 'abort', 'Shipping codegen')
    require(export['codegen']['incant_game_player']['lto'] == 'fat', 'Player LTO')
    require(get('build001-preflight-refusal.json')['engine_compiler_or_player_children'] == 0, 'Initial setup refusal')
    cpu = get('functional-001/result.json');cpu_close = get('functional001-closeout.json')
    offline = get('functional-001/artifacts/stdout.json');reference = get('reference/full-CPU-report.json')
    require(cpu['status'] == 'complete' and cpu['exit'] == 0 and cpu['child_exit'] == 0
            and cpu['exact_entire_logical_report'] and cpu['exact_full_PCM'] and not cpu['scratch_retained']
            and cpu_close['all_owned_absent'] and cpu_close['entire_logical_report_equal'] and offline == reference, 'Full CPU logical parity')
    require(cpu['artifacts']['output.pcm']['sha256'] == m['offline_PCM_sha256'] == cpu_close['PCM']['sha256']
            and cpu_close['PCM']['bytes'] == 13798400, 'Whole offline PCM identity')
    native = get('window-001/stdout.json');child = get('window-001/child-result.json');failure = get('window-001/result.json')
    nc = get('window001-closeout.json');replay = get('saved-reanalysis-001/result.json')
    require(child['exit_code'] == 0 and child['cleanup_confirmed'] and child['error'] is None
            and not child['cleanup_errors'] and nc['all_owned_pids_and_groups_absent'], 'Actual child cleanup')
    require(failure['status'] == 'failed' and failure['exit_code'] == 1
            and failure['analysis']['errors'] == {'report': "ValueError('Unknown audio diagnostics')"}
            and replay['native_run_result_remains_failed'] and replay['native_runs_or_builds_in_this_reanalysis'] == 0, 'Original wrapper failure retained')
    require(replay['status'] == 'saved-data-replay-passed' and replay['controls']['tests'] == 7, 'Saved replay controls')
    require(native['mode'] == 'native-window' and native['backend'] == 'Metal' and native['on_screen'] is True
            and native['host_steps'] == native['tick'] == 2156 and native['accepted_frames'] >= 2156
            and native['profile']['completed_frames'] == 2156 and native['profile']['failed_frames'] == 0, 'Full native gameplay')
    require(native['observations'] == offline['observations'] and native['observations']['passed']
            and len(native['observations']['checks']) == 8 and len(native['observations']['entities']) == 17, 'Native/CPU exact gameplay')
    require(native['input_recording']['sha256'] == digest(records['reference/input.json'])
            and native['input_recording']['completed'] is True
            and native['input_recording']['physical_devices'] is False
            and native['input_recording']['native_focus_and_surface_required'] is True
            and native['ui']['widgets'] == 7 and native['ui']['glyph_quads'] > 0, 'Input/HUD route')
    audio(native['audio_device'])
    # Use only trusted, checked-in readers. Never import/execute archived text.
    for row in m['analyzers']:
        require(digest(bounded(REPO / row['repo_path'], MIB)) == row['sha256'], 'Trusted reader changed')
    sys.path.insert(0, str(REPO / 'tools'))
    import presentation_boundaries as boundary
    raw = get('window-001/presentations.json');b = boundary.analyze(raw)
    require(b['complete_boundary_attribution'] and b['host_analysis']['complete_requested_host_timing']
            and not b['host_analysis']['partial_reasons'], 'Complete raw v2 attribution')
    require(b['wake_states'] == replay['wake_states'] and b['acquisition_states'] == replay['acquisition_states'], 'Wake/acquisition coverage')
    for key in ('wake', 'reservation'):
        tail = b['paired_largest_5percent_' + key + '_tail']
        require(tail['sums'] == replay['same_frame_tail_partitions'][key]['sums_ns']
                and tail['selected_samples'] == 108, 'Same-frame tail decomposition')
    for key in ('pre_attempt', 'reserve', 'committed_body', 'post_commit_haptics', 'finished_total'):
        s=b['host_analysis']['summary'][key];r=replay['independent_raw_arithmetic'][key]
        require(s['valid_subset_sum_ns'] == r['sum_ns'] and s['valid_subset_p95_nearest_rank_ns'] == r['p95_nearest_rank_ns'], 'Raw host arithmetic')
    context=get('saved-reanalysis-001/callback-context.json')
    zero=[r for r in raw['rows'] if r['outcome']=='reported_zero']
    require(zero == context['zero_timestamp_rows'] and [r['requested_host_step'] for r in zero] == [0,376,639,640,641], 'Zero timestamp scope')
    positive=[]
    for before,after in zip(raw['rows'],raw['rows'][1:]):
        if before['outcome']==after['outcome']=='presented' and before['segment']==after['segment']:
            positive.append((before,after,after['presented_seconds']-before['presented_seconds']))
    require(len(positive)==context['positive_adjacent_intervals']==2148 and all(v>0 for _,_,v in positive), 'Positive adjacent subset')
    counts={}
    for _,_,v in positive:
        q=str(round(v*120));counts[q]=counts.get(q,0)+1
    require(counts == context['empirical_8_333333ms_quantum_counts'] == {'1':28,'2':2095,'3':25}, 'Empirical quantum counts')
    require(len(context['continuity_breaks'])==8 and replay['focus_suspended_total_us']==6719110, 'Focus breaks remain visible')
    # Separately granted same-player run002: no focus interruption or gameplay zero.
    quiet_plan=get('live-packet-002/plan.json');quiet_grant=get('live-packet-002/root-grant.json')
    quiet=get('window-002/result.json');qc=get('window-002/child-result.json');qclose=get('window002-closeout.json')
    qp=get('window-002/stdout.json');qr=get('saved-analysis-002/result.json')
    previous_plan=get('live-packet-001/plan.json')
    require(quiet_plan['source']==SOURCE and quiet_plan['player_sha256']==m['player_sha256']
            and quiet_plan['argv']==[v.replace('/window-001/','/window-002/') for v in previous_plan['argv']]
            and quiet_plan['child_seconds']==120 and quiet_plan['stage_seconds']==150, 'Same-player quiet route')
    require(quiet['plan_sha256']==quiet_grant['plan_sha256']==digest(records['live-packet-002/plan.json'])
            and quiet['runner_sha256']==quiet_grant['runner_sha256']==digest(records['live-packet-002/run.py']), 'Quiet exact grant')
    require(m['uninterrupted_native_wrapper_status']=='complete' and quiet['status']=='complete' and quiet['exit_code']==0
            and qc['exit_code']==0 and qc['error'] is None and qc['cleanup_confirmed'] and not qc['cleanup_errors']
            and qclose['all_owned_pids_and_groups_absent'], 'Quiet terminal/cleanup')
    require(not quiet['performance_acceptance'] and not quiet['display_acceptance'] and not quiet['live_device_pcm_captured']
            and quiet['analysis']['focus_lost_events']==0 and quiet['analysis']['gameplay_reported_zero_timestamps']==0
            and not quiet['analysis']['errors'], 'Quiet admission/scope')
    require(qp['host_steps']==qp['tick']==2156 and qp['observations']==offline['observations']
            and qp['input_recording']['sha256']==native['input_recording']['sha256']
            and qp['ui']['widgets']==7 and qp['backend']=='Metal' and qp['on_screen'] is True, 'Quiet gameplay/input/HUD')
    audio(qp['audio_device'])
    qraw=get('window-002/presentations.json');qb=boundary.analyze(qraw)
    require(qb['complete_boundary_attribution'] and qb['host_analysis']['complete_requested_host_timing']
            and qb['host_analysis']['callback_coverage']['presented']==m['uninterrupted_gameplay_callback_coverage']==2156
            and qb['host_analysis']['callback_coverage']['zero']==1, 'Quiet raw coverage')
    qzero=[r for r in qraw['rows'] if r['outcome']=='reported_zero']
    require(len(qzero)==1 and qzero[0]['requested_host_step']==0, 'Only initial zero retained')
    for key in ('wake','reservation'):
        require(qb['paired_largest_5percent_'+key+'_tail']['sums']==qr['same_frame_tail_partitions'][key]['sums_ns'], 'Quiet paired sums')
    for key in ('pre_attempt','reserve','committed_body','post_commit_haptics','finished_total'):
        x=qb['host_analysis']['summary'][key];y=qr['independent_raw_arithmetic'][key]
        require(x['valid_subset_sum_ns']==y['sum_ns'] and x['valid_subset_p95_nearest_rank_ns']==y['p95_nearest_rank_ns'], 'Quiet raw arithmetic')
    qb['input_sha256']=digest(records['window-002/presentations.json'])
    encoded=json.dumps(qb,allow_nan=False,separators=(',',':')).encode()+b'\n'
    require(digest(encoded)==quiet['artifacts']['boundaries.json']['sha256'], 'Exact quiet derived capture')
    qcontext=get('saved-analysis-002/callback-context.json')
    intervals=[]
    for before,after in zip(qraw['rows'],qraw['rows'][1:]):
        if before['outcome']==after['outcome']=='presented' and before['segment']==after['segment']:
            intervals.append((before['drawable_id'],after['drawable_id'],after['presented_seconds']-before['presented_seconds']))
    qcounts={}
    for _,_,v in intervals:
        q=str(round(v*120));qcounts[q]=qcounts.get(q,0)+1
    require(len(intervals)==2155 and qcounts==qcontext['empirical_8_333333ms_quantum_counts']=={'1':30,'2':2097,'3':28}, 'Quiet presentation quantization')
    following={before:(after,v) for before,after,v in intervals}
    pairs=sum(v>.024 and after in following and following[after][1]<.009 for _,after,v in intervals)
    require(pairs==qcontext['long_followed_by_sub_9ms_count']==27 and len(qcontext['continuity_breaks'])==1, 'Quiet long/short pairs')
    qstates=[decode(line) for line in records['window-002/window-state.jsonl'].splitlines()]
    require(all(x['focus_lost_events']==0 for x in qstates if x['kind']=='snapshot')
            and qstates[-1]['host_steps']==2156 and qstates[-1]['returned_report'] is True, 'Actual quiet focus state')
    source=get('packet-001/source-manifest.json')
    require(source['source']==SOURCE and len(source['files'])==m['source_inventory_records']==4725, 'Source inventory')
    return source


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--originals', action='store_true');p.add_argument('--binaries', action='store_true')
    p.add_argument('--git-source', metavar='REV');p.add_argument('--source-root', type=Path)
    a=p.parse_args();m,records,originals=load();source=validate(m,records)
    if a.originals:
        for path,raw in originals.items():
            require(bounded(path,len(raw))==raw, 'Original changed: '+path)
        for row in m['external_build_logs']:file_identity(row)
    if a.binaries:
        for row in m['external_artifacts']:file_identity(row)
    if a.git_source:
        rev=subprocess.check_output(['git','rev-parse',a.git_source],cwd=REPO,text=True).strip()
        require(rev==SOURCE, 'Use actual historical producing source')
        requests=''.join(rev+':'+name+'\n' for name in source['files']).encode()
        raw=subprocess.run(['git','cat-file','--batch'],input=requests,cwd=REPO,stdout=subprocess.PIPE,check=True,timeout=60).stdout
        offset=0
        for name,want in source['files'].items():
            end=raw.index(b'\n',offset);header=raw[offset:end].split()
            require(len(header)==3 and header[1]==b'blob','Missing Git input')
            size=int(header[2]);body=raw[end+1:end+1+size]
            require(digest(body)==want,'Git input changed: '+name);offset=end+size+2
        require(offset==len(raw),'Trailing Git response')
    if a.source_root:
        for name,want in source['files'].items():
            if not name.startswith('docs/') and name not in ('STATUS.md','AGENTS.md','CLAUDE.md'):
                with (a.source_root/name).open('rb') as f:
                    require(hashlib.file_digest(f,'sha256').hexdigest()==want,'Current producing input differs: '+name)
    print(f"PASS: {len(records)} records; fresh build +2156 CPU steps; interrupted001 retained +uninterrupted002; strict raw v2 replay; no FPS gate")


if __name__=='__main__':
    main()
