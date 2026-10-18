"""Protocol owner failure/deadline tests against actual subprocess pipes."""
from contextlib import contextmanager
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import native_process
from play_worker import MAX_REQUEST, NativeRefusal, Worker

EXPECTED = {'physics_mode': 'fast', 'project_sha256': 'a' * 64}
READY = {'event': 'ready', 'version': 1, 'execution': 'native_cpu', 'physics_mode': 'fast',
         'physics_variant': 'fast', 'project_sha256': 'a' * 64, 'state': {'tick': 0}}
PREFIX = 'import sys,json,time\nprint(' + repr(json.dumps(READY)) + ',flush=True)\n'
SERVER = '''
for line in sys.stdin:
    r=json.loads(line); op=r['request']['op']
    answer={'version':1,'id':r['id']}
    if r['request'].get('params',{}).get('bad'):
        answer['error']={'code':'native','message':'invalid native field'}
    else:
        answer['result']={'playing':False} if op=='stop' else {'tick':r['id']}
    print(json.dumps(answer),flush=True)
    if op=='stop': break
'''


class WorkerOwner(unittest.TestCase):
    @contextmanager
    def child(self, source=PREFIX + SERVER):
        with tempfile.TemporaryFile() as error:
            with native_process.Interactive([sys.executable, '-c', source], stderr=error) as process:
                yield process

    def test_recoverable_native_refusal_consumes_id_and_stop_is_idempotent(self):
        with self.child() as process:
            worker = Worker(process, EXPECTED, 5)
            with self.assertRaisesRegex(NativeRefusal, 'invalid native field'):
                worker.step({'bad': True})
            self.assertEqual(worker.observe(), {'tick': 2})
            worker.stop()
            worker.stop()
            self.assertEqual(process.returncode, 0)
            with self.assertRaisesRegex(ValueError, 'stopped'):
                worker.step()

    def test_wrong_startup_identity_closes_actual_child(self):
        with self.child() as process:
            with self.assertRaisesRegex(ValueError, 'identity'):
                Worker(process, {**EXPECTED, 'physics_mode': 'deterministic'}, 5)
            self.assertIsNotNone(process.returncode)

    def test_bad_response_sequence_and_schema_poison_owner(self):
        for response in ({'version': 1, 'id': 2, 'result': {}},
                         {'version': 1, 'id': 1, 'result': {}, 'error': {}},
                         {'version': 1, 'id': 1, 'error': {'code': 'sequence', 'message': 'wrong'}},
                         {'version': 1, 'id': 1, 'result': []}):
            with self.subTest(response=response), self.child(PREFIX + 'sys.stdin.readline()\nprint(' + repr(json.dumps(response)) + ',flush=True)\ntime.sleep(60)') as process:
                worker = Worker(process, EXPECTED, 5)
                with self.assertRaises(ValueError):
                    worker.step()
                self.assertIsNotNone(process.returncode)
                with self.assertRaisesRegex(ValueError, 'successful Stop'):
                    worker.stop()

    def test_hung_reader_and_writer_have_deadlines_and_no_live_io_thread(self):
        for params in ({}, {'blocked': 'x' * (MAX_REQUEST - 200)}):
            with self.subTest(bytes=len(str(params))), self.child(PREFIX + 'time.sleep(60)') as process:
                worker = Worker(process, EXPECTED, 5)
                worker.timeout = 0.15  # The short bound tests hung requests, not interpreter startup.
                with self.assertRaises(TimeoutError):
                    worker.step(params)
                self.assertIsNotNone(process.returncode)
        import threading
        self.assertFalse(any(t.name == 'native-worker-exchange' for t in threading.enumerate()))

    def test_oversized_response_closes_child_and_local_bad_request_consumes_no_id(self):
        with self.child() as process:
            worker = Worker(process, EXPECTED, 5)
            for params in ({'huge': 'x' * MAX_REQUEST}, {'number': float('nan')}):
                with self.assertRaises(ValueError):
                    worker.step(params)
            self.assertEqual(worker.observe(), {'tick': 1})
            worker.stop()
        with self.child(PREFIX + 'sys.stdin.readline()\nsys.stdout.write("x"*(8*1024*1024+2));sys.stdout.flush()\ntime.sleep(60)') as process:
            worker = Worker(process, EXPECTED, 5)
            with self.assertRaisesRegex(ValueError, 'oversized'):
                worker.step()
            self.assertIsNotNone(process.returncode)

    def test_eof_and_startup_deadline_are_terminal(self):
        for source, failure, timeout in [('pass', ValueError, 5),
                                         ('import time;time.sleep(60)', TimeoutError, 0.2)]:
            with self.subTest(source=source), self.child(source) as process:
                with self.assertRaises(failure):
                    Worker(process, EXPECTED, timeout)
                self.assertIsNotNone(process.returncode)


if __name__ == '__main__':
    unittest.main()
