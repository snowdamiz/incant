"""Actual harmless tool trees through the public symbol helper; no native build."""
from pathlib import Path
import subprocess
import sys
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import native_process
import shipping_symbols as symbols
import test_native_process as lifecycle


class ShippingCommandLifecycle(unittest.TestCase):
    # Share the established real sentinel-writing descendant fixture, not a
    # mock process or a second process-group cleanup implementation.
    setUp = lifecycle.NativeProcessTests.setUp
    python = lifecycle.NativeProcessTests.python
    tree = lifecycle.NativeProcessTests.tree
    assert_stopped = lifecycle.NativeProcessTests.assert_stopped

    def test_symbol_command_uses_current_supervisor_with_finite_budget(self):
        def cancel():
            pass
        with mock.patch.object(native_process, 'run', wraps=native_process.run) as managed:
            actual = symbols.run(self.python('print("symbols")'), cancel=cancel)
        self.assertEqual(actual, 'symbols\n')
        self.assertEqual(managed.call_count, 1)
        self.assertEqual(managed.call_args.kwargs['timeout'], 120)
        self.assertIs(managed.call_args.kwargs['cancel'], cancel)
        self.assertEqual(managed.call_args.kwargs['stderr'], subprocess.STDOUT)

    def test_utf8_merged_output_and_nonzero_diagnostic_are_preserved(self):
        out = 'stdout ā → 日本語\n'.encode()
        err = 'stderr 🐦\n'.encode()
        script = f'import os; os.write(1, {out!r}); os.write(2, {err!r})'
        with mock.patch('subprocess._text_encoding', return_value='cp1252'):
            self.assertEqual(symbols.run(self.python(script)), (out + err).decode())
        with self.assertRaises(ValueError) as caught:
            symbols.run(self.python(script + '; raise SystemExit(17)'))
        self.assertEqual(str(caught.exception),
                         f'{Path(sys.executable).name} failed: ' + (out + err).decode())
        self.assertEqual(caught.exception.__cause__.returncode, 17)

    def test_timeout_stops_actual_pipe_holding_descendant(self):
        with self.assertRaises(subprocess.TimeoutExpired) as caught:
            symbols.run(self.tree('running'), timeout=3)
        self.assertIn(b'child ready', caught.exception.output)
        self.assert_stopped()

    def test_cancellation_stops_tree_before_returning_to_temporary_owner(self):
        def cancel():
            sentinel = self.root / 'alive'
            if sentinel.exists() and sentinel.stat().st_size > 0:
                raise RuntimeError('supervisor cancelled symbols')
        with self.assertRaisesRegex(RuntimeError, 'supervisor cancelled symbols'):
            symbols.run(self.tree('running'), timeout=10, cancel=cancel)
        self.assert_stopped()

    def test_successful_leader_stops_pipe_holding_descendant(self):
        self.assertEqual(symbols.run(self.tree('exit'), timeout=10), 'child ready\n')
        self.assert_stopped()

    def test_nonzero_leader_stops_descendant_before_error_conversion(self):
        with self.assertRaisesRegex(ValueError, 'child ready') as caught:
            symbols.run(self.tree('failure'), timeout=10)
        self.assertEqual(caught.exception.__cause__.returncode, 23)
        self.assert_stopped()

    def test_invalid_utf8_is_reported_only_after_descendant_cleanup(self):
        with self.assertRaises(UnicodeDecodeError):
            symbols.run(self.tree('bad-output'), timeout=10)
        self.assert_stopped()

    def test_prelaunch_cancellation_and_invalid_deadline_spawn_nothing(self):
        command = self.python('from pathlib import Path; Path("unexpected").touch()')
        def cancel():
            raise RuntimeError('cancel before spawn')
        with mock.patch.object(subprocess, 'Popen', side_effect=AssertionError('spawned')):
            with self.assertRaisesRegex(RuntimeError, 'cancel before spawn'):
                symbols.run(command, cancel=cancel)
            for timeout in (None, 0, -1, float('nan'), float('inf')):
                with self.subTest(timeout=timeout), self.assertRaises(ValueError):
                    symbols.run(command, timeout=timeout)


if __name__ == '__main__':
    unittest.main()
