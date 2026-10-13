#!/usr/bin/env python3
"""CPU-only real child-process tests; never builds or calls a GPU binary.

Only grant/source lookup is replaced for a temporary harness fixture. Both actual
runner wait/kill/reap/ledger paths execute with /usr/bin/true or /bin/sleep.
Revision 2 also takes a temporary stand-in for the global native-gpu.lock, debits
GPU attempts before launch, and routes SIGTERM into the same cleanup as SIGINT.
"""
import contextlib
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import build
import common as c
import gpu

RECORDS = []


def absent(pid):
    for probe in (os.kill, os.killpg):
        try:
            probe(pid, 0)
        except ProcessLookupError:
            continue
        return False
    return True


class Lifecycle(unittest.TestCase):
    def exercise(self, runner, case):
        with tempfile.TemporaryDirectory(prefix="incant-0068-r2-lifecycle-") as directory:
            root = Path(directory)
            paths = {name: root / relative for name, relative in {
                'LOGS': 'logs', 'LOCK': 'lock', 'GLOBAL_LOCK': 'native-gpu.lock',
                'LEDGER': 'ledger.jsonl', 'POST_RELEASE_LEDGER': 'post.jsonl',
                'LATCH': 'released.json', 'SCRATCH': 'tmp', 'CHECKOUT': 'checkout',
            }.items()}
            paths['CHECKOUT'].mkdir()
            paths['SCRATCH'].mkdir()
            program = Path('/usr/bin/true' if case in ('success', 'after_interrupt') else '/bin/sleep')
            args = [] if program.name == 'true' else ['30']
            seconds = 0.1 if case in ('timeout', 'ledger_error') else 5
            timer = None
            original = None
            def interrupt_after(_):
                raise KeyboardInterrupt('after-parser-control')
            appended = []
            real_append = c.durable_append
            def failing_append(path, row):
                # The debit succeeds; the terminal result row fails to persist.
                if row.get('kind') != 'debit':
                    raise OSError('ledger-append-control')
                appended.append(row)
                real_append(path, row)
            stubs = [mock.patch.multiple(c, **paths),
                     mock.patch.object(c, 'grant', return_value=({}, 'cpu-only-grant')),
                     mock.patch.object(c, 'gate', return_value=({}, 'cpu-only-grant')),
                     mock.patch.object(c, 'tools_identity', return_value={}),
                     mock.patch.object(c, 'verify_checkout', return_value={}),
                     mock.patch.object(gpu, 'frozen', return_value=(program, c.sha(program)))]
            created = []
            real_popen = subprocess.Popen
            def recording_popen(*popen_args, **popen_kwargs):
                child = real_popen(*popen_args, **popen_kwargs)
                created.append(child)
                return child
            if case == 'ledger_error':
                stubs.append(mock.patch.object(c, 'durable_append', side_effect=failing_append))
                stubs.append(mock.patch.object(gpu.subprocess, 'Popen', side_effect=recording_popen))
            with contextlib.ExitStack() as stack, contextlib.redirect_stdout(io.StringIO()):
                for stub in stubs:
                    stack.enter_context(stub)
                if case in ('interrupt', 'terminate'):
                    number = signal.SIGINT if case == 'interrupt' else signal.SIGTERM
                    timer = threading.Timer(0.15, lambda: os.kill(os.getpid(), number))
                    timer.start()
                try:
                    if runner == 'build':
                        build.run_group(case, [str(program), *args], root,
                                        {'PATH': c.SYSTEM_PATH}, seconds, 'out', 'err')
                    else:
                        gpu.run_child(case, 'review', case, 'cpu-fixture', args, '.', seconds,
                                      frames=8, submissions=5,
                                      after=interrupt_after if case == 'after_interrupt' else None)
                except (KeyboardInterrupt, c.Interrupted, OSError) as error:
                    original = error
                finally:
                    if timer is not None:
                        timer.cancel()
                        timer.join()
                rows = [json.loads(line) for line in paths['LEDGER'].read_text().splitlines()]
                results = [row for row in rows if row['kind'] != 'debit']
                debits = [row for row in rows if row['kind'] == 'debit']
                self.assertEqual(len(debits), 0 if runner == 'build' else 1)
                if runner == 'gpu':
                    self.assertEqual((debits[0]['frames'], debits[0]['submissions']), (8, 5))
                if case == 'ledger_error':
                    # The result row could not be written, yet the child was
                    # killed and reaped and its debit stays charged.
                    self.assertEqual(results, [])
                    self.assertEqual(str(original), 'ledger-append-control')
                    self.assertEqual(len(created), 1)
                    self.assertTrue(absent(created[0].pid))
                    self.assertEqual(created[0].returncode, -signal.SIGKILL)
                    RECORDS.append({'runner': runner, 'case': case, 'debit': debits[0],
                                    'original_exception_preserved': type(original).__name__,
                                    'child_pid_and_group_absent': True,
                                    'child_returncode': created[0].returncode})
                    return
                self.assertEqual(len(results), 1)
                row = results[0]
                self.assertTrue(absent(row['pid']))
                self.assertTrue(row['group_empty_after'])
                self.assertEqual(row['exit'], 0 if case in ('success', 'after_interrupt') else -signal.SIGKILL)
                self.assertEqual(row['timed_out'], case == 'timeout')
                self.assertEqual(row['wait_error'], {'interrupt': 'KeyboardInterrupt',
                                                     'terminate': 'Interrupted'}.get(case))
                self.assertEqual(original is not None, case in ('interrupt', 'terminate', 'after_interrupt'))
                if case == 'after_interrupt':
                    self.assertEqual(str(original), 'after-parser-control')
                if runner == 'gpu':
                    self.assertEqual((row['frames_reserved'], row['submissions_reserved']), (8, 5))
                    self.assertEqual(len(gpu.attempts('review', case)), 1)
                    self.assertEqual(len(gpu.results('review', case)), 1)
                    # A failed/interrupting attempt still consumes the global
                    # budget before a second child can be launched.
                    before = len(rows)
                    with self.assertRaisesRegex(SystemExit, 'appearance frame budget'):
                        gpu.run_child(case+'-excess', 'review', 'excess', 'cpu-fixture',
                                      args, '.', seconds, frames=24, submissions=5)
                    self.assertEqual(len(gpu.ledger()), before)
                    self.assertFalse((paths['LOGS'] / ('gpu-'+case+'-excess.stdout.txt')).exists())
                RECORDS.append({'runner': runner, 'case': case, 'row': row,
                                'debits': debits,
                                'original_exception_preserved': type(original).__name__ if original else None,
                                'pid_and_group_absent': True})

    def test_build_success(self): self.exercise('build', 'success')
    def test_build_timeout(self): self.exercise('build', 'timeout')
    def test_build_sigint(self): self.exercise('build', 'interrupt')
    def test_build_sigterm(self): self.exercise('build', 'terminate')
    def test_gpu_success(self): self.exercise('gpu', 'success')
    def test_gpu_timeout(self): self.exercise('gpu', 'timeout')
    def test_gpu_sigint(self): self.exercise('gpu', 'interrupt')
    def test_gpu_sigterm(self): self.exercise('gpu', 'terminate')
    def test_gpu_postprocess_interrupt(self): self.exercise('gpu', 'after_interrupt')
    def test_gpu_ledger_append_failure(self): self.exercise('gpu', 'ledger_error')


if __name__ == '__main__':
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Lifecycle)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if len(sys.argv) == 3 and sys.argv[1] == '--receipt':
        with open(sys.argv[2], 'x') as file:
            json.dump({'cpu_only': True, 'passed': result.wasSuccessful(),
                       'tests_run': result.testsRun, 'records': RECORDS}, file, indent=2)
            file.write('\n')
    elif len(sys.argv) != 1:
        raise SystemExit('use no arguments or --receipt NEW_PATH')
    raise SystemExit(0 if result.wasSuccessful() else 1)
