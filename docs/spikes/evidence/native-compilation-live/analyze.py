"""Read actual callback coverage without manufacturing a display-performance gate."""
from collections import Counter
import math


def require(value, why):
    if not value:
        raise ValueError(why)


def analyze(sidecar):
    require(sidecar['format'] == 'incant-native-presentations' and sidecar['version'] == 1,
            'Unknown sidecar format')
    require(sidecar['diagnostic_only'] and not sidecar['display_gate']
            and not sidecar['uncapped_gpu_throughput'], 'Unknown observation scope')
    capacity = sidecar['capture_capacity']
    require(type(capacity) is int and 1 <= capacity <= 36_000, 'Capacity outside source bound')
    rows, host = sidecar['rows'], sidecar['host']
    attempts, segments = host['attempts'], host['segments']
    require(len(rows) <= capacity and len(attempts) <= capacity and len(segments) <= 2*capacity,
            'Record allocation bounds')
    require([a['id'] for a in attempts] == list(range(1, len(attempts)+1)), 'Attempt identities')
    require([s['id'] for s in segments] == list(range(1, len(segments)+1)), 'Segment identities')
    by_attempt, by_segment = {a['id']: a for a in attempts}, {s['id']: s for s in segments}
    outcomes = Counter(); roles = Counter(); actual_intervals = []
    timestamp_order_issues, drawable_order_issues, gaps = [], [], []
    previous = None; last_drawable = {}; observed_attempts = set()
    allowed = {'acquired', 'requested_missing_callback', 'presented', 'reported_zero',
               'invalid', 'discarded', 'abandoned'}
    for index, row in enumerate(rows):
        require(row['outcome'] in allowed and row['attempt'] in by_attempt
                and row['segment'] in by_segment, 'Unknown outcome or correlation identity')
        attempt, segment = by_attempt[row['attempt']], by_segment[row['segment']]
        require(row['requested_host_step'] == attempt['requested_host_step']
                and row['surface_generation'] == segment['surface_generation']
                and segment['session_generation'] == attempt['session_generation']
                and segment['role'] == attempt['role'], 'Row/attempt/segment correspondence')
        require(type(row['drawable_id']) is int and row['drawable_id'] >= 0, 'Drawable identity')
        generation = row['surface_generation']
        if generation in last_drawable and row['drawable_id'] <= last_drawable[generation]:
            drawable_order_issues.append(index)
        last_drawable[generation] = row['drawable_id']
        observed_attempts.add(row['attempt'])
        outcomes[row['outcome']] += 1; roles[attempt['role']] += 1
        time = row['presented_seconds']
        if row['outcome'] == 'presented':
            require(type(time) in (float, int) and math.isfinite(time) and time > 0
                    and row['invalid_bits'] is None, 'Malformed positive timestamp')
        else:
            require(time is None, 'Non-presented row has a timestamp')
        # Adjacency is in acquisition order. Missing/zero/invalid/discard rows
        # interrupt continuity; never filter them then bridge the gap.
        if previous is not None:
            same = (previous['surface_generation'], previous['segment']) == (generation, row['segment'])
            if same and previous['outcome'] == row['outcome'] == 'presented':
                interval = time-previous['presented_seconds']
                if interval <= 0:
                    timestamp_order_issues.append({'before': index-1, 'after': index, 'seconds': interval})
                else:
                    actual_intervals.append({'before': index-1, 'after': index,
                        'segment': row['segment'], 'role': attempt['role'], 'seconds': interval,
                        'before_attempt_status': by_attempt[previous['attempt']]['status'],
                        'after_attempt_status': attempt['status']})
            else:
                gaps.append({'before': index-1, 'after': index, 'segment_boundary': not same,
                             'before_outcome': previous['outcome'], 'after_outcome': row['outcome']})
        previous = row
    coverage = sidecar['coverage']
    require(coverage['rows'] == len(rows), 'Coverage row count')
    for key, value in [('acquired', 'acquired'), ('missing', 'requested_missing_callback'),
                       ('presented', 'presented'), ('zero', 'reported_zero'), ('invalid', 'invalid'),
                       ('discarded', 'discarded'), ('abandoned', 'abandoned')]:
        require(coverage[key] == outcomes[value], 'Coverage counter differs: '+key)
    statuses = Counter(a['status'] for a in attempts)
    incomplete = bool(coverage['overflow'] or coverage['protocol_errors'] or host['omitted_attempts']
                      or host['exhausted'] or timestamp_order_issues or drawable_order_issues)
    incomplete |= sidecar['finalization'].get('stage') != 'window_finalized'
    return {'format': 'incant-presentation-coverage-observation', 'version': 1,
            'diagnostic_only': True, 'performance_acceptance': False,
            'rows': len(rows), 'outcomes': dict(outcomes), 'rows_by_role': dict(roles),
            'attempts': len(attempts), 'attempt_statuses': dict(statuses), 'segments': len(segments),
            'attempts_without_acquisition_rows': [a['id'] for a in attempts if a['id'] not in observed_attempts],
            'coverage': coverage, 'boundaries': host['boundary_counts'],
            'acquisition_failures': host['acquisition_failures'],
            'observed_positive_intervals': actual_intervals, 'continuity_breaks': gaps,
            'timestamp_order_issues': timestamp_order_issues, 'drawable_order_issues': drawable_order_issues,
            'coverage_limited_or_inconclusive': incomplete or not outcomes['presented']
                or any(outcomes[o] for o in allowed - {'presented'}),
            'initial_and_final_unobserved_intervals': True, 'finalization': sidecar['finalization'],
            'limits': ['Raw rows retained in acquisition order; no timestamps inferred or sorted to repair coverage',
                       'Initial/redraw roles and accepted host commits are distinct from callback outcomes',
                       'Only adjacent acquired-row timestamps are listed; skipped/discarded drawables split this limited subset too',
                       'Positive adjacent timestamps are displayed cadence observations, not uncapped GPU throughput',
                       'No global p95 or passing FPS/performance verdict from this coverage probe',
                       'Callback registration and final snapshot introduce observer overhead']}


def audio_observation(report):
    """Retain typed current-owner diagnostics without inferring audible continuity."""
    value = report.get('audio_device')
    require(type(value) is dict and set(value) == {
        'scope', 'diagnostics', 'glitch_free_output_verified'}, 'Unknown audio report')
    require(value['scope'] == 'current-session-owner'
            and value['glitch_free_output_verified'] is False, 'Audio report scope changed')
    diagnostics = value['diagnostics']
    require(type(diagnostics) is dict and set(diagnostics) == {
        'xrun_notifications', 'poll_limit_hits', 'counters_saturated'}, 'Unknown audio diagnostics')
    for key in ('xrun_notifications', 'poll_limit_hits'):
        require(type(diagnostics[key]) is int and 0 <= diagnostics[key] <= (1 << 64)-1,
                'Invalid audio counter: ' + key)
    require(type(diagnostics['counters_saturated']) is bool, 'Invalid audio saturation flag')
    # Zero and nonzero counts are both observations, never hardware success criteria.
    return value
