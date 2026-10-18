"""Bounded owner for trusted local tools; no project-selected executables.

Reuse native_process's non-reaping group owner. This observer assumes trusted
Cargo/Clang children retain their owned group; sampled limits are not a sandbox.
"""
import contextlib
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import time

import native_process
from .verify import digest, require

GIB = 1024**3
MIB = 1024**2


def tree_bytes(root):
    """Allocated bytes, including directories, without following links; like du -sk."""
    root = Path(root)
    before = root.stat()
    seen = set()
    total = 0

    def onerror(error):
        path = Path(error.filename) if error.filename is not None else None
        if isinstance(error, FileNotFoundError) and path is not None and path != root and path.is_relative_to(root):
            return  # Cargo can retire a temporary descendant during observation.
        raise error

    for directory, dirs, files in os.walk(root, onerror=onerror, followlinks=False):
        paths = [Path(directory), *(Path(directory) / n for n in files)]
        paths += [Path(directory) / n for n in dirs if (Path(directory) / n).is_symlink()]
        for path in paths:
            try:
                info = path.lstat()
            except FileNotFoundError:
                if path == root:
                    raise
                continue
            identity = info.st_dev, info.st_ino
            if identity not in seen:
                total += info.st_blocks * 512
                seen.add(identity)
    after = root.stat()
    require((before.st_dev, before.st_ino) == (after.st_dev, after.st_ino), 'Storage root replaced')
    return total


def process_rows():
    result = subprocess.run(['/bin/ps', '-axo', 'pid=,pgid=,rss='], capture_output=True,
                            text=True, check=True, timeout=5)
    return [tuple(map(int, line.split())) for line in result.stdout.splitlines()]


def confirm_group_absent(pid, timeout=5):
    """Observe asynchronous SIGKILL/reaping completion; never signal a reaped ID.

    The process owner has already killed the reserved group and reaped its
    direct child. Descendants can still briefly appear in ps, including zombies.
    Require their complete absence, retaining an explicit refusal if the OS has
    not retired them. The polling allowance excludes the final process snapshot,
    which has its own five-second timeout. A reused ID refuses without signalling.
    """
    deadline = time.monotonic() + timeout
    while any(pid in (p, group) for p, group, _ in process_rows()):
        require(time.monotonic() < deadline, 'Owned PID/group remains')
        time.sleep(0.01)


class Owner:
    def __init__(self, output, target, target_ceiling):
        self.output, self.target = Path(output), Path(target)
        require(not self.output.exists() and not self.output.is_symlink(), 'Create-only output')
        self.output.mkdir(mode=0o700)
        self.scratch = self.output / 'private-builds'
        self.scratch.mkdir(mode=0o700)
        self.start = time.monotonic()
        self.deadline = self.start + 4200
        self.target_ceiling = target_ceiling
        self.cancelled = False
        self.calls, self.groups, self.live_groups = [], [], set()
        self.last_storage = self.last_sample = 0.
        self.peaks = {'target': 0, 'scratch': 0, 'output': 0, 'rss': 0}
        self.original_run = native_process.run
        self.original_tree = native_process._PosixTree
        self.active = None
        self.passed = False
        self.error = None

    def save(self):
        value = {'passed': self.passed, 'error': self.error, 'seconds': time.monotonic() - self.start,
                 'wrapper_pid': os.getpid(), 'target': str(self.target),
                 'target_ceiling_bytes': self.target_ceiling, 'calls': self.calls,
                 'owned': self.groups, 'peaks': self.peaks}
        temporary = self.output / 'owner.tmp'
        temporary.write_text(json.dumps(value, indent=2) + '\n')
        temporary.replace(self.output / 'owner.json')

    def check(self, force=False):
        require(not self.cancelled and time.monotonic() < self.deadline, 'Cancelled or aggregate deadline')
        now = time.monotonic()
        if not force and now - self.last_sample < 1:
            return
        self.last_sample = now
        require(shutil.disk_usage(self.output).free >= 6 * GIB, 'Free-space floor')
        rss = sum(kib * 1024 for pid, group, kib in process_rows()
                  if pid == os.getpid() or group in self.live_groups)
        self.peaks['rss'] = max(self.peaks['rss'], rss)
        require(rss <= 8 * GIB, 'Owned aggregate RSS')
        logs = [p.stat().st_size for p in self.output.glob('*.log')]
        require(all(n <= 128 * MIB for n in logs) and sum(logs) <= 256 * MIB, 'Retained log bound')
        if force or now - self.last_storage >= 5:
            self.last_storage = now
            target, scratch = tree_bytes(self.target), tree_bytes(self.scratch)
            # Reserve 32 MiB within the two-GiB retained budget for the frozen
            # packet beside this output. Actual packet admission checks that cap.
            output = tree_bytes(self.output) - scratch + 32 * MIB
            for name, value in [('target', target), ('scratch', scratch), ('output', output)]:
                self.peaks[name] = max(self.peaks[name], value)
            require(target <= self.target_ceiling and scratch <= 8 * GIB and output <= 2 * GIB,
                    'Target, private scratch or retained output limit')
            # Build logs initially belong to the exporter's temporary staging.
            all_logs = [p.stat().st_size for p in self.output.rglob('*')
                        if p.is_file() and p.suffix in ('.log', '.jsonl')]
            require(all(n <= 128 * MIB for n in all_logs)
                    and sum(all_logs) <= 256 * MIB, 'Combined private/retained log bound')
        self.save()

    def __enter__(self):
        owner = self

        class Observed(owner.original_tree):
            def __init__(self, *args, **kwargs):
                self.receipt, self.stopped = None, False
                self.argv = list(map(str, args[0]))
                # The base may call our stop() if exit-observer setup fails.
                super().__init__(*args, **kwargs)
                try:
                    self.register()
                    owner.save()
                except BaseException:
                    self.stop()
                    raise

            def register(self):
                if self.receipt is not None:
                    return
                self.receipt = {'pid': self.process.pid, 'argv': self.argv,
                                'cleanup_confirmed': False}
                owner.groups.append(self.receipt)
                owner.live_groups.add(self.process.pid)
                if owner.active is not None:
                    owner.active['pid'] = self.process.pid

            def stop(self):
                if self.stopped:
                    return  # Never signal a group after the leader was reaped.
                self.stopped = True
                self.register()
                try:
                    super().stop()
                    self.receipt['exit'] = self.process.returncode
                    confirm_group_absent(self.process.pid)
                    self.receipt['cleanup_confirmed'] = True
                except BaseException as error:
                    self.receipt['cleanup_error'] = repr(error)
                    raise
                finally:
                    if self.receipt['cleanup_confirmed']:
                        owner.live_groups.discard(self.process.pid)
                    owner.save()

        native_process._PosixTree = Observed
        native_process.run = self.run
        self.handlers = {s: signal.signal(s, lambda *_: setattr(self, 'cancelled', True))
                         for s in (signal.SIGINT, signal.SIGTERM)}
        return self

    def __exit__(self, kind, error, traceback):
        native_process.run = self.original_run
        native_process._PosixTree = self.original_tree
        for sig, handler in self.handlers.items():
            signal.signal(sig, handler)
        self.error = repr(error) if error is not None else None
        self.save()

    def run(self, argv, **kwargs):
        self.check()
        started = time.monotonic()
        row = {'argv': list(map(str, argv)), 'started': time.time(), 'finished': False}
        self.calls.append(row)
        self.active = row
        index = len(self.calls) - 1
        paths = {s: self.output / f'{index:03}.{s}.log' for s in ('stdout', 'stderr')}
        supplied = {s: kwargs.pop(s, None) for s in paths}
        text, encoding = kwargs.pop('text', True), kwargs.pop('encoding', 'utf-8')
        cancel = kwargs.pop('cancel', None)
        timeout = min(kwargs.pop('timeout', 120), max(0.001, self.deadline - time.monotonic()))
        first = len(self.groups)
        output = {}

        def guard():
            self.check()
            if cancel is not None:
                cancel()

        try:
            with contextlib.ExitStack() as stack:
                streams = {s: supplied[s] if supplied[s] is not None else
                           stack.enter_context(paths[s].open('xb')) for s in paths}
                try:
                    result = self.original_run(row['argv'], stdout=streams['stdout'], stderr=streams['stderr'],
                                               timeout=timeout, cancel=guard, **kwargs)
                    row['exit'] = result.returncode
                except BaseException as error:
                    row['error'] = repr(error)
                    raise
                finally:
                    # Preserve even failed exporter diagnostics before its TemporaryDirectory unwinds.
                    for stream in paths:
                        handle = streams[stream]
                        if handle == subprocess.STDOUT:
                            output[stream] = None
                            continue
                        handle.flush()
                        original = Path(handle.name)
                        with original.open('rb') as log:
                            data = log.read(128 * MIB + 1)
                        require(len(data) <= 128 * MIB, 'Child output limit')
                        if original != paths[stream]:
                            paths[stream].write_bytes(data)
                        output[stream] = data.decode(encoding) if text else data
                        row[stream] = {'path': paths[stream].name, 'sha256': digest(paths[stream])}
            self.check(force=True)
            return subprocess.CompletedProcess(row['argv'], result.returncode,
                output['stdout'] if supplied['stdout'] is None else None,
                output['stderr'] if supplied['stderr'] is None else None)
        finally:
            row['finished'] = True
            row['seconds'] = time.monotonic() - started
            row['cleanup_confirmed'] = len(self.groups) > first and all(
                g['cleanup_confirmed'] for g in self.groups[first:])
            self.active = None
            self.save()


class Rpc:
    """One bounded request at a time through the existing interactive process owner."""
    def __init__(self, owner, argv, cwd, env):
        self.owner = owner
        self.stderr = (owner.output / f'rpc-{len(owner.groups)}.stderr.log').open('xb')
        self.child = None
        try:
            self.child = native_process.Interactive(argv, cwd=cwd, env=env, stderr=self.stderr)
            os.set_blocking(self.child.stdin.fileno(), False)
            os.set_blocking(self.child.stdout.fileno(), False)
        except BaseException:
            self.close()
            raise
        self.started = time.monotonic()
        self.buffer = bytearray()
        self.requests = 0
        self.history = []

    def finish(self):
        """EOF then wait without reaping the group leader before owned cleanup."""
        self.child.stdin.close()
        deadline = min(time.monotonic() + 30, self.started + 120)
        while not self.child.tree.exited():
            self.owner.check()
            require(time.monotonic() < deadline, 'RPC shutdown deadline')
            time.sleep(.05)
        self.child.close()
        require(self.child.returncode == 0, 'RPC failed at EOF')

    def close(self):
        try:
            if self.child is not None:
                self.child.close()
        finally:
            self.stderr.close()
            if hasattr(self, 'history'):
                (self.owner.output / (Path(self.stderr.name).stem + '.json')).write_text(
                    json.dumps({'requests': self.history, 'exit': self.child.returncode}, indent=2) + '\n')

    def call(self, method, params=None):
        self.requests += 1
        request = {'id': self.requests, 'method': method, 'params': params or {}}
        data = json.dumps(request).encode() + b'\n'
        require(len(data) <= 16 * 1024, 'Bounded fixed RPC request')
        deadline = min(time.monotonic() + 30, self.started + 120)
        while data or b'\n' not in self.buffer:
            self.owner.check()
            require(time.monotonic() < deadline, 'RPC request/lifetime deadline')
            ready, writable, _ = select.select([self.child.stdout], [self.child.stdin] if data else [], [], .1)
            if writable:
                count = os.write(self.child.stdin.fileno(), data)
                data = data[count:]
            if ready:
                value = os.read(self.child.stdout.fileno(), min(65536, 2 * MIB + 1 - len(self.buffer)))
                require(value, 'RPC EOF before reply')
                self.buffer.extend(value)
                require(len(self.buffer) <= 2 * MIB, 'RPC reply bound')
        line, _, remainder = self.buffer.partition(b'\n')
        self.buffer = bytearray(remainder)
        result = json.loads(line)
        self.history.append({'request': request, 'response': result})
        require(result.get('id') == self.requests and 'error' not in result and 'result' in result,
                'RPC response identity or refusal')
        return result['result']
