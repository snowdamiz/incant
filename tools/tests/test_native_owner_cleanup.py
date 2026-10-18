"""The observer waits for actual absence without signalling a retired group ID."""
from pathlib import Path
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from probes.native_wasm_restart import owner


class OwnerCleanupTests(unittest.TestCase):
    def test_transient_descendant_remains_until_later_observation(self):
        with mock.patch.object(owner, 'process_rows', side_effect=[
            [(23, 17, 0)], [(23, 17, 0)], [(88, 88, 0)],
        ]) as rows, mock.patch.object(owner.time, 'sleep') as sleep, \
                mock.patch.object(owner.os, 'kill') as kill, \
                mock.patch.object(owner.os, 'killpg', create=True) as killpg:
            owner.confirm_group_absent(17)
        self.assertEqual(rows.call_count, 3)
        self.assertEqual(sleep.call_count, 2)
        kill.assert_not_called()
        killpg.assert_not_called()

    def test_persistent_or_reused_identity_remains_a_failure(self):
        for row in [(23, 17, 0), (17, 88, 0)]:
            with self.subTest(row=row), \
                    mock.patch.object(owner, 'process_rows', return_value=[row]), \
                    mock.patch.object(owner.time, 'monotonic', side_effect=[0, 0, 5]), \
                    mock.patch.object(owner.time, 'sleep') as sleep:
                with self.assertRaisesRegex(ValueError, 'Owned PID/group remains'):
                    owner.confirm_group_absent(17)
                sleep.assert_called_once_with(0.01)

    def test_empty_group_needs_no_delay_and_inspection_failure_refuses(self):
        with mock.patch.object(owner, 'process_rows', return_value=[]), \
                mock.patch.object(owner.time, 'sleep') as sleep:
            owner.confirm_group_absent(17)
            sleep.assert_not_called()
        with mock.patch.object(owner, 'process_rows', side_effect=OSError('inspection failed')):
            with self.assertRaisesRegex(OSError, 'inspection failed'):
                owner.confirm_group_absent(17)

    @unittest.skipUnless(os.name == 'posix', 'POSIX observer wraps trusted process groups')
    def test_real_cancelled_tree_retains_failure_and_confirms_absence(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / 'target'
            target.mkdir()
            marker = root / 'child-started'
            child = ('import pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text("started"); '
                     'time.sleep(60)')
            parent = ('import subprocess,sys; '
                      'p=subprocess.Popen([sys.executable,"-c",sys.argv[1],sys.argv[2]]); p.wait()')
            managed = owner.Owner(root / 'observation', target, 1024**2)

            def cancel():
                if marker.exists():
                    raise ValueError('deliberate resource refusal')

            with managed:
                with self.assertRaisesRegex(ValueError, 'deliberate resource refusal'):
                    managed.run([sys.executable, '-c', parent, child, str(marker)],
                                timeout=10, cancel=cancel)
            self.assertEqual(marker.read_text(), 'started')
            self.assertEqual(len(managed.groups), 1)
            self.assertTrue(managed.groups[0]['cleanup_confirmed'])
            self.assertTrue(managed.calls[0]['cleanup_confirmed'])
            self.assertIn('deliberate resource refusal', managed.calls[0]['error'])
            self.assertFalse(managed.live_groups)
            pid = managed.groups[0]['pid']
            self.assertFalse(any(pid in (p, group) for p, group, _ in owner.process_rows()))


if __name__ == '__main__':
    unittest.main()
