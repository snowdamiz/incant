"""Bounded data-only checks. Equality never substitutes for expected gameplay."""
import hashlib
import math
import struct

from .fixture import ENTITY, FRAMES, SCHEMA, SPLIT, TICKS, observation, position, state
from ..native_wasm_restart.verify import equal, numeric_equal, require

RUNS = {'baseline': (0, TICKS), 'prefix': (0, SPLIT),
        'resumed': (SPLIT, TICKS), 'repeated': (SPLIT, TICKS)}
REFUSALS = {
    'corrupt': 'Error: Checkpoint(Integrity)',
    'program': 'Error: Checkpoint(ProgramMismatch)',
    'schema': 'Error: Checkpoint(Invalid("persistent Wasm state"))',
    'clock': 'Error: Checkpoint(Invalid("world/module tick pairing"))',
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def timer_state(tick):
    """Independent closed-form schedule for this fixed timer-driven fixture."""
    rows = [("future", 17, 101)]
    if tick < 20:
        rows.append(("gain", 11, 9 if tick < 9 else 20))
    rows += [(name, 0, due) for name, due in (("stop", 26), ("restart", 29), ("upgain", 34))
             if tick < due]
    elapsed = 0.
    for _ in range(tick):
        elapsed += 1. / 60.
    value = b'INCTMS01' + struct.pack('<QdII', tick, elapsed, len(rows), 80)
    for name, interval, due in sorted(rows):
        value += name.encode().ljust(64, b'\0') + struct.pack('<IIQ', len(name), interval, due)
    return value


def checkpoint(data, tick):
    require(128 <= len(data) <= 1024 * 1024, 'Small fixed checkpoint bound')
    require(data[:8] == b'INCSES01' and data[8:16] == struct.pack('<II', 7, 1), 'Ordinary Wasm v7')
    require(data[96:128] == hashlib.sha256(data[:96] + data[128:]).digest(), 'Checkpoint digest')
    lengths = struct.unpack_from('<4I', data, 80)
    require(sum(lengths) + 128 == len(data) and lengths[2] == 0 and lengths[3] > 0,
            'Exact state/no UI/active audio sections')
    start = 128 + lengths[0]
    payload = data[start:start + lengths[1]]
    current = payload.startswith(b'INCWPS02')
    if current:
        require(payload[104:] == timer_state(tick), 'Exact pending timer schedule and simulation clock')
        payload = payload[8:104]
    require(len(payload) == 96, 'Exact declared state size')
    require(payload[:16] == b'INCWST01' + struct.pack('<II', 1, 24)
            and payload[16:48] == SCHEMA and payload[48:56] == bytes(8), 'Opted-in state declaration')
    saved_tick, watermark = struct.unpack_from('<Qq', payload, 56)
    require(saved_tick == tick and tick <= watermark < 2**63 - 1, 'Saved clock and epoch watermark')
    equal(list(struct.unpack_from('<3Q', payload, 72)), list(state(tick)), 'Exact explicit gameplay state')
    return {'program': data[16:48].hex(), 'origin': data[48:80].hex(), 'state_start': start,
            'watermark': watermark, 'lengths': list(lengths)}


def mutate_checkpoint(data, kind):
    content = bytearray(data)
    if kind == 'corrupt':
        content[-1] ^= 1
        return bytes(content)
    start = 128 + struct.unpack_from('<I', content, 80)[0]
    current = content[start:start + 8] == b'INCWPS02'
    state_start = start + (8 if current else 0)
    if kind == 'program':
        content[16] ^= 1
    elif kind == 'schema':
        content[state_start + 16] ^= 1
    elif kind == 'clock':
        # Keep <= watermark so the specific world/state pairing check is reached.
        struct.pack_into('<Q', content, state_start + 56, SPLIT - 1)
        if current:
            struct.pack_into('<Q', content, start + 112, SPLIT - 1)
    else:
        raise ValueError('Unknown fixed checkpoint mutation')
    content[96:128] = hashlib.sha256(content[:96] + content[128:]).digest()
    return bytes(content)


def run(report, pcm, saved, name):
    start, end = RUNS[name]
    require(type(report.get('tick')) is int and report['tick'] == end, 'Absolute final tick')
    require(type(report.get('frames')) is int and report['frames'] == (end - start) * FRAMES,
            'Additional-tick output clock')
    require(len(pcm) == (end - start) * FRAMES * 8 and report['pcm_sha256'] == sha(pcm), 'PCM frame bytes/digest')
    require('session' not in report and 'render_input' not in report, 'Plain route without restart/render owner')
    equal(report['flipbooks'], [], 'No unrequested animation route')
    logs = report['logs']
    equal(logs, {'dropped': 0, 'entries': [
        {'tick': tick, 'level': 'info', 'message': 'fresh-guest' if tick == start + 1 else 'continued-guest'}
        for tick in range(start + 1, end + 1)]}, 'Fresh VM logs with continued simulation clock')
    observed = report['observations']
    require(observed.get('passed') is True and observed['tick'] == end and observed['tick_rate'] == 60,
            'Complete observation owner')
    expected = observation(start, end)['checks']
    checks = observed['checks']
    require(len(checks) == len(expected), 'Complete absolute observation interval')
    for actual, wanted in zip(checks, expected):
        equal([actual['name'], actual['tick'], actual['passed']], [wanted['name'], wanted['tick'], True],
              'Observation identity/pass')
        numeric_equal(actual['expect'], wanted['expect'], 'Executed observation predicate')
        numeric_equal(actual['actual'], position(wanted['tick']), 'Independent gameplay arithmetic')
    require(set(observed['entities']) == {ENTITY}, 'Selected final entity inventory')
    entity = observed['entities'][ENTITY]
    xyz = position(end)
    numeric_equal(entity, {'translation': xyz, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1],
                          'parent': None, 'velocity': None,
                          'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [*xyz, 1]]},
                  'Exact final observed world')
    samples = [sample for sample, in struct.iter_unpack('<f', pcm)]
    require(all(math.isfinite(n) for n in samples) and any(abs(n) > .001 for n in samples), 'Finite audible PCM')
    return checkpoint(saved, end)


def joined(outputs):
    require(set(outputs) == set(RUNS), 'Four exact fresh-process runs')
    identities = [run(**outputs[name], name=name) for name in RUNS]
    require(len({(item['program'], item['origin']) for item in identities}) == 1, 'One program and original scene')
    require(outputs['baseline']['saved'] == outputs['resumed']['saved'] == outputs['repeated']['saved'],
            'Exact final checkpoint continuation')
    pcm = outputs['baseline']['pcm']
    require(pcm == outputs['prefix']['pcm'] + outputs['resumed']['pcm'], 'Exact joined PCM')
    require(outputs['resumed']['pcm'] == outputs['repeated']['pcm'], 'Fresh resume determinism')
    require(not any(pcm[25 * FRAMES * 8:28 * FRAMES * 8]), 'Stopped transport remains silent')
    require(any(abs(n) > .001 for n, in struct.iter_unpack('<f', pcm[28 * FRAMES * 8:33 * FRAMES * 8])),
            'Restarted transport audible')


def refusal(receipt, name):
    equal([receipt['returncode'], receipt['stdout']], [1, ''], 'Expected failed process without success report')
    if name in REFUSALS:
        equal(receipt['stderr'].strip(), REFUSALS[name], 'Exact checkpoint refusal')
        equal(receipt['outputs_present'], [False, False], 'Admission refusal publishes no files')
    else:
        require(name in ('pcm-exists', 'save-exists'), 'Known publication refusal')
        equal(receipt['stderr'].strip(),
              'Error: Os { code: 17, kind: AlreadyExists, message: "File exists" }',
              'macOS IO AlreadyExists kind')
        require(receipt['sentinel_before'] == receipt['sentinel_after'], 'Existing output bytes/mode preserved')
        equal(receipt['outputs_present'], [True, name == 'save-exists'], 'Separate publication outcomes')
    equal(receipt['staging'], [], 'No abandoned output staging')


def cleanup(inventory, owner, absence, wrapper):
    require(type(wrapper) is int and wrapper > 0 and inventory, 'Nonempty observed inventory')
    seen = set()
    for row in inventory:
        require(set(row) == {'pid', 'pgid', 'source'} and type(row['pid']) is int and row['pid'] > 0
                and type(row['pgid']) is int and row['pgid'] > 0
                and row['source'] in ('wrapper', 'managed-leader', 'sampled-descendant'), 'Observed process identity')
        require(row['pid'] not in seen, 'Distinct observed PIDs')
        seen.add(row['pid'])
    require(wrapper in seen and any(r['pid'] == wrapper and r['source'] == 'wrapper' for r in inventory), 'Wrapper included')
    leaders = {g['pid'] for g in owner['owned']}
    require(leaders and leaders <= seen and all(g['cleanup_confirmed'] is True for g in owner['owned']), 'Managed owner coverage')
    require(len(absence) == len(seen) and {r['pid'] for r in absence} == seen
            and all(type(r['pid']) is int and r['pid_and_group_absent'] is True for r in absence),
            'Exact observed PID absence inventory')
