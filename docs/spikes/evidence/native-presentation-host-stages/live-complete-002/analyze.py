"""Independent saved-data arithmetic for this complete historical diagnostic."""
from collections import Counter


def require(value, message):
    if not value:
        raise ValueError(message)


def stats(values):
    ordered = sorted(values)
    require(ordered and all(type(v) is int and v >= 0 for v in ordered), 'Finite nonnegative integer durations')
    return {'samples':len(ordered), 'sum_ns':sum(ordered), 'minimum_ns':ordered[0],
            'p95_nearest_rank_ns':ordered[(95*len(ordered)+99)//100-1], 'maximum_ns':ordered[-1]}


def calculate(capture, reported, report, callbacks, creation, states):
    attempts = capture['host']['attempts']
    game = [a for a in attempts if a['role'] == 'gameplay']
    require(len(attempts) == 2157 and len(game) == 2156
            and [a['requested_host_step'] for a in game] == list(range(1,2157))
            and all(a['status'] == 'host_committed' for a in game), 'Complete raw host route')
    require(capture['finalization']['returned_report'] is True
            and capture['finalization']['accepted_host_steps'] == 2156
            and reported['complete_requested_host_timing'] is True and not reported['partial_reasons'], 'Complete finalization')
    require(not any(capture['host'][k] for k in ['exhausted','omitted_attempts','timing_errors']), 'Raw omissions/errors')
    names = ['pre_attempt','reserve','committed_body','post_commit_haptics','finished_total']
    metrics = {key:[] for key in names}
    wakes, pairs, by_id = [], [], {}
    forced = 0
    for a in game:
        t = a['host_timing']; s = t['schedule']
        require(s['due'] == 1 and s['ordinal'] == 0 and s['discarded_ticks'] == 0, 'Observed scheduler batch changed')
        require(s['forced_resume'] == (s['deadline_ns'] is None), 'Forced resume deadline')
        values = {'pre_attempt':t['began_ns']-s['pulse_ns'], 'reserve':t['reserved_ns']-t['began_ns'],
                  'committed_body':t['completed_ns']-t['reserved_ns'],
                  'post_commit_haptics':t['finished_ns']-t['completed_ns'],
                  'finished_total':t['finished_ns']-t['began_ns']}
        require(values['reserve']+values['committed_body']+values['post_commit_haptics'] == values['finished_total'], 'Exclusive attempt decomposition')
        original = reported['attempts'][a['id']-1]
        require(original['raw'] == a and not original['issues'], 'Original typed attempt binding')
        for name, value in values.items():
            require(type(value) is int and value >= 0
                    and original['metrics'][name] == {'state':'valid','ns':value}, 'Independent duration: '+name)
            metrics[name].append(value)
        if s['forced_resume']: forced += 1
        else: wakes.append(s['pulse_ns']-s['deadline_ns'])
        if by_id:
            before = game[a['requested_host_step']-2]
            old = before['host_timing']
            interval = t['finished_ns']-old['finished_ns']
            gap = s['pulse_ns']-old['finished_ns']
            require(gap+values['pre_attempt']+values['finished_total'] == interval, 'Paired finish decomposition')
            pairs.append({'before_step':before['requested_host_step'], 'after_step':a['requested_host_step'],
                          'interval_ns':interval, 'previous_finish_to_next_pulse_ns':gap,
                          'wake_lateness_ns':s['pulse_ns']-s['deadline_ns'], **{k+'_ns':v for k,v in values.items()}})
        by_id[a['id']] = a
    require(forced == 1 and len(wakes) == 2155, 'Exact scheduler coverage')
    duration_stats = {name:stats(values) for name,values in metrics.items()}
    duration_stats['batch_wake_lateness'] = stats(wakes)
    duration_stats['adjacent_host_finish_intervals'] = stats([p['interval_ns'] for p in pairs])
    for name, actual in duration_stats.items():
        saved = reported['summary'][name]
        require(saved == {'valid':actual['samples'], 'missing':0, 'invalid':0, 'not_applicable':1,
            'valid_subset_sum_ns':actual['sum_ns'], 'valid_subset_min_ns':actual['minimum_ns'],
            'valid_subset_max_ns':actual['maximum_ns'], 'valid_subset_p95_nearest_rank_ns':actual['p95_nearest_rank_ns']}, 'Independent aggregate: '+name)
    require([p['interval_ns'] for p in pairs] == [p['ns'] for p in reported['adjacent_host_finish_intervals'] if p['state']=='valid'], 'Original valid adjacent subset')
    outcomes = dict(Counter(row['outcome'] for row in capture['rows']))
    require(outcomes == callbacks['outcomes'] == {'reported_zero':2,'presented':2155}, 'Callback outcomes')
    positive, nonpositive = [], []
    for b,a in zip(capture['rows'],capture['rows'][1:]):
        if (b['segment'],b['surface_generation']) != (a['segment'],a['surface_generation']): continue
        if b['outcome'] == a['outcome'] == 'presented':
            seconds = a['presented_seconds']-b['presented_seconds']
            value = {'before_step':b['requested_host_step'],'after_step':a['requested_host_step'],'seconds':seconds,
                     'paired_host_finish_ns':by_id[a['attempt']]['host_timing']['finished_ns']-by_id[b['attempt']]['host_timing']['finished_ns']}
            (positive if seconds > 0 else nonpositive).append(value)
    require(len(positive) == len(callbacks['observed_positive_intervals']) == 2154
            and not nonpositive and not callbacks['timestamp_order_issues'], 'Callback adjacency retained')
    snapshots = [s for s in states if s['kind']=='snapshot']
    focused = [s for s in snapshots if s['focus_gained_events']]
    require(focused and all(s['state']['cached_focused'] and s['state']['native_focused']
            and not s['state']['simulation_suspended'] and s['focus_lost_events']==0 for s in focused), 'Focused native trajectory')
    require(states[-1]['kind']=='final' and states[-1]['host_steps']==2156 and states[-1]['returned_report'] is True, 'Final window state')
    require(report['tick']==report['host_steps']==2156 and report['accepted_frames']==2157
            and report['observations']['passed'] is True and report['profile']['completed_frames']==2156
            and report['profile']['failed_frames']==0 and report['dropped_wall_ticks']=='0', 'Complete native report')
    require(creation['snapshot']['overflow']==0
            and all(c['attempts']==c['successes'] and c['errors']==c['unwound']==0 for c in creation['snapshot']['counts'])
            and len(creation['snapshot']['rows'])==91
            and all(r['phase']=='Catalog' and r['outcome']=='Success' for r in creation['snapshot']['rows']), 'Native creation scope')
    top = sorted(pairs,key=lambda p:p['interval_ns'],reverse=True)
    tail = top[:(len(top)+19)//20]
    tail_sums = {key:sum(p[key] for p in tail) for key in ['interval_ns','previous_finish_to_next_pulse_ns','pre_attempt_ns','reserve_ns','committed_body_ns','post_commit_haptics_ns']}
    require(sum(tail_sums[k] for k in tail_sums if k!='interval_ns') == tail_sums['interval_ns'], 'Paired tail accounting')
    largest_wake = max(pairs,key=lambda p:p['wake_lateness_ns'])
    worst_after = by_id[largest_wake['after_step']+1]
    worst_before = by_id[largest_wake['before_step']+1]
    largest_wake = dict(largest_wake, previous_finish_minus_next_deadline_ns=
        worst_before['host_timing']['finished_ns']-worst_after['host_timing']['schedule']['deadline_ns'])
    return {'status':'complete_historical_2156_host_stage_diagnostic', 'host_statistics':duration_stats,
            'between_finish_and_pulse':stats([p['previous_finish_to_next_pulse_ns'] for p in pairs]),
            'largest_five_intervals':top[:5], 'maximum_wake_lateness':largest_wake,
            'largest_reservation':max(pairs,key=lambda p:p['reserve_ns']),
            'largest_committed_body':{'step':game[metrics['committed_body'].index(max(metrics['committed_body']))]['requested_host_step'], 'ns':max(metrics['committed_body'])},
            'paired_largest_5percent_interval_tail':{'samples':len(tail),'sums':tail_sums},
            'scheduler':{'one_tick_batches':2156,'forced_resume_batches':forced,'discarded_ticks':0},
            'callback_outcomes':outcomes,'positive_adjacent_callback_pairs':len(positive),
            'positive_callback_pair_p95_seconds':sorted(p['seconds'] for p in positive)[(95*len(positive)+99)//100-1],
            'largest_callback_pair':max(positive,key=lambda p:p['seconds']), 'cross_epoch_subtraction_performed':False,
            'focus':{'gains':focused[-1]['focus_gained_events'],'losses':focused[-1]['focus_lost_events']},
            'physical_extents':[list(p) for p in sorted({tuple(s['state']['physical_size']) for s in snapshots})],
            'ui_viewport':report['ui']['viewport'],'scale_factor':report['ui']['scale_factor'],
            'audio_device_diagnostics':report['audio_device'],
            'profile_frame_wall_bounds':[{k:v for k,v in phase.items() if k!='counts'} for phase in report['profile']['frame_wall']['phases']],
            'native_creation_catalog_calls':91,'native_creation_gameplay_calls':0,
            'full_gameplay_observation_parity':True,'performance_gate_passed':False,'display_cadence_measured':False}
