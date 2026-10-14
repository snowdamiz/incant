"""Real owned-child cleanup and raw protocol-failure retention controls."""
import gzip
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
import stress_rpc


class Rpc(unittest.TestCase):
    def executable(self, root, body):
        path = root / 'tool'
        path.write_text('#!' + sys.executable + '\n' + body + '\n')
        path.chmod(0o755)
        return path

    def absent(self, pid):
        with self.assertRaises(ProcessLookupError): os.kill(pid, 0)

    def test_constructor_pipe_failure_kills_and_reaps_created_child(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = self.executable(root, 'import time; time.sleep(60)')
            with patch.object(stress_rpc.os, 'set_blocking', side_effect=OSError('injected pipe failure')):
                with self.assertRaisesRegex(OSError, 'injected'):
                    stress_rpc.Session(tool, root / 'project', root / 'journal', [], root / 'failure')
            receipt = json.loads((root / 'failure.json').read_text())
            self.assertTrue(receipt['owned_process_reaped']); self.absent(receipt['pid'])
            self.assertIsNotNone(receipt['exit_code'])

    def test_partial_eof_bytes_and_error_receipt_survive_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = self.executable(root, 'import sys\nsys.stdin.readline()\nsys.stdout.write(\'{"id":1\');sys.stdout.flush()')
            session = stress_rpc.Session(tool, root / 'project', root / 'journal', [], root / 'eof')
            with self.assertRaisesRegex(ValueError, 'stopped before reply'): session.call('project.read')
            self.assertEqual(gzip.decompress((root / 'eof.stdout.gz').read_bytes()), b'{"id":1')
            receipt = json.loads((root / 'eof.json').read_text())
            self.assertTrue(receipt['owned_process_reaped']); self.absent(receipt['pid'])
            session.close()  # Idempotent; no signal to a recycled PID.

    def test_successful_exit_and_raw_response_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = self.executable(root, 'import sys,json\nfor line in sys.stdin:\n r=json.loads(line);print(json.dumps({"id":r["id"],"result":{"revision":3}}),flush=True)')
            log = []; session = stress_rpc.Session(tool, root / 'project', root / 'journal', log, root / 'ok')
            self.assertEqual(session.call('project.read'), {'revision': 3})
            session.close()
            receipt = json.loads((root / 'ok.json').read_text())
            self.assertEqual(receipt['exit_code'], 0); self.absent(receipt['pid'])
            self.assertEqual(len(log), 1)
            self.assertEqual(json.loads(gzip.decompress((root / 'ok.stdout.gz').read_bytes()))['result']['revision'], 3)

if __name__ == '__main__': unittest.main()
