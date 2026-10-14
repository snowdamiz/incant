"""Same-frame zero-cost feasibility only, never a shipping optimization claim."""
from native_package import require
from mac_performance import integer

REGIONS = ['path_query', 'initial_start', 'repeated_start']


def vector(value, bound):
    require(isinstance(value, list) and len(value) == 3, 'Navigation vector shape')
    require(all(integer(n, 'navigation aggregate') <= bound for n in value),
            'Navigation aggregate bound')
    return value


def analyze_navigation(diagnostic):
    require(diagnostic['schema'] == 2 and diagnostic['navigation_regions'] == REGIONS,
            'Navigation schema mismatch')
    rows = diagnostic['rows']
    require(rows, 'Missing navigation attribution rows')
    totals = {'ns': [0]*3, 'calls': [0]*3, 'failed': [0]*3}
    hypothetical = []
    for row in rows:
        nav = row['navigation']
        ns = vector(nav['ns'], 2**64-1)
        calls = vector(nav['calls'], 65535)
        failed = vector(nav['failed'], 65535)
        require(all(f <= n for f, n in zip(failed, calls)), 'Failures exceed calls')
        require(all(n != 0 or t == 0 for n, t in zip(calls, ns)), 'Time without calls')
        # A final start lookup happens before the inner attempt65 refusal.
        require(calls[1] <= calls[0] and calls[2] <= 64*calls[1], 'Lookup/query coverage')
        require(ns[1]+ns[2] <= ns[0] <= sum(row['source_ns'])
                and ns[0] <= row['whole_tick_ns'], 'Navigation exceeds enclosing scope')
        for key in totals:
            totals[key] = [a+b for a, b in zip(totals[key], nav[key])]
        hypothetical.append({'frame': row['frame'], 'whole_tick_ns': row['whole_tick_ns'],
                             'repeated_start_ns': ns[2],
                             'zero_cost_repeated_start_ns': row['whole_tick_ns']-ns[2]})
    rank = (95*len(rows)+99)//100-1
    ranked = sorted(hypothetical, key=lambda r: (r['zero_cost_repeated_start_ns'], r['frame']))
    zero_cost = ranked[rank]['zero_cost_repeated_start_ns']
    return {'diagnostic_only': True, 'eligible_for_performance_gate': False,
            'regions': REGIONS, 'totals': totals, 'rows': hypothetical,
            'zero_cost_same_frame_p95_ns': zero_cost,
            'stop_hypothesis': totals['calls'][2] == 0 or zero_cost > 2_000_000,
            'interpretation': 'A hypothetical zero-cost repeated scan, subtracted only from its own whole tick. Observer cost remains. This is not a new gate, speedup, or candidate measurement. A positive feasibility result only justifies separate correctness and matched shipping work.'}
