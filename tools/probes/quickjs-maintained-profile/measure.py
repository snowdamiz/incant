#!/usr/bin/env python3
"""Execute the reviewed six-process observation experiment on one existing export.

Build/source intake, functional qualification and the all-lane quiet grant precede
invocation. This runner neither builds nor approves those prerequisites or a gate.
"""
import argparse
from contextlib import contextmanager
import json
from pathlib import Path
import signal
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools'))
import mac_performance as performance
from native_package import read, require, write_json

PLAN = ROOT / 'docs/spikes/evidence/quickjs-maintained-shipping-protocol/plan.json'
PLAN_SHA256 = '8c14c925f8044d02681b06cbda1857937201ba09a1e722d218edb4425828caa9'
ORDER = ('D0', 'M0', 'M1', 'D1', 'D2', 'M2')
CHILD_SECONDS = 45
SEQUENCE_SECONDS = 300


@contextmanager
def bounded_sequence(seconds=SEQUENCE_SECONDS):
    """Latch cancellation; raise only at checkpoints with managed child ownership.

Signals cannot interrupt Popen construction or mandatory cleanup. Repeated signals
are suppressed until handlers are restored. This is an execution deadline, not a
hard wall-clock bound on filesystem IO, existing bounded host probes or reaping.
"""
    require(threading.current_thread() is threading.main_thread()
            and hasattr(signal, 'setitimer'), 'Sequence requires the POSIX main thread')
    require(signal.getitimer(signal.ITIMER_REAL) == (0.0, 0.0), 'Another process alarm is active')
    signals = (signal.SIGALRM, signal.SIGTERM, signal.SIGINT)
    previous = {number: signal.getsignal(number) for number in signals}
    deadline = time.monotonic() + seconds
    cause = None

    def cancelled(number, *_):
        nonlocal cause
        if cause is None:
            if number == signal.SIGALRM:
                cause = TimeoutError('Observation sequence exceeded its deadline')
            elif number == signal.SIGINT:
                cause = KeyboardInterrupt('Observation sequence interrupted by SIGINT')
            else:
                cause = InterruptedError('Observation sequence interrupted by SIGTERM')
            signal.setitimer(signal.ITIMER_REAL, 0)

    def check():
        if cause is None and time.monotonic() >= deadline:
            cancelled(signal.SIGALRM)
        if cause is not None:
            raise cause

    try:
        for number in signals:
            signal.signal(number, cancelled)
        signal.setitimer(signal.ITIMER_REAL, seconds)
        yield check
        check()
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for number, handler in previous.items():
            signal.signal(number, handler)


def budget(interval):
    lower = performance.integer(interval['lower'], 'script lower')
    upper = performance.integer(interval['upper'], 'script upper')
    require(lower <= upper, 'Reversed script interval')
    return 'pass' if upper <= 2_000_000 else 'fail' if lower > 2_000_000 else 'inconclusive'


def observation(report, mode, ticks):
    identity = performance.profile_instrumentation(report)
    require(identity['profile_version'] == 3 and identity['quickjs_heap_mode'] == mode,
            'Returned heap instrumentation differs from protocol arm')
    heap = report['profile']['quickjs_heap']
    require(heap['samples'] == ticks and heap['latest']['limit_bytes'] == 32 * 1024**2,
            'Heap observation coverage or native limit differs')
    rows = report['profile']['scripts']
    require(isinstance(rows, list) and len(rows) == 2, 'Expected exactly two script sources')
    sources, means = [], []
    for row in rows:
        source = row['source']
        require(isinstance(source, list) and len(source) == 16
                and all(type(n) is int and 0 <= n <= 255 for n in source), 'Invalid script source ID')
        sources.append(tuple(source))
        totals = row['totals']
        require(performance.integer(totals['calls'], 'source calls') == ticks
                and performance.integer(totals['unwound'], 'source unwinds') == 0
                and performance.integer(totals['unavailable_cpu_samples'], 'source missing CPU') == 0,
                'Incomplete script source context')
        means.append({'source': source,
                      'wall_mean_ns': [performance.integer(totals['wall_ns'], 'source wall total'), ticks],
                      'thread_cpu_mean_ns': [performance.integer(totals['thread_cpu_ns'], 'source CPU total'), ticks]})
    require(sources == sorted(set(sources)), 'Script source rows are duplicate or unordered')
    return means


def interval_difference(detailed, maintained):
    # Independent quantile intervals, never a per-tick telemetry duration.
    budget(detailed)
    budget(maintained)
    return {'lower': detailed['lower'] - maintained['upper'],
            'upper': detailed['upper'] - maintained['lower']}


def measure_sequence(package, output, functional_report, functional_pcm, plan_path=PLAN):
    raw_plan = read(plan_path.absolute(), 1024**2)
    require(performance.digest(raw_plan) == PLAN_SHA256, 'Protocol plan differs from reviewed bytes')
    plan = json.loads(raw_plan)
    workload = plan['workload']
    receipt_path = Path(workload['input_receipt'])
    input_receipt = read(receipt_path, 1024**2)
    require(performance.digest(input_receipt) == workload['input_receipt_sha256'],
            'Workload input receipt changed')
    inputs = json.loads(input_receipt)
    recording, observe = Path(inputs['input']), Path(inputs['observe'])
    frozen_inputs = {}
    for path, key in [(recording, 'input_sha256'), (observe, 'observe_sha256')]:
        frozen_inputs[key] = read(path, 8 * 1024**2)
        require(performance.digest(frozen_inputs[key]) == workload[key], 'Workload input bytes changed')
    package, output = package.absolute(), output.absolute()
    _, exported, identities, rate = performance.package_inputs(package)
    require(exported['project_sha256'] == workload['project_sha256']
            and exported['physics_mode'] == workload['physics_mode']
            and exported['target'] == plan['build']['target'] and rate == workload['tick_rate']
            and identities['program'] is None, 'Package differs from approved TypeScript workload')
    require(exported.get('startup_ui_viewport') == workload['ui_viewport'], 'UI viewport differs')
    functional_raw = read(functional_report.absolute(), 16 * 1024**2)
    expected = json.loads(functional_raw)
    require('profile' not in expected and expected['tick'] == workload['ticks']
            and expected['observations']['passed'] is True, 'Expected complete unprofiled functional control')
    pcm = read(functional_pcm.absolute(), 128 * 1024**2)
    require(len(pcm) == workload['ticks'] * workload['sample_rate'] // rate * 8
            and expected['pcm_sha256'] == performance.digest(pcm), 'Functional PCM binding differs')
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(),
            'Choose a new output directory')
    require(package != output and package not in output.parents and output not in package.parents,
            'Package/output overlap')
    output.mkdir()
    recording, observe = output / 'input.json', output / 'observe.json'
    for path, key in [(recording, 'input_sha256'), (observe, 'observe_sha256')]:
        path.write_bytes(frozen_inputs[key])
    state = {'format': 'incant-heap-observation-sequence', 'version': 1,
             'protocol_plan_sha256': PLAN_SHA256, 'order': list(ORDER),
             'anchors': {'maintained': 'M0', 'detailed': 'D0'}, 'identities': identities,
             'functional_report_sha256': performance.digest(functional_raw),
             'functional_pcm_sha256': performance.digest(pcm), 'attempts': [],
             'child_timeout_seconds': CHILD_SECONDS, 'sequence_execution_deadline_seconds': SEQUENCE_SECONDS,
             'status': 'running', 'phase_gate_passed': False, 'started_ns': time.time_ns(),
             'limits': ['Observation-method experiment; no guest-loop improvement claim',
                        'Cross-mode differences describe independent quantiles, not telemetry p95',
                        'Callback context is a mean; no timing subtraction',
                        'Cancellation waits for safe checkpoints after filesystem IO, bounded host probes and mandatory cleanup',
                        'Source/build/functional/quiet prerequisites require separate reviewed evidence']}
    measured, reference_machine, reference_sources = {}, None, None
    try:
        with bounded_sequence() as cancel:
            for label in ORDER:
                cancel()
                mode = 'detailed' if label.startswith('D') else 'maintained'
                attempt = {'label': label, 'heap_mode': mode, 'status': 'started'}
                state['attempts'].append(attempt)
                write_json(output / 'sequence.json', state)
                performance.measure(package, output / label, workload['ticks'], recording, observe,
                                    heap_mode=mode, process_timeout=CHILD_SECONDS, cancel=cancel)
                cancel()
                current = performance.load_baseline(output / label / 'measurement.json')
                require(current['identities'] == identities, 'Candidate package changed between arms')
                require(current['match']['inputs'] == {'input': workload['input_sha256'], 'observe': workload['observe_sha256']},
                        'Measurement input identities differ')
                machine = current['match']['machine']
                if reference_machine is None:
                    reference_machine = machine
                require(machine == reference_machine, 'Mac configuration changed between arms')
                raw = json.loads(read(output / label / 'stdout.json', 16 * 1024**2))
                context = observation(raw, mode, workload['ticks'])
                sources = [row['source'] for row in context]
                if reference_sources is None:
                    reference_sources = sources
                require(sources == reference_sources, 'Script source identities changed between arms')
                logical = dict(raw)
                logical.pop('profile')
                require(logical == expected, 'Timed functional output differs from unprofiled control')
                require(read(output / label / 'output.pcm', 128 * 1024**2) == pcm,
                        'Timed PCM differs from unprofiled control')
                measured[label] = current
                attempt.update(status='complete', profile_instrumentation=current['match']['profile_instrumentation'],
                               script_2ms_p95=budget(current['metrics']['script.tick.p95_wall_ns']),
                               source_context_means=context,
                               measurement_sha256=performance.digest(read(output / label / 'measurement.json', 16 * 1024**2)))
                if label not in ('M0', 'D0'):
                    anchor = 'D0' if mode == 'detailed' else 'M0'
                    attempt['stability_anchor'] = anchor
                    attempt['same_mode_stability'] = performance.comparison(current, measured[anchor])
                write_json(output / 'sequence.json', state)
                cancel()
            state['descriptive_detailed_minus_maintained'] = {
                str(index): interval_difference(measured['D' + str(index)]['metrics']['script.tick.p95_wall_ns'],
                                                measured['M' + str(index)]['metrics']['script.tick.p95_wall_ns'])
                for index in range(3)}
            require(performance.package_inputs(package)[2] == identities, 'Candidate package changed')
            cancel()
        state['status'] = 'complete'
        return state
    except BaseException as error:
        state['status'] = 'failed'
        state['error'] = type(error).__name__ + ': ' + str(error)
        if state['attempts'] and state['attempts'][-1]['status'] == 'started':
            state['attempts'][-1].update(status='failed', error=state['error'])
        raise
    finally:
        state['ended_ns'] = time.time_ns()
        write_json(output / 'sequence.json', state)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--functional-report', type=Path, required=True)
    parser.add_argument('--functional-pcm', type=Path, required=True)
    parser.add_argument('--plan', type=Path, default=PLAN)
    args = parser.parse_args()
    result = measure_sequence(args.package, args.output, args.functional_report, args.functional_pcm, args.plan)
    print(json.dumps({'sequence': str(args.output / 'sequence.json'), 'status': result['status'],
                      'phase_gate_passed': False}))
