"""Run trusted engine tools without leaving descendants using private build inputs.

This is lifecycle management, not a sandbox for project-selected executables.
POSIX descendants must retain the owned session/process group and credentials;
Windows descendants inherit a non-breakaway, kill-on-close Job Object. Cleanup
also runs on successful exit, before returning control to temporary-file owners.
Captured output has subprocess-style unbounded storage; send build logs to files.
"""
import ctypes
import math
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import threading
import time


class _Reader:
    def __init__(self, stream):
        self.stream = stream
        self.chunks = []
        self.error = None
        self.thread = threading.Thread(target=self.read, name='native-process-output')

    def read(self):
        try:
            while chunk := self.stream.read(65536):
                self.chunks.append(chunk)
        except BaseException as error:
            self.error = error

    def value(self):
        return b''.join(self.chunks)


def _darwin_group_is_empty(group):
    # Darwin killpg excludes zombies and reports EPERM for a zombie-only group.
    # Confirm no live members instead of swallowing a genuine permission error.
    # https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c
    # https://github.com/apple-oss-distributions/xnu/blob/main/libsyscall/wrappers/libproc/libproc.c
    library = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
    list_pids = library.proc_listpgrppids
    list_pids.argtypes = (ctypes.c_uint32, ctypes.c_void_p, ctypes.c_int)
    list_pids.restype = ctypes.c_int
    capacity = 64
    while capacity <= 1_048_576:
        pids = (ctypes.c_int * capacity)()
        ctypes.set_errno(0)
        count = list_pids(group, pids, ctypes.sizeof(pids))  # Returns PID count.
        # libproc converts its syscall's -1 to zero while retaining errno.
        if count < 0 or (count == 0 and ctypes.get_errno()):
            raise OSError(ctypes.get_errno(), 'Cannot inspect owned process group')
        if count < capacity:
            for pid in pids[:count]:
                if pid > 0:
                    try:
                        if os.getpgid(pid) == group:
                            return False
                    except ProcessLookupError:  # Exited/zombie; cannot execute.
                        pass
            return True
        capacity *= 2
    raise RuntimeError('Owned process group exceeds inspection limit')


class _DarwinExitObserver:
    """Observe the owned child without reaping it on pre-3.13 macOS Python."""
    def __init__(self, pid):
        self.queue = select.kqueue()
        self.already_exited = False
        try:
            event = select.kevent(pid, filter=select.KQ_FILTER_PROC,
                                 flags=select.KQ_EV_ADD | select.KQ_EV_ONESHOT,
                                 fflags=select.KQ_NOTE_EXIT)
            try:
                self.queue.control([event], 0, 0)
            except ProcessLookupError:
                # The unreaped direct child still reserves its PID. Darwin
                # refuses registration if it exited before kevent was added.
                self.already_exited = True
        except BaseException:
            self.queue.close()
            raise

    def exited(self):
        if not self.already_exited:
            self.already_exited = bool(self.queue.control(None, 1, 0))
        return self.already_exited

    def close(self):
        self.queue.close()


class _PosixTree:
    def __init__(self, command, options, interactive=False):
        self.use_waitid = callable(getattr(os, 'waitid', None)) and all(
            hasattr(os, name) for name in ('WNOWAIT', 'WEXITED', 'P_PID'))
        self.observer = None
        if not self.use_waitid and not (sys.platform == 'darwin' and hasattr(select, 'kqueue')):
            raise RuntimeError('Managed tools require non-reaping POSIX child observation')
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE if interactive else subprocess.DEVNULL,
                                        start_new_session=True, **options)
        if not self.use_waitid:
            try:
                self.observer = _DarwinExitObserver(self.process.pid)
            except BaseException:
                try:
                    self.stop()
                finally:
                    for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
                        if stream is not None:
                            stream.close()
                raise

    def release(self):
        pass

    def exited(self):
        # Do not reap: the leader's reserved PID prevents group-ID reuse before
        # killpg. communicate()/poll() would reap it and can also wait forever
        # when a descendant keeps stdout open after the leader has exited.
        if self.observer is not None:
            return self.observer.exited()
        return os.waitid(os.P_PID, self.process.pid,
                         os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None

    def stop(self):
        try:
            try:
                os.killpg(self.process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            except PermissionError:
                if sys.platform != 'darwin' or not _darwin_group_is_empty(self.process.pid):
                    raise
        finally:
            # The direct child remains ours until this wait, including errors.
            try:
                self.process.wait()
            finally:
                if self.observer is not None:
                    self.observer.close()


class _WindowsJob:
    # https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
    # https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information
    def __init__(self):
        from ctypes import wintypes

        class BasicLimits(ctypes.Structure):
            _fields_ = [('ProcessTime', ctypes.c_int64), ('JobTime', ctypes.c_int64),
                        ('Flags', wintypes.DWORD), ('MinWorkingSet', ctypes.c_size_t),
                        ('MaxWorkingSet', ctypes.c_size_t), ('ActiveProcessLimit', wintypes.DWORD),
                        ('Affinity', ctypes.c_size_t), ('PriorityClass', wintypes.DWORD),
                        ('SchedulingClass', wintypes.DWORD)]

        class ExtendedLimits(ctypes.Structure):
            _fields_ = [('Basic', BasicLimits), ('IoCounters', ctypes.c_uint64 * 6),
                        ('ProcessMemory', ctypes.c_size_t), ('JobMemory', ctypes.c_size_t),
                        ('PeakProcessMemory', ctypes.c_size_t), ('PeakJobMemory', ctypes.c_size_t)]

        class Accounting(ctypes.Structure):
            _fields_ = [('Times', ctypes.c_int64 * 4), ('PageFaults', wintypes.DWORD),
                        ('TotalProcesses', wintypes.DWORD), ('ActiveProcesses', wintypes.DWORD),
                        ('TerminatedProcesses', wintypes.DWORD)]

        self.accounting_type = Accounting
        self.api = ctypes.WinDLL('kernel32', use_last_error=True)
        signatures = {
            'CreateJobObjectW': ((ctypes.c_void_p, wintypes.LPCWSTR), wintypes.HANDLE),
            'SetInformationJobObject': ((wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p,
                                         wintypes.DWORD), wintypes.BOOL),
            'QueryInformationJobObject': ((wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p,
                                           wintypes.DWORD, ctypes.c_void_p), wintypes.BOOL),
            'OpenProcess': ((wintypes.DWORD, wintypes.BOOL, wintypes.DWORD), wintypes.HANDLE),
            'AssignProcessToJobObject': ((wintypes.HANDLE, wintypes.HANDLE), wintypes.BOOL),
            'TerminateJobObject': ((wintypes.HANDLE, wintypes.UINT), wintypes.BOOL),
            'CloseHandle': ((wintypes.HANDLE,), wintypes.BOOL),
        }
        for name, (args, result) in signatures.items():
            function = getattr(self.api, name)
            function.argtypes, function.restype = args, result
        self.handle = self.api.CreateJobObjectW(None, None)  # Non-inheritable.
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            limits = ExtendedLimits()
            limits.Basic.Flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.
            self.check(self.api.SetInformationJobObject(self.handle, 9, ctypes.byref(limits),
                                                        ctypes.sizeof(limits)))
        except BaseException:
            self.close()
            raise

    @staticmethod
    def check(result):
        if not result:
            raise ctypes.WinError(ctypes.get_last_error())

    def assign(self, pid):
        process = self.api.OpenProcess(0x0100 | 0x0001, False, pid)  # SET_QUOTA | TERMINATE.
        self.check(process)
        try:
            self.check(self.api.AssignProcessToJobObject(self.handle, process))
        finally:
            self.check(self.api.CloseHandle(process))

    def terminate(self):
        self.check(self.api.TerminateJobObject(self.handle, 1))
        # TerminateJobObject initiates termination. Wait for every member, not
        # only the wrapper, before a caller can delete its private inputs.
        while True:
            accounting = self.accounting_type()
            self.check(self.api.QueryInformationJobObject(self.handle, 1, ctypes.byref(accounting),
                                                          ctypes.sizeof(accounting), None))
            if accounting.ActiveProcesses == 0:
                return
            time.sleep(0.01)

    def close(self):
        if self.handle:
            handle, self.handle = self.handle, None
            self.check(self.api.CloseHandle(handle))


class _WindowsTree:
    def __init__(self, command, options, interactive=False):
        self.job = _WindowsJob()
        self.interactive = interactive
        self.process = None
        try:
            # This fixed, isolated Python wrapper cannot spawn a command until
            # assignment succeeds. No assign-after-command-spawn race, taskkill
            # fallback, inheritable job handle, or breakaway flag is permitted.
            wrapper = [sys.executable, '-I', '-S', str(Path(__file__).resolve()),
                       '--interactive-child' if interactive else '--managed-child', *command]
            self.process = subprocess.Popen(wrapper, stdin=subprocess.PIPE, **options)
            self.job.assign(self.process.pid)
        except BaseException:
            try:
                if self.process is not None:
                    self.process.kill()  # Still gated; it has no descendants.
                    self.process.wait()
                    for stream in (self.process.stdin, self.process.stdout, self.process.stderr):
                        if stream is not None:
                            stream.close()
            finally:
                self.job.close()
            raise

    def release(self):
        self.process.stdin.write(b'GO\n')
        self.process.stdin.flush()
        if not self.interactive:
            self.process.stdin.close()

    def exited(self):
        return self.process.poll() is not None  # Job handle owns descendant lifetime.

    def stop(self):
        try:
            self.job.terminate()
        finally:
            try:
                self.job.close()  # Also kills leaked descendants after success/errors.
            finally:
                self.process.wait()
                if not self.process.stdin.closed:
                    self.process.stdin.close()


def _wait(tree, readers, command, timeout, deadline, cancel=None):
    while True:
        if cancel is not None:
            cancel()
        if tree.exited():
            return
        for reader in readers:
            if reader.error is not None:
                raise reader.error
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(command, timeout)
        time.sleep(min(0.01, remaining))


def _command(command):
    if isinstance(command, (str, bytes, os.PathLike)):
        raise TypeError('command must be an argv sequence, never shell text')
    command = [os.fsdecode(os.fspath(argument)) for argument in command]
    if not command:
        raise ValueError('command must not be empty')
    if os.name == 'nt' and command[0].rstrip(' .').lower().endswith(('.bat', '.cmd')):
        raise ValueError('Windows batch files require a shell and are not managed commands')
    return command


def _timeout(timeout):
    if not isinstance(timeout, (int, float)) or not math.isfinite(timeout) or timeout <= 0:
        raise ValueError('timeout must be finite and positive')


class Interactive:
    """Owned trusted-tool pipes; use as a context manager and drain output.

    The caller supplies bounded protocol IO/deadlines. Never poll/wait the raw
    child: wait() here preserves the POSIX group identity until tree cleanup.
    Context exit, timeout and successful leader exit kill/reap all descendants
    before private inputs may be deleted. close() is idempotent.
    """
    def __init__(self, command, *, cwd=None, env=None, stderr=None):
        self.command = _command(command)
        self.tree = (_WindowsTree if os.name == 'nt' else _PosixTree)(
            self.command, dict(cwd=cwd, env=env, stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE if stderr is None else stderr,
                               text=False), interactive=True)
        self.closed = False
        self.stdin, self.stdout, self.stderr = (
            self.tree.process.stdin, self.tree.process.stdout, self.tree.process.stderr)
        try:
            self.tree.release()
        except BaseException:
            self.close()
            raise

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    @property
    def returncode(self):
        return self.tree.process.returncode

    def wait(self, timeout=30):
        _timeout(timeout)
        if not self.closed:
            try:
                _wait(self.tree, [], self.command, timeout, time.monotonic() + timeout)
            finally:
                self.close()
        return self.tree.process.returncode

    def close(self):
        if self.closed:
            return
        # Mark first: a second cleanup must never signal a reused POSIX group ID.
        self.closed = True
        try:
            self.tree.stop()
        finally:
            for stream in (self.stdin, self.stdout, self.stderr):
                if stream is not None:
                    stream.close()


def run(command, *, cwd=None, env=None, timeout=120, stdout=None, stderr=None,
        text=True, encoding='utf-8', cancel=None):
    """Run argv without a shell; default stdout/stderr are captured, stdin is EOF.

    Return CompletedProcess even for a nonzero command exit. TimeoutExpired
    retains captured bytes, as subprocess.run does. Decoding happens only after
    cleanup, so malformed output cannot leak children. Windows command-launch
    failures are reported by the gated wrapper as a nonzero exit with diagnostic
    stderr. POSIX launch failures raise OSError. Kernel termination/reaping may
    exceed the execution timeout; returning with a live owned tree is forbidden.
    Optional cancel() may raise before launch and during the managed wait. Signal
    handlers should only latch cancellation, never raise during spawn or cleanup.
    """
    if cancel is not None and not callable(cancel):
        raise TypeError('cancel must be callable')
    command = _command(command)
    _timeout(timeout)
    if cancel is not None:
        cancel()
    deadline = time.monotonic() + timeout
    options = dict(cwd=cwd, env=env, stdout=subprocess.PIPE if stdout is None else stdout,
                   stderr=subprocess.PIPE if stderr is None else stderr, text=False)
    tree = (_WindowsTree if os.name == 'nt' else _PosixTree)(command, options)
    readers = []
    failure = None
    try:
        if cancel is not None:
            cancel()
        for stream in (tree.process.stdout, tree.process.stderr):
            if stream is not None:
                reader = _Reader(stream)
                readers.append(reader)
                reader.thread.start()
        tree.release()
        _wait(tree, readers, command, timeout, deadline, cancel)
    except BaseException as error:
        failure = error
    finally:
        try:
            tree.stop()
        finally:
            for reader in readers:
                if reader.thread.ident is not None:
                    reader.thread.join()
            for stream in (tree.process.stdout, tree.process.stderr):
                if stream is not None:
                    stream.close()
    captures = {reader.stream: reader.value() for reader in readers}
    out, err = captures.get(tree.process.stdout), captures.get(tree.process.stderr)
    if isinstance(failure, subprocess.TimeoutExpired):
        failure.output, failure.stderr = out, err
    if failure is not None:
        raise failure
    if cancel is not None:
        cancel()
    for reader in readers:
        if reader.error is not None:
            raise reader.error
    if text:
        # Match subprocess text-mode universal newline handling.
        out = None if out is None else out.decode(encoding).replace('\r\n', '\n').replace('\r', '\n')
        err = None if err is None else err.decode(encoding).replace('\r\n', '\n').replace('\r', '\n')
    return subprocess.CompletedProcess(command, tree.process.returncode, out, err)


def _managed_child(command, interactive=False):
    # Fixed trusted entry point, never a project-selected Python module/script.
    if os.name != 'nt' or not command:
        return 125
    # Do not buffer beyond the assignment gate: subsequent bytes belong to the
    # actual interactive child, even if the parent already wrote its request.
    gate = b''
    while len(gate) < 3:
        chunk = os.read(sys.stdin.fileno(), 3 - len(gate))
        if not chunk:
            return 125
        gate += chunk
    if gate != b'GO\n':
        return 125
    try:
        result = subprocess.run(command, stdin=sys.stdin if interactive else subprocess.DEVNULL,
                                check=False)
    except OSError as error:
        print(f'Managed command could not start: {error}', file=sys.stderr, flush=True)
        return 127
    # Keep Windows DWORD exit codes (including crashes) without truncation.
    api = ctypes.WinDLL('kernel32', use_last_error=True)
    api.ExitProcess.argtypes = (ctypes.c_uint32,)
    api.ExitProcess.restype = None
    api.ExitProcess(result.returncode & 0xffffffff)
    return 125  # Unreachable if the OS contract holds.


if __name__ == '__main__':
    if len(sys.argv) < 3 or sys.argv[1] not in ('--managed-child', '--interactive-child'):
        raise SystemExit('This engine helper is imported, not a public command.')
    raise SystemExit(_managed_child(sys.argv[2:], interactive=sys.argv[1] == '--interactive-child'))
