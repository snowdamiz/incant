"""Independent bounded arithmetic and histogram reconstruction from raw reports."""
import copy
import hashlib
import math

TICKS, COUNT, PCM_BYTES = 3600, 1024, 23_040_000
ALPHABET = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'


def owner(n):
    return ''.join(ALPHABET[((10000 + n) >> (5 * i)) & 31] for i in reversed(range(26)))


def silent_hash():
    h = hashlib.sha256()
    for offset in range(0, PCM_BYTES, 65536):
        h.update(bytes(min(65536, PCM_BYTES - offset)))
    return h.hexdigest()


def functional(report, indices):
    assert report['tick'] == TICKS and report['frames'] == 2_880_000
    assert report['logs'] == {'entries': [], 'dropped': 0}
    assert report['sources'] == report['flipbooks'] == []
    observations = report['observations']
    assert observations['passed'] is True and observations['tick_rate'] == 60
    assert set(observations['entities']) == {owner(n) for n in indices}
    for n in indices:
        row = observations['entities'][owner(n)]
        x = n / 8
        for tick in range(1, TICKS + 1):
            if (tick - 1) % 600 < 300:
                x += 6 * (1 / 60)
        assert row['parent'] is None and row['velocity'] is None
        assert row['rotation'] == [0, 0, 0, 1] and row['scale'] == [1, 1, 1]
        assert len(row['translation']) == 3
        assert all(math.isfinite(a) and abs(a - b) <= 1e-9
                   for a, b in zip(row['translation'], [x, TICKS, 6]))
        assert row['world_transform'] == [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], row['translation'] + [1]]
    return {key: value for key, value in report.items() if key != 'profile'}


def integer(v):
    assert type(v) is int and 0 <= v <= 2**64 - 1
    return v


def metrics(report, phase):
    assert phase in ('script.tick', 'script.static_module') and report['tick'] == TICKS
    p = report['profile']
    assert p['version'] == 2 and p['completed_frames'] == TICKS and p['failed_frames'] == 0
    assert p['dropped_source_samples'] == 0 and p['tracy_process_transport'] is False
    f = p['frame_wall']
    assert (f['version'], f['unit'], f['bucket_width_ns'], f['max_ns']) == (1, 'ns', 10000, 100000000)
    assert f['attempted_frames'] == f['successful_frames'] == TICKS and f['saturated'] is False
    assert all(f[k] == 0 for k in ('failed_frames', 'invalid_frames', 'active_frames'))
    rows = {r['name']: r for r in f['phases']}
    assert len(rows) == len(f['phases'])
    result = {}
    for name in ('player.frame', phase):
        r = rows[name]
        assert r['samples'] == TICKS and all(r[k] == 0 for k in ('missing_frames', 'invalid_frames', 'overflow_samples'))
        assert len(r['counts']) == 10001 and sum(integer(v) for v in r['counts']) == TICKS
        rank, cumulative = math.ceil(TICKS * 95 / 100), 0
        for index, count in enumerate(r['counts']):
            cumulative += count
            if cumulative >= rank:
                bounds = {'lower': 0 if index == 0 else (index - 1) * 10000 + 1,
                          'upper': index * 10000}
                break
        assert bounds == {'lower': r['p95_lower_ns'], 'upper': r['p95_upper_ns']}
        result[name + '.p95_wall_ns'] = bounds
    a = p['rust_allocations']
    assert a['installed'] is True and a['saturated'] is False
    peak = integer(a['process_peak_requested_bytes'])
    assert peak > 0
    result['rust.process_peak_requested_bytes'] = {'lower': peak, 'upper': peak}
    return result


def controls(report):
    """Retained data mutations must be observable to the independent verifier."""
    refused = 0
    for mutate in (
        lambda r: r['profile']['frame_wall'].__setitem__('successful_frames', TICKS - 1),
        lambda r: r['profile']['frame_wall'].__setitem__('saturated', True),
        lambda r: next(x for x in r['profile']['frame_wall']['phases'] if x['name'] == 'script.tick').__setitem__('overflow_samples', 1),
        lambda r: next(x for x in r['profile']['frame_wall']['phases'] if x['name'] == 'script.tick').__setitem__('p95_upper_ns', 7),
    ):
        altered = copy.deepcopy(report)
        mutate(altered)
        try:
            metrics(altered, 'script.tick')
        except AssertionError:
            refused += 1
        else:
            raise AssertionError('Malformed measurement was accepted')
    altered = copy.deepcopy(report)
    altered['observations']['entities'][owner(0)]['translation'][0] += 0.01
    try:
        functional(altered, [0, COUNT // 2, COUNT - 1])
    except AssertionError:
        refused += 1
    else:
        raise AssertionError('Wrong row accepted')
    return refused
