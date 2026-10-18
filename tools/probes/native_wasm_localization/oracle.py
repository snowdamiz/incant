"""Bounded data-only proof: expected rows, observable motion, copied state and PCM."""
import hashlib
import math
import struct

from ..native_wasm_restart.verify import equal, numeric_equal, require
from . import fixture as f

RUNS = {'baseline': (0, f.TICKS), 'prefix': (0, f.SPLIT),
        'resumed': (f.SPLIT, f.TICKS), 'repeated': (f.SPLIT, f.TICKS)}
sha = lambda data: hashlib.sha256(data).hexdigest()


def gameplay(report, pcm, start, end):
    require(type(report.get('tick')) is int and report['tick'] == end, 'Exact accepted tick')
    require(type(report.get('frames')) is int and report['frames'] == (end - start) * f.FRAMES,
            'Exact audio frame clock')
    require(len(pcm) == (end - start) * f.FRAMES * 8 and report['pcm_sha256'] == sha(pcm), 'PCM identity')
    samples = [v for v, in struct.iter_unpack('<f', pcm)]
    require(all(math.isfinite(v) for v in samples) and any(abs(v) > .001 for v in samples), 'Finite nonzero PCM')
    equal(report['logs'], {'dropped': 0, 'entries': [
        {'tick': tick, 'level': 'info', 'message': f.LOCALES[(tick - 1) % 4]}
        for tick in range(start + 1, end + 1)]}, 'Committed callback locale logs')
    observed = report['observations']
    require(observed['passed'] is True and observed['tick'] == end and observed['tick_rate'] == 60,
            'Complete selected observation')
    wanted = f.observation(start, end)['checks']
    require(len(observed['checks']) == len(wanted), 'No omitted tick predicates')
    for actual, expected in zip(observed['checks'], wanted):
        equal([actual['name'], actual['tick'], actual['passed']],
              [expected['name'], expected['tick'], True], 'Predicate identity')
        numeric_equal(actual['expect'], expected['expect'], 'Executed exact predicate')
        numeric_equal(actual['actual'], f.position(expected['tick']), 'Independent row digest and total')
    require(set(observed['entities']) == {f.ENTITY}, 'Exact observed entity set')
    xyz = f.position(end)
    numeric_equal(observed['entities'][f.ENTITY], {
        'translation': xyz, 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1], 'parent': None,
        'velocity': None, 'world_transform': [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [*xyz, 1]]},
        'Exact final runtime entity')


def checkpoint(data, tick):
    require(128 <= len(data) <= 1024**2, 'Checkpoint bound')
    require(data[:16] == b'INCSES01' + struct.pack('<II', 7, 1), 'Ordinary Wasm v7 checkpoint')
    require(data[96:128] == hashlib.sha256(data[:96] + data[128:]).digest(), 'Checkpoint integrity')
    lengths = struct.unpack_from('<4I', data, 80)
    require(sum(lengths) + 128 == len(data) and lengths[2] == 0 and lengths[3] > 0, 'Exact sections')
    start = 128 + lengths[0]
    payload = data[start:start + lengths[1]]
    require(len(payload) == 80 and payload[:16] == b'INCWST01' + struct.pack('<II', 1, 8)
            and payload[16:48] == f.SCHEMA and payload[48:56] == bytes(8), 'Explicit eight-byte state only')
    saved_tick, watermark, count = struct.unpack_from('<QqQ', payload, 56)
    require(saved_tick == count == tick and tick <= watermark < 2**63 - 1, 'Saved guest count/clock')
    return data[16:80]


def run(report, pcm, saved, name):
    start, end = RUNS[name]
    require('session' not in report and 'render_input' not in report, 'Plain headless route')
    equal(report['flipbooks'], [], 'No animation route')
    gameplay(report, pcm, start, end)
    return checkpoint(saved, end)


def joined(outputs, reference, reference_pcm):
    require(set(outputs) == set(RUNS), 'Four exact checkpoint runs')
    identities = [run(**outputs[name], name=name) for name in RUNS]
    require(len(set(identities)) == 1, 'One exported program/origin')
    require(outputs['baseline']['saved'] == outputs['resumed']['saved'] == outputs['repeated']['saved'],
            'Exact checkpoint after fresh-VM continuation')
    require(outputs['baseline']['pcm'] == outputs['prefix']['pcm'] + outputs['resumed']['pcm']
            and outputs['resumed']['pcm'] == outputs['repeated']['pcm'], 'Exact PCM continuation')
    require(reference['passed'] is True and reference['gpu_submitted'] is False
            and reference['cpu_render_input'] is False and reference['script_source_parsing'] is True,
            'Actual authored TS native CPU reference')
    gameplay(reference, reference_pcm, 0, f.TICKS)
    equal(reference['logs'], outputs['baseline']['report']['logs'], 'Rust/TS exact locale observations')
    numeric_equal(reference['observations'], outputs['baseline']['report']['observations'], 'Rust/TS exact selected gameplay')
    require(reference_pcm == outputs['baseline']['pcm'], 'Rust/TS actual PCM bytes')


def refusal(receipt, kind):
    require(kind in f.FAILURES.values(), 'Known fixed refusal')
    equal([receipt['returncode'], receipt['stdout']], [1, ''], 'Failed callback cannot publish success report')
    error = 'Boundary(Limit("localization query quota"))' if kind == 'quota' else 'Trap(1)'
    equal(receipt['stderr'].strip(), f'Error: Session(Module({error}))', 'Exact callback failure classification')
    equal(receipt['outputs_present'], [False, False], 'No partial PCM/checkpoint publication')
    equal(receipt['staging'], [], 'No leaked publication staging')
