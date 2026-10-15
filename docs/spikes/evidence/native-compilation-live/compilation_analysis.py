"""Bounded record analysis; never infer driver work or repair clocks/coverage."""
from collections import Counter, defaultdict
from analyze import require

KINDS = ['NagaLibrary', 'MslLibrary', 'BinaryLibrary', 'RenderPipeline', 'MeshPipeline', 'ComputePipeline']
PHASES = ['Unscoped', 'Startup', 'Catalog', 'Background', 'Gameplay']
STAGES = {'Unspecified', 'Vertex', 'Fragment', 'Compute', 'Task', 'Mesh', 'RayGeneration', 'Miss', 'AnyHit', 'ClosestHit'}
FIELDS = ['attempts', 'successes', 'errors', 'unwound']
OUTCOMES = {'Pending': None, 'Success': 'successes', 'Error': 'errors', 'Unwound': 'unwound'}
BOUNDARIES = ['Focus', 'Resize', 'Suspend', 'Resume', 'Restart', 'Reconfigured', 'Recreated', 'Deferred', 'ReservationError']
U64 = (1 << 64) - 1


def integer(value):
    return type(value) is int and 0 <= value <= U64


def analyze_compilation(value, presentations=None):
    require(value['format'] == 'incant-native-compilation' and type(value['version']) is int
            and value['version'] == 1 and value['diagnostic_only'] is True
            and value['shader_stall_gate'] is False and value['display_gate'] is False,
            'Unknown creation observation scope')
    capacity = value['capacity']
    require(integer(capacity) and 1 <= capacity <= 4096
            and integer(value['requested_host_steps']) and 1 <= value['requested_host_steps'] <= 36000,
            'Creation capture bounds')
    require(value['scope_phase_order'] == PHASES and value['boundary_order'] == BOUNDARIES,
            'Changed phase/boundary identities')
    require(len(value['boundaries']) == len(BOUNDARIES)
            and all(integer(n) for n in value['boundaries']) and integer(value['segment'])
            and type(value['segment_overflow']) is bool, 'Malformed host boundary counters')
    scopes = value['host_scopes']
    require(type(scopes['saturated']) is bool and len(scopes['scopes']) == len(PHASES), 'Host scope shape')
    for counts in scopes['scopes']:
        require(set(counts) == {'started', 'committed', 'abandoned'}
                and all(integer(n) for n in counts.values()), 'Host scope counters')
        if not scopes['saturated']:
            require(counts['committed'] + counts['abandoned'] <= counts['started'], 'Impossible host scope outcomes')
    host_pending = (sum(c['started'] - c['committed'] - c['abandoned'] for c in scopes['scopes'])
                    if not scopes['saturated'] else None)
    snapshot = value['snapshot']
    if snapshot is None:
        return {'format': 'incant-native-creation-observation', 'capture_missing': True,
                'coverage_limited_or_inconclusive': True, 'shader_stall_gate': False,
                'finalization': value['finalization'], 'host_scopes': scopes,
                'pending_host_scopes': host_pending, 'display_gate': False}
    require(snapshot['kind_order'] == KINDS and integer(snapshot['device']) and snapshot['device'] > 0,
            'Creation device/kind identity')
    require(type(snapshot['saturated']) is bool and type(snapshot['capture_started']) is bool
            and type(snapshot['scope_active']) is bool and integer(snapshot['overflow']), 'Snapshot flags')
    require(snapshot['snapshot_ns'] is None or integer(snapshot['snapshot_ns']), 'Snapshot clock')
    anomalies = snapshot['clock_anomalies']
    require(set(anomalies) == {'entry', 'completion', 'snapshot'} and all(integer(n) for n in anomalies.values()),
            'Clock anomaly counters')
    require(len(snapshot['rows']) <= capacity, 'Creation row allocation bound')
    for series in ['counts', 'unbound']:
        require(len(snapshot[series]) == len(KINDS), 'Cumulative counter shape')
        for counts in snapshot[series]:
            require(set(counts) == set(FIELDS) and all(integer(n) for n in counts.values()), 'Cumulative counters')
            if not snapshot['saturated']:
                require(sum(counts[k] for k in FIELDS[1:]) <= counts['attempts'], 'Impossible native outcomes')
    retained = {kind: Counter() for kind in KINDS}
    summaries, correlations, missing_clock = {}, [], Counter()
    attempts = defaultdict(list)
    if presentations is not None:
        for attempt in presentations['host']['attempts']:
            if attempt['role'] == 'gameplay':
                attempts[(attempt['session_generation'], attempt['requested_host_step'])].append(attempt['id'])
    last = None
    for row in snapshot['rows']:
        require(row['kind'] in KINDS and row['phase'] in PHASES and row['stage'] in STAGES
                and row['outcome'] in OUTCOMES, 'Unknown creation row identity')
        require(all(integer(row[k]) for k in ['sequence', 'session_generation', 'surface_generation', 'segment', 'requested_host_step'])
                and row['sequence'] > 0, 'Invalid creation context counters')
        if last is not None and not snapshot['saturated']:
            require(row['sequence'] == last + 1, 'Retained creation sequence gap')
        last = row['sequence']
        if row['phase'] == 'Unscoped':
            require(all(row[k] == 0 for k in ['session_generation', 'surface_generation', 'segment', 'requested_host_step']),
                    'Unscoped row claims a host identity')
        else:
            require(row['session_generation'] > 0, 'Scoped session identity')
        if row['phase'] == 'Gameplay':
            require(all(row[k] > 0 for k in ['surface_generation', 'segment', 'requested_host_step']), 'Gameplay entry identity')
        require(row['error_domain'] in {'None', 'Library', 'RenderPipeline', 'ComputePipeline', 'Other'}
                and type(row['error_code']) is int and -(1 << 63) <= row['error_code'] < (1 << 63), 'Native error identity')
        for field in ['begin_ns', 'elapsed_ns']:
            require(row[field] is None or integer(row[field]), 'Invalid creation clock value')
        require(row['outcome'] != 'Pending' or row['elapsed_ns'] is None, 'Pending native call has completion duration')
        missing_clock['entry'] += row['begin_ns'] is None
        missing_clock['completion'] += row['outcome'] != 'Pending' and row['elapsed_ns'] is None
        retained[row['kind']]['attempts'] += 1
        if OUTCOMES[row['outcome']]:
            retained[row['kind']][OUTCOMES[row['outcome']]] += 1
        key = (row['phase'], row['kind'])
        summary = summaries.setdefault(key, {'calls': 0, 'outcomes': Counter(), 'valid_duration_calls': 0, 'sum_call_ns': 0, 'max_call_ns': None})
        summary['calls'] += 1; summary['outcomes'][row['outcome']] += 1
        if row['elapsed_ns'] is not None:
            summary['valid_duration_calls'] += 1; summary['sum_call_ns'] += row['elapsed_ns']
            summary['max_call_ns'] = max(summary['max_call_ns'] or 0, row['elapsed_ns'])
        if row['phase'] == 'Gameplay':
            candidates = attempts[(row['session_generation'], row['requested_host_step'])]
            correlations.append({'sequence': row['sequence'], 'matching_host_attempt_ids': candidates,
                                 'scope_entry_surface': row['surface_generation'],
                                 'exact_drawable_or_time_join': False})
    if not snapshot['saturated']:
        deficits = []
        for kind, total, prefix in zip(KINDS, snapshot['counts'], snapshot['unbound'], strict=True):
            deficit = {k: total[k] - prefix[k] - retained[kind][k] for k in FIELDS}
            require(all(n >= 0 for n in deficit.values()) and sum(deficit[k] for k in FIELDS[1:]) <= deficit['attempts'],
                    'Counts disagree with prefix/retained/overflow outcomes')
            deficits.append(deficit)
        require(sum(d['attempts'] for d in deficits) == snapshot['overflow'], 'Omitted call count differs')
        if snapshot['rows']:
            require(snapshot['rows'][0]['sequence'] == sum(c['attempts'] for c in snapshot['unbound']) + 1,
                    'Retained sequence does not follow unbound prefix')
        require(all(anomalies[k] >= missing_clock[k] for k in ['entry', 'completion'])
                and (snapshot['snapshot_ns'] is not None or anomalies['snapshot'] > 0), 'Missing clock anomaly correspondence')
    pending = sum(c['attempts'] - sum(c[k] for k in FIELDS[1:]) for c in snapshot['counts']) if not snapshot['saturated'] else None
    catalog_seen = any(r['phase'] == 'Catalog' for r in snapshot['rows'])
    limited = bool(snapshot['saturated'] or snapshot['overflow'] or any(anomalies.values())
                   or snapshot['scope_active'] or not snapshot['capture_started'] or pending
                   or scopes['saturated'] or host_pending or value['segment_overflow'] or not catalog_seen
                   or value['finalization'].get('stage') != 'window_finalized')
    return {'format': 'incant-native-creation-observation', 'shader_stall_gate': False, 'display_gate': False,
            'capture_missing': False, 'coverage_limited_or_inconclusive': limited,
            'kind_order': KINDS, 'scope_phase_order': PHASES,
            'counts': snapshot['counts'], 'unbound': snapshot['unbound'], 'overflow': snapshot['overflow'],
            'pending_calls': pending, 'clock_anomalies': anomalies, 'catalog_rows_observed': catalog_seen,
            'phase_kind_observations': [dict(phase=p, kind=k, **v) for (p, k), v in sorted(summaries.items())],
            'host_scopes': scopes, 'pending_host_scopes': host_pending,
            'boundaries': dict(zip(BOUNDARIES, value['boundaries'], strict=True)),
            'host_attempt_candidates': correlations, 'finalization': value['finalization'],
            'limits': ['Only six native call sites; excludes deferred driver work and lookup/translation/queue waits',
                       'Sum of call intervals is not exclusive CPU or displayed frame time',
                       'Scopes retain entry identities; segment numbering and clock epochs differ from presentation data',
                       'Matching host attempts are candidates, never proof of exact drawable or causal display gap',
                       'Zero observed gameplay calls cannot establish shader-hitch freedom']}
