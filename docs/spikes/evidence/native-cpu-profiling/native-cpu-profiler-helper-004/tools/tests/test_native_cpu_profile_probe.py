"""Real small subprocess lifecycle controls; not shipping game execution evidence."""
import json
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROBE = runpy.run_path(str(ROOT / 'tools/probes/native-cpu-profile.py'))
Runner = PROBE['Runner']


class ProfileProbeProcessTests(unittest.TestCase):
    def test_regular_files_do_not_block_large_final_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); runner = Runner(root)
            row = runner.run('large', [sys.executable, '-c',
                "import json,sys; print(json.dumps({'data':'x'*2000000})); sys.stderr.write('e'*1000000)"])
            self.assertEqual(row['exit'], 0)
            self.assertEqual(len(json.loads(row['stdout'])['data']), 2_000_000)
            self.assertEqual(len(row['stderr']), 1_000_000)
            self.assertEqual((root / 'commands/large.stdout').read_text(), row['stdout'])
            with self.assertRaises(ProcessLookupError):
                os.kill(row['pid'], 0)

    def test_timeout_reaps_leaf_and_retains_output_receipt(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); runner = Runner(root)
            with self.assertRaises(subprocess.TimeoutExpired):
                runner.run('timeout', [sys.executable, '-c',
                    "import time; print('before-timeout',flush=True); time.sleep(60)"], timeout=0.3)
            row = json.loads((root / 'commands/timeout.json').read_text())
            self.assertEqual(row['stdout'], 'before-timeout\n')
            self.assertEqual(row['exception'], 'TimeoutExpired')
            self.assertNotEqual(row['exit'], 0)
            with self.assertRaises(ProcessLookupError):
                os.kill(row['pid'], 0)

    def test_capture_failure_reaps_concurrent_leaf(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); runner = Runner(root)
            with self.assertRaisesRegex(ValueError, 'rejected player'):
                with runner.child('capture', [sys.executable, '-c', 'import time; time.sleep(60)']):
                    raise ValueError('rejected player')
            row = json.loads((root / 'commands/capture.json').read_text())
            self.assertEqual(row['exception'], 'ValueError')
            self.assertNotEqual(row['exit'], 0)
            with self.assertRaises(ProcessLookupError):
                os.kill(row['pid'], 0)

    def test_export_invocation_retains_real_early_refusal(self):
        # Real public exporter rejects before any build for an existing output;
        # invocation and failure are retained without spawning an outer wrapper.
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); runner = Runner(root)
            project = root / 'project.json'; project.write_text('{}')
            with self.assertRaisesRegex(ValueError, 'Output already exists'):
                runner.export('summary', project, root)
            row = json.loads((root / 'commands/export-summary.json').read_text())
            self.assertEqual(row['kind'], 'in_process_export')
            self.assertEqual(row['host_pid'], os.getpid())
            self.assertEqual(row['exit'], 1)
            self.assertEqual(row['exception']['type'], 'ValueError')


if __name__ == '__main__':
    unittest.main()
