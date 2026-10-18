"""Schema5 inclusive observations; never infer interpreter time or saved CPU."""
from analyze import vector
from common import TICKS
from native_package import require

CAPABILITIES = ['character_motion', 'trigger_events', 'navigation_path', 'locale', 'localize', 'other']
REGIONS = ['call', 'parse', 'execute', 'materialize']
FIELDS = {'inclusive_ns', 'calls', 'failed', 'source_masks', 'recursive_calls'}


def matrix(value, name, maximum=2**64 - 1):
    require(isinstance(value, list) and len(value) == 6, 'Invalid query ' + name + ' shape')
    return [vector(row, 4, 'query ' + name, maximum) for row in value]


def analyze_queries(diagnostic, slots, tail_frames):
    require(diagnostic['query_capabilities'] == CAPABILITIES
            and diagnostic['query_regions'] == REGIONS, 'Unknown query schema')
    require('navigation_regions' not in diagnostic and 'budget_check_regions' not in diagnostic,
            'Combined query diagnostic is not admitted')
    rows = diagnostic['rows']
    require(len(rows) == TICKS, 'Missing query frames')
    known_mask = sum(1 << slot for slot in slots)
    for row in rows:
        require('navigation' not in row and 'budget_checks' not in row, 'Combined query row')
        q = row['queries']
        require(set(q) == FIELDS, 'Unknown query counters')
        times = matrix(q['inclusive_ns'], 'times')
        calls = matrix(q['calls'], 'calls', 2**32 - 1)
        failed = matrix(q['failed'], 'failures', 2**32 - 1)
        masks = vector(q['source_masks'], 6, 'query sources')
        recursive = vector(q['recursive_calls'], 6, 'recursive calls', 2**32 - 1)
        require(recursive == [0] * 6, 'Recursive inclusive calls cannot claim exclusive coverage')
        for ns, count, errors, mask in zip(times, calls, failed, masks, strict=True):
            require(mask & ~known_mask == 0 and bool(mask) == bool(count[0]), 'Query source identity')
            require(all(n or duration == 0 for n, duration in zip(count, ns)), 'Query time without call')
            require(all(error <= n for error, n in zip(errors, count)), 'Query failures exceed calls')
            require(errors == [0] * 4 and count == [count[0]] * 4,
                    'Successful Core route needs every query stage without refusal')
            require(sum(ns[1:]) <= ns[0], 'Nested query stages exceed calls')
            require(ns[0] <= sum(row['source_ns'][slot] for slot in slots if mask & (1 << slot)),
                    'Query exceeds declared source callbacks')
        require(sum(ns[0] for ns in times) <= sum(row['source_ns']), 'Query calls exceed callbacks')
        for slot in slots:
            require(sum(ns[0] for ns, mask in zip(times, masks) if mask == 1 << slot)
                    <= row['source_ns'][slot], 'Single-source query calls exceed their callback')

    def aggregate(selected):
        totals = {key: [[sum(row['queries'][key][c][s] for row in selected) for s in range(4)]
                        for c in range(6)] for key in ('inclusive_ns', 'calls', 'failed')}
        masks = [0] * 6
        for row in selected:
            for c in range(6):
                masks[c] |= row['queries']['source_masks'][c]
        return {'frames': [row['frame'] for row in selected], 'count': len(selected), **totals,
                'source_masks': masks,
                'call_residual_ns': [ns[0] - sum(ns[1:]) for ns in totals['inclusive_ns']],
                'callback_residual_ns': sum(sum(row['source_ns']) for row in selected)
                                       - sum(ns[0] for ns in totals['inclusive_ns']),
                'wall_ns_per_call': [[{'numerator_ns': ns, 'denominator_calls': count} if count else None
                                     for ns, count in zip(times, calls)]
                                    for times, calls in zip(totals['inclusive_ns'], totals['calls'])]}

    require(len(set(tail_frames)) == len(tail_frames)
            and set(tail_frames) <= {row['frame'] for row in rows}, 'Unknown query tail')
    by_frame = {row['frame']: row for row in rows}
    all_frames = aggregate(rows)
    require(sum(c[0] for c in all_frames['calls']) > 0, 'No native query observations')
    return {'schema': 1, 'capabilities': CAPABILITIES, 'regions': REGIONS,
            'all_frames': all_frames,
            'same_frame_tail': aggregate([by_frame[f] for f in tail_frames]),
            'other_queries_present': all_frames['calls'][-1][0] > 0,
            'interpretation': 'Stage spans belong to calls; calls belong to source callbacks. '
                'Call residual includes admission, limits, checks and interstage observer work. '
                'Callback residual includes VM/bridge work and query token setup/drop aggregation. '
                'Source masks identify contributors, not per-source durations. '
                'All times are instrumented wall observations, not exclusive CPU or predicted savings. '
                'No marginal-quantile subtraction, speedup or performance-gate conclusion.'}
