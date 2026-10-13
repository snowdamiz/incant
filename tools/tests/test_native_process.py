"""Real process-tree regressions; no shell, Cargo, renderer or project execution."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import native_process


# All descendants are trusted test programs that retain the inherited group/job.
# The grandchild continuously writes through an unbuffered handle. The parent
# waits for that actual write, so cleanup assertions cannot pass vacuously.
GRANDCHILD = '''
import os, sys, time
limit = time.monotonic() + 60
with open(sys.argv[1], 'ab', buffering=0) as sentinel:
    while time.monotonic() < limit:
        sentinel.write(b'x')
        time.sleep(0.01)
'''
CHILD = '''
import json, os, subprocess, sys, time
from pathlib import Path
root = Path(sys.argv[1])
mode = sys.argv[2]
output = subprocess.DEVNULL if mode == 'no-pipes' else None
grandchild = subprocess.Popen([sys.executable, '-c', sys.argv[3], str(root / 'alive')],
                             stdout=output, stderr=output)
(root / 'pids.json').write_text(json.dumps([os.getpid(), grandchild.pid]))
limit = time.monotonic() + 10
while not (root / 'alive').exists():
    if time.monotonic() > limit:
        raise RuntimeError('Grandchild did not start')
    time.sleep(0.005)
print('child ready', flush=True)
if mode == 'running':
    time.sleep(60)
elif mode == 'bad-output':
    sys.stdout.buffer.write(b'\\xff')
    sys.stdout.buffer.flush()
elif mode == 'failure':
    sys.exit(23)
'''
PARENT = '''
import subprocess, sys
child = subprocess.Popen([sys.executable, '-c', sys.argv[1], *sys.argv[2:]])
sys.exit(child.wait())
'''


class NativeProcessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def python(self, source, *arguments):
        return [sys.executable, '-c', source, *map(str, arguments)]

    def tree(self, mode):
        return self.python(PARENT, CHILD, self.root, mode, GRANDCHILD)

    def assert_stopped(self):
        sentinel = self.root / 'alive'
        self.assertTrue(sentinel.exists(), 'Grandchild must actually have executed')
        before = sentinel.read_bytes()
        self.assertGreater(len(before), 0)
        time.sleep(0.15)
        self.assertEqual(sentinel.read_bytes(), before, 'Descendant is still writing after run returned')
        if os.name == 'nt':
            # A live writer would prevent deleting its open file on Windows.
            sentinel.unlink()
        else:
            # Zombie descendants may await OS reaping, but must not execute.
            for pid in json.loads((self.root / 'pids.json').read_text()):
                status = subprocess.run(['/bin/ps', '-o', 'stat=', '-p', str(pid)],
                                        capture_output=True, text=True, timeout=5)
                self.assertTrue(not status.stdout.strip() or status.stdout.lstrip().startswith('Z'),
                                f'Descendant {pid} remains live: {status.stdout!r}')

    def test_captures_unicode_and_nonzero_without_shell(self):
        literal = 'spaces; $(do-not-run) & < > " \' \\ and emoji 🐦'
        result = native_process.run(self.python(
            'import sys; sys.stdout.reconfigure(encoding="utf-8"); print(sys.argv[1]); print("diagnostic", file=sys.stderr); sys.exit(7)', literal))
        self.assertEqual(result.returncode, 7)
        self.assertEqual(result.stdout, literal + '\n')
        self.assertEqual(result.stderr, 'diagnostic\n')
        self.assertEqual(result.args[-1], literal)

    def test_cwd_env_and_closed_stdin(self):
        environment = dict(os.environ, INCANT_PROCESS_TEST='test-value')
        result = native_process.run(self.python(
            'import json,os,sys; print(json.dumps([os.getcwd(), os.environ["INCANT_PROCESS_TEST"], sys.stdin.read()]))'),
            cwd=self.root, env=environment)
        self.assertEqual(json.loads(result.stdout), [str(self.root.resolve()), 'test-value', ''])

    def test_binary_capture_and_universal_newlines(self):
        command = self.python('import os; os.write(1, b"a\\r\\nb\\rc\\n"); os.write(2,b"\\xff")')
        result = native_process.run(command, text=False)
        self.assertEqual((result.stdout, result.stderr), (b'a\r\nb\rc\n', b'\xff'))
        result = native_process.run(command, encoding='latin-1')
        self.assertEqual((result.stdout, result.stderr), ('a\nb\nc\n', 'ÿ'))

    def test_redirected_files_and_merged_output(self):
        command = self.python('import os; os.write(1,b"out"); os.write(2,b"err")')
        output = self.root / 'output'
        with output.open('wb') as stream:
            result = native_process.run(command, stdout=stream, stderr=subprocess.STDOUT)
        self.assertEqual((result.stdout, result.stderr, result.returncode), (None, None, 0))
        self.assertEqual(output.read_bytes(), b'outerr')
        result = native_process.run(command, stderr=subprocess.STDOUT)
        self.assertEqual((result.stdout, result.stderr), ('outerr', None))
        result = native_process.run(command, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.assertEqual((result.stdout, result.stderr), (None, None))

    def test_large_independent_streams_do_not_deadlock(self):
        result = native_process.run(self.python(
            'import os; os.write(1,b"x"*262144); os.write(2,b"y"*262144)'), timeout=10)
        self.assertEqual(result.stdout, 'x' * 262144)
        self.assertEqual(result.stderr, 'y' * 262144)

    def test_interactive_gate_preserves_first_payload_and_eof(self):
        payload = b'first request: ' + b'x' * 65536 + b'\n'
        with native_process.Interactive(self.python(
            'import sys; data=sys.stdin.buffer.readline(); '
            'sys.stdout.buffer.write(data); sys.stdout.buffer.flush(); '
            'assert sys.stdin.buffer.read()==b""; sys.exit(17)')) as child:
            child.stdin.write(payload)
            child.stdin.flush()
            self.assertEqual(child.stdout.readline(), payload)
            child.stdin.close()
            self.assertEqual(child.wait(timeout=10), 17)
            child.close()
            self.assertEqual(child.wait(timeout=10), 17)
        self.assertTrue(child.stdout.closed)

    def test_interactive_timeout_reaps_actual_pipe_holding_descendant(self):
        with native_process.Interactive(self.tree('running')) as child:
            self.assertEqual(child.stdout.readline(), b'child ready\n')
            with self.assertRaises(subprocess.TimeoutExpired):
                child.wait(timeout=0.05)
        self.assert_stopped()

    def test_interactive_caller_failure_reaps_actual_descendants(self):
        with self.assertRaisesRegex(RuntimeError, 'caller failed'):
            with native_process.Interactive(self.tree('running')) as child:
                self.assertEqual(child.stdout.readline(), b'child ready\n')
                raise RuntimeError('caller failed')
        child.close()
        self.assert_stopped()

    def test_interactive_success_cleans_descendants_before_private_files_return(self):
        with native_process.Interactive(self.tree('exit')) as child:
            self.assertEqual(child.stdout.readline(), b'child ready\n')
            self.assertEqual(child.wait(timeout=10), 0)
        self.assert_stopped()

    def test_timeout_kills_grandchild_before_returning_output(self):
        with self.assertRaises(subprocess.TimeoutExpired) as caught:
            native_process.run(self.tree('running'), timeout=3)
        self.assertIn(b'child ready', caught.exception.output)
        self.assertEqual(caught.exception.stderr, b'')
        self.assert_stopped()

    def test_successful_parent_exit_cleans_pipe_holding_descendant(self):
        # communicate() alone would remain blocked on the living grandchild.
        result = native_process.run(self.tree('exit'), timeout=10)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, 'child ready\n')
        self.assert_stopped()

    def test_nonzero_parent_exit_also_cleans_descendant(self):
        result = native_process.run(self.tree('failure'), timeout=10)
        self.assertEqual(result.returncode, 23)
        self.assert_stopped()

    def test_success_cleans_descendant_without_inherited_capture_pipes(self):
        result = native_process.run(self.tree('no-pipes'), timeout=10)
        self.assertEqual(result.returncode, 0)
        self.assert_stopped()

    def test_decode_failure_occurs_after_descendant_cleanup(self):
        with self.assertRaises(UnicodeDecodeError):
            native_process.run(self.tree('bad-output'), timeout=10)
        self.assert_stopped()

    def test_keyboard_interrupt_during_wait_cleans_actual_tree(self):
        def interrupt(*_):
            limit = time.monotonic() + 10
            while not (self.root / 'alive').exists():
                if time.monotonic() > limit:
                    self.fail('Grandchild did not start')
                time.sleep(0.005)
            raise KeyboardInterrupt('injected caller interruption')
        with mock.patch.object(native_process, '_wait', side_effect=interrupt):
            with self.assertRaises(KeyboardInterrupt):
                native_process.run(self.tree('running'), timeout=15)
        self.assert_stopped()

    def test_reader_setup_failure_cleans_direct_child(self):
        seen = []
        original = native_process.subprocess.Popen
        def spawn(*args, **kwargs):
            process = original(*args, **kwargs)
            seen.append(process)
            return process
        with mock.patch.object(native_process.subprocess, 'Popen', side_effect=spawn):
            with mock.patch.object(native_process.threading.Thread, 'start', side_effect=RuntimeError('thread refusal')):
                with self.assertRaisesRegex(RuntimeError, 'thread refusal'):
                    native_process.run(self.python('import time; time.sleep(60)'))
        self.assertEqual(len(seen), 1)
        self.assertIsNotNone(seen[0].returncode)
        self.assertTrue(seen[0].stdout.closed)
        self.assertTrue(seen[0].stderr.closed)

    def test_invalid_arguments_fail_before_launch(self):
        for command in ('echo command', b'echo', Path('command'), []):
            with self.assertRaises((TypeError, ValueError)):
                native_process.run(command)
        for timeout in (None, 0, -1, float('nan'), float('inf')):
            with self.assertRaises(ValueError):
                native_process.run(self.python('pass'), timeout=timeout)

    def test_missing_executable_is_explicit_and_cleans_wrapper(self):
        missing = self.root / 'missing-executable'
        if os.name == 'nt':
            result = native_process.run([missing])
            self.assertEqual(result.returncode, 127)
            self.assertIn('could not start', result.stderr)
        else:
            with self.assertRaises(FileNotFoundError):
                native_process.run([missing])

    @unittest.skipUnless(os.name == 'nt', 'Windows implicit batch shell boundary')
    def test_batch_file_refused_before_launch(self):
        command = self.root / 'tool.CMD'
        marker = self.root / 'must-not-exist'
        command.write_text(f'@echo unsafe > "{marker}"')
        with self.assertRaisesRegex(ValueError, 'batch files'):
            native_process.run([command])
        self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == 'nt', 'Windows Job Object boundary')
    def test_failed_job_assignment_never_releases_command(self):
        marker = self.root / 'must-not-exist'
        with mock.patch.object(native_process._WindowsJob, 'assign', side_effect=OSError('assignment refused')):
            with self.assertRaisesRegex(OSError, 'assignment refused'):
                native_process.run(self.python('from pathlib import Path; import sys; Path(sys.argv[1]).touch()', marker))
        self.assertFalse(marker.exists())

    @unittest.skipUnless(sys.platform == 'darwin', 'Darwin permission boundary')
    def test_darwin_live_group_permission_failure_is_not_swallowed(self):
        with mock.patch.object(native_process.os, 'killpg', side_effect=PermissionError('denied')):
            with mock.patch.object(native_process, '_darwin_group_is_empty', return_value=False):
                with self.assertRaisesRegex(PermissionError, 'denied'):
                    native_process.run(self.python('pass'))

    @unittest.skipUnless(sys.platform == 'darwin', 'Darwin zombie-only process-group behavior')
    def test_darwin_zombie_only_group_is_not_a_cleanup_permission_failure(self):
        # Several rapid exits exercise waitid before group cleanup and reaping.
        for _ in range(4):
            self.assertEqual(native_process.run(self.python('pass')).returncode, 0)


@unittest.skipUnless(sys.platform == 'darwin', 'Darwin pre-3.13 Python compatibility')
class DarwinWithoutWaitidTests(NativeProcessTests):
    def setUp(self):
        super().setUp()
        patcher = mock.patch.object(native_process.os, 'waitid', None, create=True)
        patcher.start()
        self.addCleanup(patcher.stop)

    def test_exit_observation_does_not_consume_the_child_exit_status(self):
        tree = native_process._PosixTree(self.python('import sys; sys.exit(17)'),
            dict(stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
        try:
            deadline = time.monotonic() + 10
            while not tree.exited():
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.005)
            self.assertTrue(tree.exited(), 'The observed exit stays latched')
            self.assertIsNone(tree.process.returncode, 'Only stop may reap the direct child')
        finally:
            tree.stop()
        self.assertEqual(tree.process.returncode, 17)
        self.assertTrue(tree.observer.queue.closed)

    def test_observer_setup_failure_reaps_child_and_closes_streams(self):
        seen = []
        original = native_process.subprocess.Popen

        def spawn(*args, **kwargs):
            process = original(*args, **kwargs)
            seen.append(process)
            return process

        with mock.patch.object(native_process.subprocess, 'Popen', side_effect=spawn), \
             mock.patch.object(native_process, '_DarwinExitObserver', side_effect=OSError('observer refusal')):
            with self.assertRaisesRegex(OSError, 'observer refusal'):
                native_process.run(self.python('import time; time.sleep(60)'))
        self.assertEqual(len(seen), 1)
        self.assertIsNotNone(seen[0].returncode)
        self.assertTrue(seen[0].stdout.closed)
        self.assertTrue(seen[0].stderr.closed)

    def test_interactive_observer_failure_closes_all_three_pipes(self):
        seen = []
        original = native_process.subprocess.Popen

        def spawn(*args, **kwargs):
            process = original(*args, **kwargs)
            seen.append(process)
            return process

        with mock.patch.object(native_process.subprocess, 'Popen', side_effect=spawn), \
             mock.patch.object(native_process, '_DarwinExitObserver', side_effect=OSError('observer refusal')):
            with self.assertRaisesRegex(OSError, 'observer refusal'):
                native_process.Interactive(self.python('import time; time.sleep(60)'))
        self.assertEqual(len(seen), 1)
        self.assertIsNotNone(seen[0].returncode)
        for stream in (seen[0].stdin, seen[0].stdout, seen[0].stderr):
            self.assertTrue(stream.closed)

    def test_child_exit_before_registration_remains_waitable(self):
        process = subprocess.Popen(self.python('import sys; sys.exit(19)'))
        observer = None
        try:
            deadline = time.monotonic() + 10
            while True:
                state = subprocess.run(['/bin/ps', '-o', 'stat=', '-p', str(process.pid)],
                                       capture_output=True, text=True, timeout=5)
                if state.stdout.lstrip().startswith('Z'):
                    break
                self.assertLess(time.monotonic(), deadline)
                time.sleep(0.005)
            observer = native_process._DarwinExitObserver(process.pid)
            self.assertTrue(observer.exited())
            self.assertIsNone(process.returncode)
            self.assertEqual(process.wait(), 19)
        finally:
            if observer is not None:
                observer.close()
            if process.poll() is None:
                process.kill()
            process.wait()


if __name__ == '__main__':
    unittest.main()
