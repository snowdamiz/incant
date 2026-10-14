"""Bounded public authoring RPC, with raw receipts and owned child cleanup.

This helper is for trusted local Mac/POSIX tools, not project-selected binaries.
"""
import gzip
import hashlib
import json
import os
from pathlib import Path
import select
import sys
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'tools'))
import native_process

LIMIT = 32 * 1024 * 1024

class Session:
    def __init__(self, binary, project, journal, log, evidence, timeout=60):
        if os.name != 'posix':
            raise ValueError('Core stress authoring requires POSIX bounded pipe IO')
        self.log, self.evidence, self.timeout = log, evidence, timeout
        self.command = [str(binary), 'rpc', str(project), '--journal', str(journal)]
        self.error = evidence.with_suffix('.stderr').open('wb')
        self.raw = gzip.open(evidence.with_suffix('.rpc.jsonl.gz'), 'wb')
        self.stdout = gzip.open(evidence.with_suffix('.stdout.gz'), 'wb')
        self.process = None
        self.count = 0; self.failure = None; self.closed = False
        try:
            self.process = native_process.Interactive(self.command, stderr=self.error)
            self.pid = self.process.tree.process.pid
            for stream in (self.process.stdin, self.process.stdout):
                os.set_blocking(stream.fileno(), False)
        except BaseException as error:
            try:
                if self.process is not None: self.process.close()
            finally:
                self.raw.close(); self.stdout.close(); self.error.close()
                evidence.with_suffix('.json').write_text(json.dumps({
                    'argv': self.command, 'failure': repr(error),
                    'pid': getattr(self, 'pid', None),
                    'exit_code': self.process.returncode if self.process is not None else None,
                    'owned_process_reaped': self.process is not None,
                }, indent=2) + '\n')
            raise

    def call(self, method, params=None):
        try:
            self.count += 1
            request = {'id': self.count, 'method': method, 'params': params or {}}
            payload = (json.dumps(request, separators=(',', ':'), allow_nan=False) + '\n').encode()
            if len(payload) > LIMIT:
                raise ValueError('RPC request exceeds bound')
            self.raw.write(b'{"request":' + payload.rstrip() + b'}\n')
            deadline = time.monotonic() + self.timeout
            sent = 0; reply = bytearray()
            while not reply.endswith(b'\n'):
                remaining = deadline - time.monotonic()
                if remaining <= 0: raise TimeoutError('RPC operation deadline exceeded')
                reads, writes, _ = select.select([self.process.stdout],
                    [self.process.stdin] if sent < len(payload) else [], [], remaining)
                if writes:
                    sent += os.write(self.process.stdin.fileno(), payload[sent:sent + 65536])
                if reads:
                    block = os.read(self.process.stdout.fileno(), 65536)
                    if not block: raise ValueError('RPC stopped before reply')
                    self.stdout.write(block); self.stdout.flush()
                    reply.extend(block)
                    if len(reply) > LIMIT: raise ValueError('RPC response exceeds bound')
            self.raw.write(b'{"response":' + reply.rstrip() + b'}\n'); self.raw.flush()
            response = json.loads(reply)
            if response.get('id') != self.count or 'error' in response:
                raise ValueError(f'{method}: {response}')
            result = response['result']
            self.log.append({'method': method, 'request_sha256': hashlib.sha256(payload).hexdigest(),
                             'response_sha256': hashlib.sha256(reply).hexdigest(),
                             'revision': result.get('revision') if isinstance(result, dict) else None})
            return result
        except BaseException as error:
            self.failure = repr(error); self.close(); raise

    def execute(self, description, commands):
        before = self.call('project.read')
        result = self.call('command.execute', {'expected_revision': before['revision'],
            'description': description, 'commands': commands})
        if result['revision'] != before['revision'] + 1:
            raise ValueError('Authoring transaction did not advance exactly one revision')
        return result['transaction_id']

    def close(self):
        if self.closed: return
        self.closed = True
        try:
            if self.failure is None:
                self.process.stdin.close()
                if self.process.wait(timeout=self.timeout) != 0:
                    raise ValueError('RPC closed with a nonzero status')
        except BaseException as error:
            self.failure = repr(error); raise
        finally:
            try: self.process.close()
            finally:
                self.raw.close(); self.stdout.close(); self.error.close()
                self.evidence.with_suffix('.json').write_text(json.dumps({
                    'argv': self.command, 'pid': self.pid, 'exit_code': self.process.returncode,
                    'calls': self.count, 'failure': self.failure, 'owned_process_reaped': True,
                }, indent=2) + '\n')
