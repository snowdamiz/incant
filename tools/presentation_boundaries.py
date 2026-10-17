#!/usr/bin/env python3
"""Strict v2 wake/acquisition attribution. Saved data only; no native execution.

The v1 analyzer and its archived evidence remain byte-identical. Durations here
are observer-inclusive wall spans, never displayed FPS, CPU savings or causality.
"""
import argparse
import copy
import json
from pathlib import Path
import sys
import presentation_host_stages as v1

CONTRACT = {**v1.CONTRACT, 'version':2,
    'wake':'NewEvents arrival consumed once by MainEventsCleared; missing/overwritten/deadline mismatch is not a timer verdict',
    'acquisition':'existing get_current_texture call only, before unbind and view creation; not GPU completion'}
LIMITS = {**v1.LIMITS, 'attempt_bytes_max':512}
CAUSES = ('unknown','init','poll','wait_cancelled','resume_time_reached')
ACQUISITIONS = ('success','suboptimal','timeout','occluded','outdated','lost','validation')
SUCCESS = ('success','suboptimal')
require, keys, uint = v1.require, v1.keys, v1.uint


def project(data):
    keys(data,'format version diagnostic_only display_gate uncapped_gpu_throughput final_process_exit_status source_identity per_present_registration_overhead snapshot_after_gameplay requested_host_steps capture_capacity limits host_timing_contract coverage boundary_bit_order failure_index_order host rows finalization wake_tracking','v2 capture')
    v1.exact(data['source_identity'],'requires external executable/source/package binding','source_identity')
    v1.exact(data['per_present_registration_overhead'],True,'per_present_registration_overhead')
    v1.exact(data['snapshot_after_gameplay'],True,'snapshot_after_gameplay')
    v1.exact(data.get('version'),2,'version')
    v1.exact(data.get('host_timing_contract'),CONTRACT,'host_timing_contract')
    v1.exact(data.get('limits'),LIMITS,'limits')
    counts=data['wake_tracking']
    keys(counts,'events pulses superseded_events pulses_without_event overflow','wake_tracking')
    for name in ('events','pulses','superseded_events','pulses_without_event'):
        uint(counts[name],name)
    require(type(counts['overflow']) is bool,'wake overflow must be boolean')
    require(counts['superseded_events']<=counts['events'] and counts['pulses_without_event']<=counts['pulses'],'Wake counter ordering')
    projected=copy.deepcopy(data)
    projected['version']=1;projected['host_timing_contract']=dict(v1.CONTRACT);projected['limits']=dict(v1.LIMITS)
    projected.pop('wake_tracking')
    for original,clean in zip(data['host']['attempts'],projected['host']['attempts']):
        t=original['host_timing']
        keys(t,'began_ns reserved_ns completed_ns finished_ns schedule wake acquisitions','v2 host_timing')
        wake=t['wake']
        if wake is not None:
            keys(wake,'sequence pulse_ns events_since_previous_pulse event','pulse wake')
            uint(wake['sequence'],'pulse sequence',counts['pulses'])
            require(wake['sequence']>0,'Zero pulse sequence')
            uint(wake['pulse_ns'],'wake pulse',nullable=True)
            uint(wake['events_since_previous_pulse'],'events per pulse',counts['events'])
            event=wake['event']
            if event is not None:
                keys(event,'sequence cause arrived_ns requested_resume_supplied requested_resume_ns','wake event')
                uint(event['sequence'],'event sequence',counts['events'])
                require(event['sequence']>0 and event['cause'] in CAUSES,'Wake cause/sequence')
                for name in ('arrived_ns','requested_resume_ns'):uint(event[name],name,nullable=True)
                require(type(event['requested_resume_supplied']) is bool,'Requested-resume presence')
        calls=t['acquisitions'];v1.array(calls,2,'acquisitions');require(len(calls)==2,'Exactly two fixed acquisition slots')
        for call in calls:
            if call is None:continue
            keys(call,'began_ns ended_ns outcome','acquisition')
            for name in ('began_ns','ended_ns'):uint(call[name],name,nullable=True)
            require(call['outcome'] is None or call['outcome'] in ACQUISITIONS,'Acquisition outcome')
        clean['host_timing'].pop('wake');clean['host_timing'].pop('acquisitions')
    return projected


def wake_metrics(t,forced):
    absent={'state':'not_applicable' if forced else 'missing','before_new_events_ns':None,'after_new_events_ns':None}
    w=t['wake'];s=t['schedule']
    if w is not None and w['event'] is not None and w['event']['cause']=='unknown':
        return {**absent,'state':'invalid','reason':'unsupported_wake_cause'}
    if forced:return {**absent,'reason':'forced_resume'}
    if w is None:return {**absent,'reason':'missing_pulse_binding'}
    if s is None or w['pulse_ns']!=s['pulse_ns']:return {**absent,'state':'invalid','reason':'pulse_mismatch'}
    if w['events_since_previous_pulse']==0:return {**absent,'reason':'no_wait_turn'}
    if w['events_since_previous_pulse']!=1:return {**absent,'state':'invalid','reason':'ambiguous_wakes'}
    event=w['event']
    if event is None or None in (event['arrived_ns'],event['requested_resume_ns'],s['deadline_ns'],w['pulse_ns']):
        return {**absent,'reason':'missing_wake_clock_or_deadline'}
    if not event['requested_resume_supplied'] or event['requested_resume_ns']!=s['deadline_ns']:
        return {**absent,'state':'invalid','reason':'deadline_mismatch'}
    arrival,deadline,pulse=event['arrived_ns'],s['deadline_ns'],s['pulse_ns']
    if event['cause'] not in ('wait_cancelled','resume_time_reached') or arrival>pulse or (event['cause']=='resume_time_reached' and arrival<deadline):
        return {**absent,'state':'invalid','reason':'cause_or_clock_order'}
    # Signed early arrival is retained; it must never saturate to zero.
    return {'state':'valid','reason':None,'before_new_events_ns':arrival-deadline,
            'after_new_events_ns':pulse-arrival}


def acquisition_metrics(t):
    values=[];last=t['began_ns'];empty=False;issues=[]
    for call in t['acquisitions']:
        if call is None:empty=True;continue
        if empty:issues.append('acquisition_slot_gap')
        start,end=call['began_ns'],call['ended_ns']
        duration=v1.delta(start,end)
        if start is not None and last is not None and start<last:issues.append('acquisition_order')
        if values and values[-1]['raw']['outcome'] not in ('outdated','lost'):issues.append('unexpected_acquisition_retry')
        values.append({'raw':call,'duration':duration})
        last=end
        if call['outcome'] is None:issues.append('unfinished_acquisition')
    total=None;rest=None;state='missing'
    if values and all(v['duration']['state']=='valid' and v['raw']['outcome'] is not None for v in values):
        total=sum(v['duration']['ns'] for v in values);state='valid'
    if t['reserved_ns'] is not None:
        if not values or values[-1]['raw']['outcome'] not in SUCCESS:issues.append('reserved_without_acquisition_success')
        if last is not None and last>t['reserved_ns']:issues.append('acquisition_after_reservation')
        if total is not None and t['began_ns'] is not None:
            rest=t['reserved_ns']-t['began_ns']-total
            if rest<0:issues.append('acquisition_exceeds_reservation')
    elif values and values[-1]['raw']['outcome'] in SUCCESS and t['completed_ns'] is not None:
        issues.append('successful_acquisition_without_reservation')
    if t['completed_ns'] is not None and last is not None and last>t['completed_ns']:issues.append('acquisition_after_completion')
    if issues:state='invalid';total=rest=None
    return {'state':state,'calls':values,'sum_call_ns':total,'remaining_reservation_ns':rest,'issues':issues}


def analyze(data):
    projected=project(data);base=v1.analyze(projected)
    rows=[]
    for original in data['host']['attempts']:
        t=original['host_timing'];s=t['schedule']
        wake=wake_metrics(t,s is not None and s['forced_resume']) if original['role']=='gameplay' else {
            'state':'not_applicable','reason':'non_gameplay','before_new_events_ns':None,'after_new_events_ns':None}
        rows.append({'attempt':original['id'],'raw_wake':t['wake'],'wake':wake,'acquisition':acquisition_metrics(t)})
    wake_batches=[]
    last_pulse=last_event=0
    for batch in base['scheduler_batches']:
        selected=[rows[i-1] for i in batch['attempt_ids']]
        bindings=[r['raw_wake'] for r in selected]
        w=bindings[0]
        invalid=not batch['valid'] or not batch['complete_ordinals'] or any(binding!=w for binding in bindings)
        if w is not None:
            event=w['event'];invalid|=w['sequence']<=last_pulse
            last_pulse=max(last_pulse,w['sequence'])
            if event is not None:
                invalid|=event['sequence']<=last_event
                last_event=max(last_event,event['sequence'])
        if invalid:
            for row in selected:row['wake']={'state':'invalid','reason':'stale_or_inconsistent_batch_wake','before_new_events_ns':None,'after_new_events_ns':None}
        for row in selected:
            if row['wake']['state']=='valid':
                require(row['wake']['before_new_events_ns']+row['wake']['after_new_events_ns']==batch['wake_lateness']['ns'],'Paired wake decomposition')
        if all(row['wake']['state']=='valid' for row in selected):
            wake_batches.append({'attempt_ids':batch['attempt_ids'], 'pulse_lateness_ns':batch['wake_lateness']['ns'],
                'before_new_events_ns':selected[0]['wake']['before_new_events_ns'],
                'after_new_events_ns':selected[0]['wake']['after_new_events_ns']})
    wake_states={state:sum(r['wake']['state']==state for r in rows) for state in ('valid','missing','invalid','not_applicable')}
    acquisition_states={state:sum(r['acquisition']['state']==state for r in rows) for state in ('valid','missing','invalid')}
    complete=base['complete_requested_host_timing'] and not data['wake_tracking']['overflow'] and not (wake_states['missing'] or wake_states['invalid'] or acquisition_states['missing'] or acquisition_states['invalid'])
    reservation=[]
    for row,old in zip(rows,base['attempts']):
        acquisition=row['acquisition'];reserve=old['metrics']['reserve']
        if old['raw']['role']=='gameplay' and reserve['state']=='valid' and acquisition['state']=='valid' and acquisition['remaining_reservation_ns'] is not None:
            reservation.append({'attempt':row['attempt'],'reserve_ns':reserve['ns'],'acquisition_call_ns':acquisition['sum_call_ns'],
                                'other_reservation_ns':acquisition['remaining_reservation_ns']})
    wake_tail=paired_tail(wake_batches,'pulse_lateness_ns',('before_new_events_ns','after_new_events_ns'))
    reserve_tail=paired_tail(reservation,'reserve_ns',('acquisition_call_ns','other_reservation_ns'))
    return {'format':'incant-presentation-boundary-analysis','version':2,'diagnostic_only':True,
            'performance_gate_passed':False,'display_cadence_measured':False,'complete_boundary_attribution':complete,
            'wake_tracking':data['wake_tracking'],'wake_states':wake_states,'acquisition_states':acquisition_states,
            'attempts':rows,'host_analysis':base,'wake_batches':wake_batches,
            'paired_largest_5percent_wake_tail':wake_tail,'paired_largest_5percent_reservation_tail':reserve_tail,
            'limitations':['NewEvents is an observer arrival, not the native timer callback or kernel wake.',
                'Acquisition-call wall time includes wgpu/HAL work and waits, not exclusively nextDrawable or CPU time.',
                'Missing/ambiguous/no-wait turns remain explicit; no stale wake or zero-filled measurement.',
                'Same-clock signed components only; no cross-epoch subtraction, FPS or causal improvement claim.']}


def paired_tail(rows, order, components):
    selected=sorted(rows,key=lambda row:row[order],reverse=True)[:(len(rows)+19)//20]
    sums={key:sum(row[key] for row in selected) for key in (order,*components)}
    require(sums[order]==sum(sums[key] for key in components),'Paired tail decomposition')
    return {'valid_subset_only':True,'valid_samples':len(rows),'selected_samples':len(selected),
            'sums':sums,'component_order':list(components),
            'majority_component':(components[0] if sums[components[0]]>sums[components[1]] else components[1] if sums[components[1]]>sums[components[0]] else None) if selected else None,
            'rows':selected}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture',type=Path)
    parser.add_argument('--max-input-mib',type=int,choices=(32,64),default=64)
    args=parser.parse_args()
    try:
        data,digest=v1.read_capture(args.capture,args.max_input_mib)
        result=analyze(data);result['input_sha256']=digest
        encoded=json.dumps(result,allow_nan=False,separators=(',',':')).encode()
        require(len(encoded)<=v1.MAX_BYTES,'Analysis output exceeds 64 MiB')
        sys.stdout.buffer.write(encoded+b'\n')
    except (ValueError,KeyError,TypeError,OSError) as error:
        print(str(error),file=sys.stderr);return 2
    return 0


if __name__=='__main__':sys.exit(main())
