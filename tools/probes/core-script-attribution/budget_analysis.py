"""Exact schema3 aggregates; wall observations never imply reclaimable CPU time."""
from analyze import vector
from common import TICKS
from native_package import require

REGIONS = ['interrupt_callback', 'explicit_callback', 'interrupt_other', 'explicit_other']


def analyze_budget(diagnostic, tail_frames):
    require(diagnostic['budget_check_regions'] == REGIONS, 'Unknown budget-check regions')
    require('navigation_regions' not in diagnostic, 'Unexpected combined diagnostic')
    rows = diagnostic['rows']
    require(len(rows) == TICKS, 'Missing budget frames')
    for row in rows:
        require('navigation' not in row, 'Unexpected navigation row')
        counts = row['budget_checks']
        require(set(counts) == {'ns', 'calls', 'refused', 'clock_reads'}, 'Unknown budget counters')
        ns = vector(counts['ns'], 4, 'budget ns')
        calls = vector(counts['calls'], 4, 'budget calls', 2**32 - 1)
        refused = vector(counts['refused'], 4, 'budget refusals', 2**32 - 1)
        reads = vector(counts['clock_reads'], 4, 'budget clock reads', 2**32 - 1)
        require(refused == [0] * 4 and reads == calls, 'Successful route check/clock coverage differs')
        require(all(count or duration == 0 for count, duration in zip(calls, ns)),
                'Duration without a check')
        callbacks = sum(row['source_ns'])
        require(sum(ns[:2]) <= callbacks and sum(ns[2:]) <= row['trace_ns'] - callbacks,
                'Budget checks exceed enclosing callback/other work')
    require(sum(row['budget_checks']['calls'][0] for row in rows) > 0
            and sum(row['budget_checks']['calls'][1] for row in rows) > 0,
            'Full Core route lacks actual interrupt/native checks')

    def aggregate(selected):
        totals = {key: [sum(row['budget_checks'][key][i] for row in selected) for i in range(4)]
                  for key in ('ns', 'calls', 'refused', 'clock_reads')}
        return {'frames': [row['frame'] for row in selected], 'count': len(selected), **totals,
                'wall_ns_per_check': [{'numerator_ns': ns, 'denominator_checks': calls} if calls else None
                                      for ns, calls in zip(totals['ns'], totals['calls'])]}

    tail = set(tail_frames)
    require(len(tail) == len(tail_frames) and tail <= {row['frame'] for row in rows}, 'Unknown budget tail')
    by_frame = {row['frame']: row for row in rows}
    return {'schema': 1, 'regions': REGIONS, 'all_frames': aggregate(rows),
            'same_frame_tail': aggregate([by_frame[frame] for frame in tail_frames]),
            'interpretation': 'Nested monotonic wall lock/check observations include clock and observer work. '
                              'clock_reads counts unchanged enforcement attempts, excluding resets. '
                              'No CPU-cost equivalence, time removal, saving, gate or optimization claim.'}
