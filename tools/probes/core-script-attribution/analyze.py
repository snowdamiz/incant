"""Correlated observations only. No marginal-quantile subtraction or gate verdict."""
from common import TICKS
from native_package import require
from mac_performance import interval, integer, WIDTH, MAX_NS

REGIONS = ['initial_checks', 'snapshots', 'group_setup', 'api_setup', 'capabilities',
           'undo_capture', 'view_exposure', 'callbacks', 'revoke_checks', 'validation',
           'transaction_stage', 'scope_cleanup', 'publication', 'heap_observation']
NUMERIC = ['prepare', 'change_ticks', 'publish_globals']


def vector(value, size, name, maximum=2**64 - 1):
    require(isinstance(value, list) and len(value) == size, 'Invalid ' + name + ' shape')
    require(all(integer(v, name) <= maximum for v in value), 'Invalid ' + name + ' value')
    return value


def analyze(report, expected_sources, navigation=False, budget=False):
    require(not (navigation and budget), 'Combined private diagnostic is not admitted')
    p = report['profile']
    require(type(p['version']) is int and p['version'] == 4, 'Only diagnostic v4 is admitted')
    require(report['tick'] == TICKS and p['completed_frames'] == TICKS and p['failed_frames'] == 0
            and p['dropped_source_samples'] == 0 and p['tracy_process_transport'] is False,
            'Incomplete diagnostic process')
    heap = p['quickjs_heap']
    require(integer(heap['version'], 'heap version') == 1 and heap['mode'] == 'maintained'
            and heap['sampling'] == 'script_tick_end_per_vm' and heap['samples'] == TICKS
            and heap['detailed_samples'] == heap['invalid_samples'] == 0
            and heap['sampled_peak_estimated_used_bytes'] is None
            and heap['latest']['estimated_used_bytes'] is None
            and heap['latest']['limit_bytes'] == 32 * 1024**2, 'Heap mode/coverage changed')
    for name, peak in [('allocated_bytes', 'sampled_peak_allocated_bytes'),
                       ('allocation_count', 'sampled_peak_allocation_count')]:
        require(integer(heap[peak], peak) >= integer(heap['latest'][name], name), 'Invalid heap peak')
    d = p['script_phase_diagnostic']
    require(integer(d['schema'], 'diagnostic schema') == (3 if budget else 2 if navigation else 1) and d['diagnostic_only'] is True
            and d['eligible_for_performance_gate'] is False and d['complete'] is True,
            'Incomplete or non-diagnostic attribution')
    require(d['max_rows'] == 4096 and 0 < integer(d['raw_row_bytes'], 'row size') <= 1024
            and d['raw_storage_bound_bytes'] == 4096 * d['raw_row_bytes']
            and d['attempted'] == TICKS and d['open'] == 0,
            'Storage/attempt contract differs')
    require(d['regions'] == REGIONS and d['numeric_regions'] == NUMERIC, 'Unknown phase schema')
    require(('budget_check_regions' in d) == budget, 'Requested budget mode differs from report')
    rows = d['rows']
    require(isinstance(rows, list) and len(rows) == TICKS, 'Missing correlated frames')
    sources = p['scripts']
    require(len(sources) == 2 and [s['source'] for s in sources] == expected_sources,
            'Actual callback sources differ from functional baseline')
    slots = [integer(s['slot'], 'source slot') for s in sources]
    require(len(set(slots)) == 2 and all(s < 64 for s in slots), 'Invalid source slots')
    bins = [0] * (MAX_NS // WIDTH + 1)
    whole = []
    for i, row in enumerate(rows, 1):
        require(('budget_checks' in row) == budget, 'Requested budget mode differs from row')
        require(integer(row['frame'], 'frame') == integer(row['ordinal'], 'ordinal') == i and row['valid'] is True
                and row['success'] is True and row['unwound'] is False
                and integer(row['whole_tick_calls'], 'tick calls') == 1, 'Invalid/missing/reordered row')
        region = vector(row['regions_ns'], len(REGIONS), 'region')
        nested = vector(row['numeric_ns'], len(NUMERIC), 'numeric')
        calls = vector(row['numeric_calls'], len(NUMERIC), 'numeric calls', 65535)
        source_ns = vector(row['source_ns'], 64, 'source ns')
        source_calls = vector(row['source_calls'], 64, 'source calls', 65535)
        require(calls == [1, 1, 1], 'Numeric path coverage changed')
        require(all(n == int(slot in slots) for slot, n in enumerate(source_calls)),
                'Callback count changed')
        require(all(source_ns[slot] == 0 for slot in range(64) if slot not in slots),
                'Unknown callback contribution')
        trace = integer(row['trace_ns'], 'trace')
        tick = integer(row['whole_tick_ns'], 'whole tick')
        require(trace == sum(region) and trace >= tick and tick <= MAX_NS,
                'Whole/partition coverage differs')
        require(sum(source_ns) <= region[7] and nested[0] <= region[9]
                and nested[1] + nested[2] <= region[11], 'Nested spans exceed their parent')
        whole.append(tick)
        bins[(tick + WIDTH - 1) // WIDTH] += 1
    zones = [z for z in p['zones'] if z['name'] == 'script.tick']
    require(len(zones) == 1, 'Missing unique ScriptTick total')
    totals = zones[0]['totals']
    require(totals['calls'] == TICKS and totals['wall_ns'] == sum(whole)
            and totals['max_wall_ns'] == max(whole) and totals['unwound'] == 0
            and totals['unavailable_cpu_samples'] == 0, 'Whole timer correspondence differs')
    for source in sources:
        slot, totals = source['slot'], source['totals']
        require(totals['calls'] == TICKS and totals['unwound'] == 0
                and totals['unavailable_cpu_samples'] == 0
                and sum(r['source_ns'][slot] for r in rows) <= totals['wall_ns'],
                'Callback aggregate exceeds enclosing source timer')
    frame = p['frame_wall']
    require(integer(frame['version'], 'frame version') == 1 and frame['unit'] == 'ns'
            and frame['bucket_width_ns'] == WIDTH and frame['max_ns'] == MAX_NS
            and frame['attempted_frames'] == frame['successful_frames'] == TICKS
            and all(frame[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
            and frame['saturated'] is False, 'Frame protocol differs')
    hist = [r for r in frame['phases'] if r['name'] == 'script.tick']
    require(len(hist) == 1 and hist[0]['counts'] == bins, 'Raw whole-tick histogram differs')
    bounds = interval(hist[0], TICKS)
    # Selection is from the same measured whole ticks, never marginal phases.
    tail = sorted(rows, key=lambda r: (-r['whole_tick_ns'], r['frame']))[:(TICKS + 19) // 20]

    def aggregate(selected):
        return {'frames': [r['frame'] for r in selected], 'count': len(selected),
                'whole_tick_total_ns': sum(r['whole_tick_ns'] for r in selected),
                'trace_total_ns': sum(r['trace_ns'] for r in selected),
                'region_totals_ns': [sum(r['regions_ns'][i] for r in selected) for i in range(len(REGIONS))],
                'numeric_nested_totals_ns': [sum(r['numeric_ns'][i] for r in selected) for i in range(3)],
                'callback_nested_totals_ns': {str(s): sum(r['source_ns'][s] for r in selected) for s in slots},
                'residual_totals_ns': {
                    'callback_region': sum(r['regions_ns'][7] - sum(r['source_ns']) for r in selected),
                    'validation': sum(r['regions_ns'][9] - r['numeric_ns'][0] for r in selected),
                    'scope_cleanup': sum(r['regions_ns'][11] - sum(r['numeric_ns'][1:]) for r in selected),
                    'trace_minus_whole': sum(r['trace_ns'] - r['whole_tick_ns'] for r in selected)}}
    result = {'format': 'incant-script-attribution-observation', 'version': 1,
            'diagnostic_only': True, 'eligible_for_performance_gate': False,
            'histogram_reconstructed': True, 'instrumented_whole_p95_interval_ns': bounds,
            'regions': REGIONS, 'numeric_regions': NUMERIC,
            'sources': [{'slot': s['slot'], 'source': s['source']} for s in sources],
            'all_frames': aggregate(rows), 'same_frame_tail': aggregate(tail),
            'tail_rows': tail, 'limits': [
                'Instrumentation changes the measured work; no normal shipping 2ms/5% verdict',
                'Nested spans are subsets, not additive exclusive time',
                'Residuals retain observer bookkeeping and uninstrumented work',
                'No timing subtraction between independent marginal quantiles',
                'One fixed instrumented route identifies candidates, not an optimization gain']}

    if navigation:
        from navigation_analysis import analyze_navigation
        result['navigation'] = analyze_navigation(d)
    if budget:
        from budget_analysis import analyze_budget
        result['budget_checks'] = analyze_budget(d, result['same_frame_tail']['frames'])
    return result
