"""Independent raw profile-v2 and interval-comparison audit; no player execution."""
import copy


def metrics(report):
    ticks = report['tick']
    assert type(ticks) is int and ticks == 2156
    p = report['profile']
    assert p['version'] == 2 and p['completed_frames'] == ticks
    assert p['failed_frames'] == p['dropped_source_samples'] == 0
    assert p['tracy_process_transport'] is False
    wall = p['frame_wall']
    assert (wall['version'], wall['unit'], wall['bucket_width_ns'], wall['max_ns']) == (1, 'ns', 10000, 100000000)
    assert wall['attempted_frames'] == wall['successful_frames'] == ticks
    assert all(wall[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
    assert wall['saturated'] is False
    phases = {r['name']: r for r in wall['phases']}
    assert len(phases) == len(wall['phases'])
    result = {}
    rank = (ticks * 95 + 99) // 100
    for name in ('player.frame', 'script.tick'):
        row = phases[name]
        assert row['samples'] == ticks
        assert all(row[k] == 0 for k in ('missing_frames', 'invalid_frames', 'overflow_samples'))
        counts = row['counts']
        assert len(counts) == 10001 and all(type(n) is int and 0 <= n <= ticks for n in counts)
        assert sum(counts) == ticks
        remaining = rank
        for index, count in enumerate(counts):
            remaining -= count
            if remaining <= 0:
                interval = {'lower': max(0, (index - 1) * 10000 + 1), 'upper': index * 10000}
                assert [row['p95_lower_ns'], row['p95_upper_ns']] == list(interval.values())
                result[name + '.p95_wall_ns'] = interval
                break
    allocations = p['rust_allocations']
    assert allocations['installed'] is True and allocations['saturated'] is False
    assert allocations['failures'] == 0
    peak = allocations['process_peak_requested_bytes']
    assert type(peak) is int and peak > 0
    result['rust.process_peak_requested_bytes'] = {'lower': peak, 'upper': peak}
    return result


def compare(current, baseline):
    assert current['match'] == baseline['match']
    states = {}
    for name, now in current['metrics'].items():
        old = baseline['metrics'][name]
        assert 0 <= now['lower'] <= now['upper'] and 0 <= old['lower'] <= old['upper']
        states[name] = ('pass' if now['upper'] * 20 <= old['lower'] * 21 else
                        'regression' if now['lower'] * 20 > old['upper'] * 21 else 'inconclusive')
    script = current['metrics']['script.tick.p95_wall_ns']
    budget = 'pass' if script['upper'] <= 2000000 else ('fail' if script['lower'] > 2000000 else 'inconclusive')
    return {'regression_limit_percent': 5, 'metrics': states, 'script_2ms_p95': budget,
            'passed': budget == 'pass' and all(v == 'pass' for v in states.values()), 'phase_gate_passed': False}



def controls(report, baseline, candidate):
    """Reject malformed data, identity changes and uncertain threshold passes."""
    for field in ('missing_frames', 'invalid_frames', 'overflow_samples', 'p95_lower_ns'):
        damaged = copy.deepcopy(report)
        row = next(r for r in damaged['profile']['frame_wall']['phases'] if r['name'] == 'script.tick')
        row[field] += 1
        try:
            metrics(damaged)
        except AssertionError:
            pass
        else:
            raise AssertionError('Accepted malformed ' + field)
    damaged = copy.deepcopy(report)
    next(r for r in damaged['profile']['frame_wall']['phases'] if r['name'] == 'script.tick')['counts'][0] += 1
    try:
        metrics(damaged)
    except AssertionError:
        pass
    else:
        raise AssertionError('Accepted wrong histogram mass')
    changed = copy.deepcopy(candidate)
    changed['match']['project_sha256'] = 'different-workload'
    try:
        compare(changed, baseline)
    except AssertionError:
        pass
    else:
        raise AssertionError('Accepted changed workload')
    name = 'script.tick.p95_wall_ns'
    old = {'match': {}, 'metrics': {name: {'lower': 100, 'upper': 100}}}
    for lower, upper, state in ((105, 105, 'pass'), (106, 106, 'regression'), (105, 106, 'inconclusive')):
        now = {'match': {}, 'metrics': {name: {'lower': lower, 'upper': upper}}}
        assert compare(now, old)['metrics'][name] == state
    for lower, upper, state in ((2000000, 2000000, 'pass'), (2000001, 2000001, 'fail'), (1990001, 2010000, 'inconclusive')):
        now = {'match': {}, 'metrics': {name: {'lower': lower, 'upper': upper}}}
        assert compare(now, old)['script_2ms_p95'] == state
    return 12
