"""Strict saved-row arithmetic for one same-player diagnostic pair. Never launches."""
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import sys

HERE=Path(__file__).resolve().parent
E=Path('/Users/andreyyurlov/.codex/incant-evidence/native-metal-minimum-duration-001')
REPO=HERE.parents[4]
RECORDS=None
sys.path.insert(0,str(REPO/'tools'))
import presentation_host_stages as host
import presentation_boundaries as boundary


def require(value,message):
    if not value: raise ValueError(message)


def sha(path):
    if RECORDS is not None:return hashlib.sha256(RECORDS[str(path)]).hexdigest()
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def read(path,cap=8*1024**2):
    raw=RECORDS[str(path)] if RECORDS is not None else host.read_bounded(path,cap)
    require(len(raw)<=cap,'Input cap')
    return json.loads(raw,object_pairs_hook=host.duplicate_free,
                      parse_constant=lambda x:require(False,'Nonfinite JSON '+x))


def stats(values):
    require(bool(values) and all(math.isfinite(x) for x in values),'Missing/nonfinite measurements')
    ordered=sorted(values)
    return {'samples':len(values),'sum_ns':math.fsum(values),'min_ns':ordered[0],
            'max_ns':ordered[-1],'p95_nearest_rank_ns':ordered[math.ceil(.95*len(values))-1],
            'mean_ns':math.fsum(values)/len(values)}


def cadence(rows,attempts):
    # Never sort away gaps or filter unsuccessful gameplay acquisitions.
    game=[r for r in rows if attempts[r['attempt']]['role']=='gameplay']
    require([r['requested_host_step'] for r in game]==list(range(1,2157)),
            'Consecutive exact gameplay callback membership')
    require(all(r['outcome']=='presented' and r['presented_seconds']>0 for r in game),
            'Every gameplay callback must be positive; do not bridge missing/zero rows')
    require(len({(r['segment'],r['surface_generation']) for r in game})==1,
            'Do not bridge presentation boundaries')
    times=[r['presented_seconds'] for r in game]
    intervals=[(b-a)*1e9 for a,b in zip(times,times[1:])]
    require(all(v>0 for v in intervals),'Non-increasing callback time')
    finishes=[attempts[r['attempt']]['host_timing']['finished_ns'] for r in game]
    require(all(type(x) is int for x in finishes) and all(a<b for a,b in zip(finishes,finishes[1:])),
            'Host-finish order')
    presentation_relative=[(x-times[0])*1e9 for x in times]
    host_relative=[x-finishes[0] for x in finishes]
    proxy=[p-h for p,h in zip(presentation_relative,host_relative)]
    phase=[v-i*(1e9/60) for i,v in enumerate(presentation_relative)]
    quanta=[round(v/(1e9/120)) for v in intervals]
    return {'positive_gameplay_callbacks':len(game),'presentation_span_ns':presentation_relative[-1],
        'host_finish_span_ns':host_relative[-1],'intervals':stats(intervals),
        'empirical_120Hz_quantum_counts':dict(sorted(Counter(quanta).items())),
        'three_then_one_adjacent_pairs':sum(a==3 and b==1 for a,b in zip(quanta,quanta[1:])),
        'quantum_rounding_residual_ns':stats([v-q*(1e9/120) for v,q in zip(intervals,quanta)]),
        'relative_60Hz_phase_ns':{'first':phase[0],'last':phase[-1],'min':min(phase),'max':max(phase)},
        'relative_presentation_minus_host_span_proxy_ns':{
            'first':proxy[0],'last':proxy[-1],'min':min(proxy),'max':max(proxy)},
        'clock_limits':'Separate first-row anchors only. No common clock epoch, absolute queue depth, GPU completion or input-latency claim. Empirical120Hz quantization is a diagnostic binning rule, not a queried display mode.'}


def arm(name):
    W=E/('window-ordinary-001' if name=='ordinary' else 'window-minimum-duration60-001')
    data=read(W/'presentations.json');digest=sha(W/'presentations.json')
    projected=boundary.project(data)
    # The original strict analyzers are replayed unchanged, including failure.
    a=host.analyze(projected);b=boundary.analyze(data)
    result,report=read(W/'result.json'),read(W/'stdout.json')
    attempts={x['id']:x for x in data['host']['attempts']}
    require(report['tick']==report['host_steps']==2156 and report['observations']['passed'],
            'Full functional replay required')
    observed=report['observations']
    require(len(observed['checks'])==8 and len(observed['entities'])==17,'Exact observation membership')
    frames=[x for x in attempts.values() if x['role']=='gameplay']
    require(len(frames)==2156 and all(x['status']=='host_committed' for x in frames),'Committed gameplay rows')
    by_acq={x['attempt']:x['acquisition'] for x in b['attempts']}
    acq=[sum(call['ended_ns']-call['began_ns']
             for call in x['host_timing']['acquisitions'] if call is not None) for x in frames]
    require(acq==[by_acq[x['id']]['sum_call_ns'] for x in frames],
            'Raw acquisition intervals disagree with strict v2 decomposition')
    require(all(by_acq[x['id']]['state']=='valid' for x in frames),'Acquisition evidence invalid')
    metrics={
       'reserve':stats([x['host_timing']['reserved_ns']-x['host_timing']['began_ns'] for x in frames]),
       'acquisition':stats(acq),
       'body':stats([x['host_timing']['completed_ns']-x['host_timing']['reserved_ns'] for x in frames]),
       'finished_total':stats([x['host_timing']['finished_ns']-x['host_timing']['began_ns'] for x in frames])}
    incomplete=[x for x in a['scheduler_batches'] if not x['complete_ordinals']]
    badwake=[{'attempt':x['attempt'],'requested_host_step':attempts[x['attempt']]['requested_host_step'],
              'state':x['wake']} for x in b['attempts'] if x['wake']['state']=='invalid']
    return {'arm':name,'original_controller_status':result['status'],'original_controller_exit':result['exit_code'],
       'original_strict_timing_complete':a['complete_requested_host_timing'],
       'original_strict_partial_reasons':a['partial_reasons'],
       'original_strict_boundary_complete':b['complete_boundary_attribution'],
       'raw_presentations_sha256':digest,'raw_report_sha256':sha(W/'stdout.json'),
       'native_pacing':report.get('native_pacing'),'observations':observed,
       'cadence':cadence(data['rows'],attempts),'same_row_wall_metrics':metrics,
       'first_gameplay_began_ns':frames[0]['host_timing']['began_ns'],
       'dropped_wall_ticks':int(report['dropped_wall_ticks']),
       'due_histogram':dict(sorted(Counter(x['schedule']['due'] for x in a['scheduler_batches']).items())),
       'incomplete_scheduler_batches':incomplete,'invalid_wake_rows':badwake,
       'wake_tracking':data['wake_tracking'],'callback_coverage':data['coverage'],
       'audio_counters':report['audio_device'],'window_final':result['analysis']['window_state_final'],
       'focus_lost_events':result['analysis']['focus_lost_events'],
       'interpretation_limits':['Per-row reserve/acquisition/body are observer-inclusive serial wall spans, not reclaimable CPU costs. Multi-ordinal pre-attempt spans overlap and are not summed as work.',
        'Whole child startup is unmatched and fixed order may warm OS/driver caches. Compare gameplay-relative spans, not overall child speed. One pair is not statistical adoption or general performance evidence.']}


def analyze(records=None):
    global RECORDS
    RECORDS=records
    ordinary,treatment=arm('ordinary'),arm('minimum-duration60')
    require(ordinary['observations']==treatment['observations'],'Paired full gameplay observations differ')
    require(ordinary['original_controller_status']=='complete' and treatment['original_controller_status']=='failed',
            'Original acceptance statuses must remain')
    # Diagnose the actual strict refusal without changing either analyzer/gate.
    incomplete=treatment['incomplete_scheduler_batches']
    require(len(incomplete)==1 and incomplete[0]['valid'] and incomplete[0]['schedule']['due']==4
            and incomplete[0]['ordinals']==[0,1,2] and incomplete[0]['attempt_ids']==[2156,2157,2158],
            'Unexpected strict-refusal diagnosis')
    p=lambda a:a['cadence']['presentation_span_ns']
    h=lambda a:a['cadence']['host_finish_span_ns']
    proxy=lambda a:a['cadence']['relative_presentation_minus_host_span_proxy_ns']['last']
    return {'format':'incant-minimum-duration-paired-saved-analysis-v1','source':'380a03a4da2e3882dce8e199e85fcce3d13e4006',
       'ordinary':ordinary,'treatment':treatment,
       'differences':{'presentation_span_ns':p(treatment)-p(ordinary),
           'mean_presentation_interval_ns':(p(treatment)-p(ordinary))/2155,
           'host_finish_span_ns':h(treatment)-h(ordinary),
           'relative_span_proxy_final_ns':proxy(treatment)-proxy(ordinary),
           'dropped_wall_ticks':treatment['dropped_wall_ticks']-ordinary['dropped_wall_ticks']},
       'strict_failure_diagnosis':'The final due4 scheduler batch reaches stop-after2156 after ordinals0,1,2 (host steps2154..2156); scheduled ordinal3 is never consumed. The unchanged v1 gate requires complete ordinals and fails; unchanged v2 marks those same3 wake rows stale/inconsistent. This identifies a terminal cutoff, not a claim the frozen acceptance passed.',
       'conclusion':'Do not adopt minimum-duration60 from this pair. Removing short intervals coincides with longer gameplay presentation span, much more drawable acquisition wall wait, multi-step catch-up batches and4discarded wall ticks. The strict treatment result remains FAILED.',
       'performance_gate_passed':False,'display_gate_passed':False,'default_policy_changed':False,
       'retry_or_new_native_execution':False,'clock_limits':'Relative independently anchored spans only; no absolute cross-clock subtraction or direct queue-depth/input-latency measurement.'}


if __name__=='__main__':
    from verify import load
    _,records=load()
    print(json.dumps(analyze(records),indent=2,allow_nan=False))
