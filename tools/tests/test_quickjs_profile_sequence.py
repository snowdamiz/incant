"""Synthetic sequence/refusal controls and real trusted-Python tree cleanup.

No engine build, game execution, GPU work or performance measurement is performed.
"""
from contextlib import contextmanager
import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
import mac_performance
import native_process
from test_mac_performance import current_report
from test_native_process import CHILD, GRANDCHILD, PARENT

RUNNER = TOOLS / 'probes/quickjs-maintained-profile/measure.py'
spec = importlib.util.spec_from_file_location('quickjs_profile_sequence', RUNNER)
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class SequenceTests(unittest.TestCase):
    @contextmanager
    def fixture(self, mutate=None, mutate_pcm=None):
        # Small synthetic PCM/input bytes; production uses the hash-bound plan.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            package = root / 'package'; package.mkdir()
            output = root / 'sequence'
            pcm = b'x' * 800
            logical = {'tick': 100, 'observations': {'passed': True},
                       'pcm_sha256': mac_performance.digest(pcm), 'state': 'fixture'}
            functional = root / 'functional.json'
            functional.write_text(json.dumps(logical))
            functional_pcm = root / 'functional.pcm'; functional_pcm.write_bytes(pcm)
            inputs = {}
            for name in ('input', 'observe'):
                path = root / (name + '.json'); path.write_text('{}')
                inputs[name] = str(path)
                inputs[name + '_sha256'] = mac_performance.digest(path.read_bytes())
            receipt = root / 'inputs.json'; receipt.write_text(json.dumps(inputs))
            plan = {'workload': {'input_receipt': str(receipt),
                    'input_receipt_sha256': mac_performance.digest(receipt.read_bytes()),
                    'input_sha256': inputs['input_sha256'], 'observe_sha256': inputs['observe_sha256'],
                    'project_sha256': 'fixture', 'physics_mode': 'fast', 'ticks': 100,
                    'tick_rate': 60, 'sample_rate': 60, 'ui_viewport': '640x480'},
                    'build': {'target': 'aarch64-apple-darwin'}}
            plan_path = root / 'plan.json'; plan_path.write_text(json.dumps(plan))
            exported = {'target': 'aarch64-apple-darwin', 'codegen': {}, 'physics_mode': 'fast',
                        'compiler': {}, 'project_sha256': 'fixture', 'startup_ui_viewport': '640x480',
                        'profiling': {'compiler': {'features': {}}}}
            identity = {'data': {}, 'script_source_sha256': 'fixture', 'program': None,
                        'player_sha256': 'one-binary'}
            calls = []

            def run(argv, **options):
                # The actual helper writes argv, validates the raw report, and
                # load_baseline rederives its hash-bound metrics/identity.
                path = Path(argv[argv.index('--output') + 1])
                label = path.parent.name
                before = json.loads((output / 'sequence.json').read_text())
                self.assertEqual(before['anchors'], {'maintained': 'M0', 'detailed': 'D0'})
                self.assertEqual(before['order'], list(subject.ORDER))
                self.assertEqual(before['attempts'][-1]['status'], 'started')
                mode = 'detailed' if '--profile-heap-detail' in argv else 'maintained'
                report = current_report(mode)
                report.update(logical)
                report['profile']['quickjs_heap']['latest']['limit_bytes'] = 32 * 1024**2
                report['profile']['scripts'] = [
                    {'source': [0] * 15 + [index], 'totals': {'calls': 100, 'unwound': 0,
                     'unavailable_cpu_samples': 0, 'wall_ns': 1000, 'thread_cpu_ns': 500}}
                    for index in (1, 2)]
                if mutate is not None:
                    mutate(label, report)
                options['stdout'].write(json.dumps(report).encode())
                path.write_bytes(mutate_pcm(label, pcm) if mutate_pcm else pcm)
                calls.append((label, argv, options))
                return subprocess.CompletedProcess(argv, 0)

            with patch.object(subject, 'PLAN_SHA256', mac_performance.digest(plan_path.read_bytes())), \
                    patch.object(mac_performance, 'package_inputs', return_value=(package / 'player', exported, identity, 60)), \
                    patch.object(mac_performance, 'mac_identity', return_value={'model': 'fixture'}), \
                    patch.object(native_process, 'run', side_effect=run):
                yield {'package': package, 'output': output, 'functional_report': functional,
                       'functional_pcm': functional_pcm, 'plan_path': plan_path}, calls

    def test_fixed_six_process_order_one_binary_preselected_failing_m0_and_same_mode_anchors(self):
        def fail_m0_budget(label, report):
            if label == 'M0':
                row = next(row for row in report['profile']['frame_wall']['phases'] if row['name'] == 'script.tick')
                row['counts'][100] = 0; row['counts'][240] = 100
                row.update(p95_lower_ns=2_390_001, p95_upper_ns=2_400_000)
        with self.fixture(fail_m0_budget) as (arguments, calls), \
                patch.object(mac_performance, 'comparison', wraps=mac_performance.comparison) as compare:
            state = subject.measure_sequence(**arguments)
            self.assertEqual(state['status'], 'complete')
            self.assertFalse(state['phase_gate_passed'])
            self.assertEqual([call[0] for call in calls], list(subject.ORDER))
            self.assertEqual(len({call[1][0] for call in calls}), 1)
            for label, argv, options in calls:
                self.assertEqual('--profile-heap-detail' in argv, label.startswith('D'))
                self.assertEqual(options['timeout'], 45)
            m0 = next(attempt for attempt in state['attempts'] if attempt['label'] == 'M0')
            self.assertEqual(m0['script_2ms_p95'], 'fail')
            self.assertEqual([(attempt['label'], attempt['stability_anchor'])
                              for attempt in state['attempts'] if 'stability_anchor' in attempt],
                             [('M1', 'M0'), ('D1', 'D0'), ('D2', 'D0'), ('M2', 'M0')])
            self.assertEqual(compare.call_count, 4)
            for call in compare.call_args_list:
                current, prior = call.args
                self.assertEqual(current['match']['profile_instrumentation'], prior['match']['profile_instrumentation'])
            self.assertEqual(state['descriptive_detailed_minus_maintained']['0'],
                             {'lower': -1_409_999, 'upper': -1_390_001})
            self.assertEqual(m0['source_context_means'][0]['wall_mean_ns'], [1000, 100])

    def test_explicit_mode_refusal_stops_without_retry_and_retains_raw_attempt(self):
        for bad_mode in ('legacy', 'maintained'):
            def mutate(label, report):
                if label == 'D0':
                    if bad_mode == 'legacy':
                        report['profile']['version'] = 2
                        report['profile'].pop('quickjs_heap')
                    else:
                        report['profile']['quickjs_heap'] = current_report(bad_mode)['profile']['quickjs_heap']
            with self.subTest(bad_mode=bad_mode), self.fixture(mutate) as (arguments, calls):
                with self.assertRaisesRegex(ValueError, 'requested mode'):
                    subject.measure_sequence(**arguments)
                self.assertEqual([call[0] for call in calls], ['D0'])
                output = arguments['output']
                state = json.loads((output / 'sequence.json').read_text())
                self.assertEqual(state['status'], 'failed')
                self.assertEqual(state['attempts'][0]['status'], 'failed')
                self.assertTrue((output / 'D0/stdout.json').exists())
                command = json.loads((output / 'D0/command.json').read_text())
                self.assertIn('error', command)
                self.assertFalse((output / 'D0/measurement.json').exists())

    def test_functional_mismatch_stops_at_failed_attempt_and_preserves_prior_samples(self):
        def mutate(label, report):
            if label == 'M1':
                report['state'] = 'different'
        with self.fixture(mutate) as (arguments, calls):
            with self.assertRaisesRegex(ValueError, 'functional output differs'):
                subject.measure_sequence(**arguments)
            self.assertEqual([call[0] for call in calls], ['D0', 'M0', 'M1'])
            state = json.loads((arguments['output'] / 'sequence.json').read_text())
            self.assertEqual([attempt['status'] for attempt in state['attempts']], ['complete', 'complete', 'failed'])
            self.assertNotIn('descriptive_detailed_minus_maintained', state)
            self.assertFalse(state['phase_gate_passed'])

    def test_pcm_and_source_or_heap_coverage_refuse(self):
        def short_sources(label, report):
            report['profile']['scripts'].pop()
        def wrong_calls(label, report):
            report['profile']['scripts'][0]['totals']['calls'] = 99
        def wrong_limit(label, report):
            report['profile']['quickjs_heap']['latest']['limit_bytes'] = 64
        def wrong_samples(label, report):
            report['profile']['quickjs_heap']['samples'] = 99
        for mutate in (short_sources, wrong_calls, wrong_limit, wrong_samples):
            with self.subTest(mutate=mutate.__name__), self.fixture(mutate) as (arguments, calls):
                with self.assertRaises(ValueError):
                    subject.measure_sequence(**arguments)
                self.assertEqual(len(calls), 1)
                state = json.loads((arguments['output'] / 'sequence.json').read_text())
                self.assertEqual(state['attempts'][0]['status'], 'failed')
        with self.fixture() as (arguments, calls):
            arguments['functional_pcm'].write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'PCM binding differs'):
                subject.measure_sequence(**arguments)
            self.assertEqual(calls, [])
        with self.fixture(mutate_pcm=lambda label, pcm: b'y' * len(pcm)) as (arguments, calls):
            with self.assertRaisesRegex(ValueError, 'Timed PCM differs'):
                subject.measure_sequence(**arguments)
            self.assertEqual(len(calls), 1)
            self.assertEqual(json.loads((arguments['output'] / 'sequence.json').read_text())['status'], 'failed')

    def test_deadline_stops_before_next_arm_and_records_incomplete_attempt(self):
        bounded = subject.bounded_sequence
        def delay(label, report):
            time.sleep(0.1)
        with self.fixture(delay) as (arguments, calls), \
                patch.object(subject, 'bounded_sequence', side_effect=lambda: bounded(0.05)):
            with self.assertRaisesRegex(TimeoutError, 'sequence exceeded'):
                subject.measure_sequence(**arguments)
            self.assertEqual([call[0] for call in calls], ['D0'])
            state = json.loads((arguments['output'] / 'sequence.json').read_text())
            self.assertEqual(state['status'], 'failed')
            self.assertEqual(state['attempts'][0]['status'], 'failed')
            self.assertTrue((arguments['output'] / 'D0/stdout.json').exists())

    def test_source_context_refuses_unwind_cpu_loss_duplicates_order_and_noninteger_ids(self):
        with self.fixture() as (arguments, calls):
            subject.measure_sequence(**arguments)
            report = json.loads((arguments['output'] / 'M0/stdout.json').read_text())
        for field in ('unwound', 'unavailable_cpu_samples'):
            bad = copy.deepcopy(report); bad['profile']['scripts'][0]['totals'][field] = 1
            with self.assertRaisesRegex(ValueError, 'Incomplete'):
                subject.observation(bad, 'maintained', 100)
        for kind in ('duplicate', 'reversed', 'boolean'):
            bad = copy.deepcopy(report)
            if kind == 'duplicate':
                bad['profile']['scripts'][1]['source'] = bad['profile']['scripts'][0]['source']
            elif kind == 'reversed':
                bad['profile']['scripts'].reverse()
            else:
                bad['profile']['scripts'][0]['source'][0] = True
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                subject.observation(bad, 'maintained', 100)

    def test_modified_protocol_bytes_refuse_before_measurement(self):
        with self.fixture() as (arguments, calls):
            with arguments['plan_path'].open('a') as stream:
                stream.write('\n')
            with self.assertRaisesRegex(ValueError, 'reviewed bytes'):
                subject.measure_sequence(**arguments)
            self.assertEqual(calls, [])
            self.assertFalse(arguments['output'].exists())


@unittest.skipUnless(os.name == 'posix' and hasattr(signal, 'setitimer'), 'POSIX sequence signals')
class SequenceLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        child = CHILD.replace("print('child ready', flush=True)",
                              "print('child ready', flush=True)\n(root / 'ready').touch()")
        self.command = [sys.executable, '-c', PARENT, child, str(self.root), 'running', GRANDCHILD]

    @contextmanager
    def ready_execution(self, sequence_seconds=None):
        wait = native_process._wait
        def wait_after_ready(tree, readers, command, timeout, deadline, cancel=None):
            startup = time.monotonic() + 5
            while not (self.root / 'ready').exists():
                if cancel is not None:
                    cancel()
                self.assertFalse(tree.exited(), 'Fixture exited before readiness')
                self.assertLess(time.monotonic(), startup, 'Fixture readiness deadline exceeded')
                time.sleep(.005)
            # The marker follows a real grandchild write and flushed stdout.
            # Rearm the real sequence alarm, leaving cleanup/signal handlers intact.
            if sequence_seconds is not None:
                signal.setitimer(signal.ITIMER_REAL, sequence_seconds)
            return wait(tree, readers, command, timeout, time.monotonic() + timeout, cancel)
        with patch.object(native_process, '_wait', wait_after_ready):
            yield

    def assert_stopped(self):
        sentinel = self.root / 'alive'
        self.assertTrue(sentinel.exists(), 'Grandchild must actually have executed')
        before = sentinel.read_bytes()
        self.assertGreater(len(before), 0)
        time.sleep(0.15)
        self.assertEqual(sentinel.read_bytes(), before, 'Descendant still writes after cleanup')
        for pid in json.loads((self.root / 'pids.json').read_text()):
            status = subprocess.run(['/bin/ps', '-o', 'stat=', '-p', str(pid)],
                                    capture_output=True, text=True, timeout=5)
            self.assertTrue(not status.stdout.strip() or status.stdout.lstrip().startswith('Z'),
                            f'Descendant {pid} remains live: {status.stdout!r}')

    def test_per_child_timeout_reaps_actual_descendants(self):
        with subject.bounded_sequence(6) as cancel, self.ready_execution():
            with self.assertRaises(subprocess.TimeoutExpired) as caught:
                native_process.run(self.command, timeout=0.5, cancel=cancel)
        self.assertIn(b'child ready', caught.exception.output)
        self.assert_stopped()

    def test_sequence_deadline_reaps_actual_descendants_and_restores_handlers(self):
        before = {number: signal.getsignal(number) for number in (signal.SIGALRM, signal.SIGTERM, signal.SIGINT)}
        with self.assertRaisesRegex(TimeoutError, 'sequence exceeded'):
            with subject.bounded_sequence(6) as cancel, self.ready_execution(sequence_seconds=0.5):
                native_process.run(self.command, timeout=5, cancel=cancel)
        self.assert_stopped()
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL), (0.0, 0.0))
        self.assertEqual({number: signal.getsignal(number) for number in before}, before)

    def test_missing_readiness_refuses_and_reaps_actual_descendants(self):
        # This child runs the same writer tree but never creates our ready marker.
        self.command[3] = CHILD
        with self.assertRaisesRegex(AssertionError, 'readiness deadline exceeded'):
            with subject.bounded_sequence(6) as cancel, self.ready_execution():
                native_process.run(self.command, timeout=0.5, cancel=cancel)
        self.assertFalse((self.root / 'ready').exists())
        self.assert_stopped()

    def test_readiness_arming_error_reaps_actual_descendants(self):
        arm = signal.setitimer
        def fail_execution_arm(which, seconds, *args):
            if seconds == 0.5:
                raise OSError('test execution alarm could not arm')
            return arm(which, seconds, *args)
        with self.assertRaisesRegex(OSError, 'could not arm'), \
                patch.object(signal, 'setitimer', fail_execution_arm):
            with subject.bounded_sequence(6) as cancel, self.ready_execution(sequence_seconds=0.5):
                native_process.run(self.command, timeout=5, cancel=cancel)
        self.assertEqual(signal.getitimer(signal.ITIMER_REAL), (0.0, 0.0))
        self.assert_stopped()

    def test_term_and_repeat_signals_during_cleanup_reap_actual_descendants(self):
        errors = []
        def terminate_when_started():
            limit = time.monotonic() + 5
            while not (self.root / 'alive').exists() or (self.root / 'alive').stat().st_size == 0:
                if time.monotonic() >= limit:
                    errors.append('Grandchild did not start')
                    return
                time.sleep(0.005)
            os.kill(os.getpid(), signal.SIGTERM)
        sender = threading.Thread(target=terminate_when_started)
        stop = native_process._PosixTree.stop
        cleaned = []
        def repeated_signals(tree):
            # These arrive after managed wait caught the first cancellation.
            # No one may interrupt the actual kill/wait/reader cleanup.
            for number in (signal.SIGTERM, signal.SIGINT, signal.SIGALRM):
                os.kill(os.getpid(), number)
            stop(tree)
            cleaned.append(tree.process.returncode)
        try:
            with self.assertRaisesRegex(InterruptedError, 'SIGTERM'):
                with subject.bounded_sequence(5) as cancel, \
                        patch.object(native_process._PosixTree, 'stop', repeated_signals):
                    sender.start()
                    native_process.run(self.command, timeout=5, cancel=cancel)
        finally:
            sender.join(timeout=6)
        self.assertFalse(sender.is_alive())
        self.assertEqual(errors, [])
        self.assertEqual(len(cleaned), 1)
        self.assertIsNotNone(cleaned[0])
        self.assert_stopped()

    def test_deadline_during_child_timeout_cleanup_cannot_interrupt_reaping(self):
        stop = native_process._PosixTree.stop
        def deadline_during_stop(tree):
            time.sleep(0.25)  # Execution deadline fires during mandatory cleanup.
            stop(tree)
        with self.assertRaisesRegex(TimeoutError, 'sequence exceeded'):
            with subject.bounded_sequence(6) as cancel, self.ready_execution(sequence_seconds=0.65):
                with patch.object(native_process._PosixTree, 'stop', deadline_during_stop):
                    with self.assertRaises(subprocess.TimeoutExpired):
                        native_process.run(self.command, timeout=0.5, cancel=cancel)
                cancel()
        self.assert_stopped()

    def test_cancellation_and_invalid_callback_refuse_before_launch(self):
        with patch.object(native_process.subprocess, 'Popen') as spawn:
            with self.assertRaisesRegex(TypeError, 'callable'):
                native_process.run(self.command, cancel=True)
            def cancelled():
                raise InterruptedError('cancelled before launch')
            with self.assertRaisesRegex(InterruptedError, 'before launch'):
                native_process.run(self.command, cancel=cancelled)
            spawn.assert_not_called()

    def test_term_during_spawn_is_deferred_until_owned_cleanup(self):
        spawn = native_process.subprocess.Popen
        children = []
        def cancel_during_spawn(*args, **kwargs):
            child = spawn(*args, **kwargs)
            children.append(child)
            os.kill(os.getpid(), signal.SIGTERM)
            return child
        with self.assertRaisesRegex(InterruptedError, 'SIGTERM'):
            with subject.bounded_sequence(5) as cancel, \
                    patch.object(native_process.subprocess, 'Popen', side_effect=cancel_during_spawn):
                native_process.run([sys.executable, '-c', 'import time; time.sleep(60)'],
                                   timeout=5, cancel=cancel)
        self.assertEqual(len(children), 1)
        self.assertIsNotNone(children[0].returncode)
        self.assertTrue(children[0].stdout.closed)
        self.assertTrue(children[0].stderr.closed)


if __name__ == '__main__':
    unittest.main()
