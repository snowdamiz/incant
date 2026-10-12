"""Trusted developer owner of a fixed-mode retained native CPU worker.

This is engine tooling, never a guest shell capability. The caller chooses the
project and engine cache; the project cannot choose executable paths or flags.
The context owns startup, protocol deadlines and complete descendant cleanup.
"""
from contextlib import contextmanager
import json
from pathlib import Path
import queue
import threading

import native_process
from native_package import require
from play_native import ROOT, prepare

MAX_REQUEST = 256 * 1024
MAX_RESPONSE = 8 * 1024 * 1024


class NativeRefusal(ValueError):
    """The child consumed this request ID but refused native admission."""


class Worker:
    def __init__(self, process, expected, timeout):
        native_process._timeout(timeout)
        self.process = process
        self.timeout = timeout
        self.next_id = 1
        self.stopped = False
        self.clean_stop = False
        self.ready = self._io(None)
        try:
            require(self.ready.get('event') == 'ready' and self.ready.get('version') == 1
                    and self.ready.get('execution') == 'native_cpu'
                    and self.ready.get('physics_mode') == expected['physics_mode']
                    and self.ready.get('physics_variant') == expected['physics_mode']
                    and self.ready.get('project_sha256') == expected['project_sha256']
                    and self.ready.get('state', {}).get('tick') == 0,
                    'Worker startup identity differs from selected project/kernel')
        except BaseException:
            self.process.close()
            raise

    def _io(self, payload):
        result = queue.Queue(maxsize=1)

        def exchange():
            try:
                if payload is not None:
                    self.process.stdin.write(payload)
                    self.process.stdin.flush()
                line = self.process.stdout.readline(MAX_RESPONSE + 2)
                require(bool(line) and line.endswith(b'\n') and len(line) <= MAX_RESPONSE + 1,
                        'Worker response is missing, truncated or oversized')
                value = json.loads(line)
                require(isinstance(value, dict), 'Worker response must be an object')
                result.put((value, None))
            except BaseException as error:
                result.put((None, error))

        thread = threading.Thread(target=exchange, name='native-worker-exchange')
        thread.start()
        try:
            try:
                value, error = result.get(timeout=self.timeout)
            except queue.Empty as error:
                raise TimeoutError('Native worker request deadline exceeded') from error
            if error is not None:
                raise error
            return value
        except BaseException:
            self.process.close()
            self.stopped = True
            raise
        finally:
            # On timeout/interrupt, tree cleanup closes even inherited handles
            # before joining a blocked writer/reader. No detached IO thread.
            thread.join()

    def _request(self, operation, params=None):
        require(not self.stopped, 'Native worker is stopped')
        operation = {'op': operation}
        if params is not None:
            operation['params'] = params
        request = {'version': 1, 'id': self.next_id, 'request': operation}
        payload = (json.dumps(request, allow_nan=False, separators=(',', ':')) + '\n').encode()
        require(len(payload) <= MAX_REQUEST, 'Worker request exceeds 256 KiB')
        response = self._io(payload)
        try:
            require(response.get('version') == 1 and response.get('id') == self.next_id,
                    'Worker response version/sequence mismatch')
            require(('result' in response) != ('error' in response), 'Ambiguous worker response')
            self.next_id += 1
            if 'error' in response:
                error = response['error']
                require(isinstance(error, dict) and error.get('code') == 'native'
                        and isinstance(error.get('message'), str), 'Unexpected worker protocol refusal')
                raise NativeRefusal(error['message'])
            require(isinstance(response['result'], dict), 'Worker result must be an object')
            return response['result']
        except NativeRefusal:
            raise
        except BaseException:
            self.process.close()
            self.stopped = True
            raise

    def step(self, params=None):
        return self._request('step', {} if params is None else params)

    def observe(self, params=None):
        return self._request('observe', {} if params is None else params)

    def stop(self):
        if self.stopped:
            require(self.clean_stop, 'Native worker ended without a successful Stop')
            return
        try:
            require(self._request('stop') == {'playing': False}, 'Worker did not acknowledge Stop')
            self.process.stdin.close()
            require(self.process.wait(timeout=self.timeout) == 0, 'Worker failed during Stop')
            self.clean_stop = True
        finally:
            self.process.close()
            self.stopped = True


@contextmanager
def start(project, target_dir, *, scene=None, camera=None, ui_viewport=None,
          sample_rate=48000, timeout=30, evidence=None):
    """Select/copy a binary, yield its verified live session, then discard it."""
    native_process._timeout(timeout)
    project = Path(project).resolve(strict=True)
    require(isinstance(sample_rate, int) and 8000 <= sample_rate <= 192000, 'Invalid sample rate')
    with prepare(project, target_dir, evidence) as (binary, env, receipt, private):
        command = [str(binary), 'native-worker', str(project),
                   '--project-sha256', receipt['project_sha256'], '--sample-rate', str(sample_rate)]
        for name, value in (('--scene', scene), ('--camera', camera), ('--ui-viewport', ui_viewport)):
            if value is not None:
                command += [name, str(value)]
        receipt['worker_command'] = command
        with (private / 'worker.stderr').open('wb') as stderr:
            with native_process.Interactive(command, cwd=ROOT, env=env, stderr=stderr) as process:
                try:
                    worker = Worker(process, receipt, timeout)
                    receipt['ready'] = worker.ready
                    yield worker
                    worker.stop()
                    receipt['passed'] = True
                finally:
                    process.close()
                    receipt['worker_exit_code'] = process.returncode
