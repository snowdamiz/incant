"""Owned process-group lifecycle for exactly one child at a time (macOS).

Standard library only (ctypes for /usr/lib/libproc.dylib; other platforms are
refused). `run` starts the child in a new session behind a spawn gate: the
leader first runs a tiny trampoline (this interpreter, -I -S -B) that blocks on
a pipe. While it is alive and blocked, the runner reads its exact identity --
pid plus kernel start time to the microsecond, both unchanged by exec -- and
records it durably through `on_spawn`. Only then does the runner send the gate
message (b"g" + the exact constructed environment as JSON) and close the pipe;
the trampoline execs the real argv with exactly that environment. If the
runner dies or fails before that, the pipe closes and the trampoline exits 125
without executing anything.

The leader is never reaped before its group is verified empty: its exit is
observed with waitid(WNOWAIT), so its PID -- and therefore the PGID -- stays
reserved for this group whenever the group is signalled. Whatever happens
(normal exit, timeout, KeyboardInterrupt, SIGTERM or SIGHUP to the runner, an
`on_spawn` or `on_member` failure) the `finally` path sends SIGTERM, waits a
grace period, sends SIGKILL, waits until no member other than the unreaped
leader remains, records any survivors' exact identities, and only then reaps
the leader. Signals stay blocked during cleanup. Nothing is ever signalled
after the leader has been reaped. Members seen while the group runs are
reported through `on_member` with their exact identities.

Limitation (explicit): SIGKILL of the runner cannot be caught. The child then
keeps running with its recorded identity and keeps any inherited descriptor
(`inherit_fds`, e.g. the shared native GPU lock) until it exits or `reconcile`
stops it after proving that identity.
"""

import ctypes
import datetime
import json
import os
import signal
import subprocess
import sys
import time

CLEANUP_SIGNALS = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}
GATE_CLOSED_EXIT = 125
# Spawn gate trampoline. argv: <gate fd> <program> <args...>. It reads until
# EOF; anything but b"g" + a JSON environment exits 125 without exec.
GATE = (
    "import json, os, sys\n"
    "fd = int(sys.argv[1])\n"
    "data = b''\n"
    "while True:\n"
    "    chunk = os.read(fd, 65536)\n"
    "    if not chunk:\n"
    "        break\n"
    "    data += chunk\n"
    "os.close(fd)\n"
    "if data[:1] != b'g':\n"
    f"    os._exit({GATE_CLOSED_EXIT})\n"
    "try:\n"
    "    env = json.loads(data[1:])\n"
    "except ValueError:\n"
    f"    os._exit({GATE_CLOSED_EXIT})\n"
    "os.execve(sys.argv[2], sys.argv[2:], env)\n"
)


class Terminated(BaseException):
    """SIGTERM or SIGHUP delivered to the runner (converted so `finally` runs)."""


def install_signal_handlers():
    def raise_terminated(signum, _frame):
        raise Terminated(signal.Signals(signum).name)

    signal.signal(signal.SIGTERM, raise_terminated)
    signal.signal(signal.SIGHUP, raise_terminated)


# ------------------------------------------------------------------ exact identity (libproc)

PROC_PIDTBSDINFO = 3
SZOMB = 5


class _BsdInfo(ctypes.Structure):
    """struct proc_bsdinfo from <sys/proc_info.h> (136 bytes)."""
    _fields_ = [("pbi_flags", ctypes.c_uint32), ("pbi_status", ctypes.c_uint32),
                ("pbi_xstatus", ctypes.c_uint32), ("pbi_pid", ctypes.c_uint32), ("pbi_ppid", ctypes.c_uint32),
                ("pbi_uid", ctypes.c_uint32), ("pbi_gid", ctypes.c_uint32), ("pbi_ruid", ctypes.c_uint32),
                ("pbi_rgid", ctypes.c_uint32), ("pbi_svuid", ctypes.c_uint32), ("pbi_svgid", ctypes.c_uint32),
                ("rfu_1", ctypes.c_uint32), ("pbi_comm", ctypes.c_char * 16), ("pbi_name", ctypes.c_char * 32),
                ("pbi_nfiles", ctypes.c_uint32), ("pbi_pgid", ctypes.c_uint32), ("pbi_pjobc", ctypes.c_uint32),
                ("e_tdev", ctypes.c_uint32), ("e_tpgid", ctypes.c_uint32), ("pbi_nice", ctypes.c_int32),
                ("pbi_start_tvsec", ctypes.c_uint64), ("pbi_start_tvusec", ctypes.c_uint64)]


_LIBPROC = None


def _libproc():
    global _LIBPROC
    if _LIBPROC is None:
        if sys.platform != "darwin":
            raise OSError("exact process identity needs macOS libproc; this platform is refused")
        lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
        lib.proc_pidinfo.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_uint64, ctypes.c_void_p, ctypes.c_int]
        lib.proc_pidinfo.restype = ctypes.c_int
        lib.proc_listpgrppids.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_int]
        lib.proc_listpgrppids.restype = ctypes.c_int
        lib.proc_listallpids.argtypes = [ctypes.c_void_p, ctypes.c_int]
        lib.proc_listallpids.restype = ctypes.c_int
        lib.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
        lib.proc_pidpath.restype = ctypes.c_int
        if ctypes.sizeof(_BsdInfo) != 136:
            raise OSError("unexpected proc_bsdinfo layout; refusing")
        _LIBPROC = lib
    return _LIBPROC


def identity(pid):
    """Exact identity of a live process, or None when it is absent or a zombie.

    {pid, start: [seconds, microseconds], uid, pgid, ppid, comm}. The kernel
    start time is unchanged by exec, so the spawn-gate trampoline and the
    program it becomes share one identity.
    """
    info = _BsdInfo()
    size = ctypes.sizeof(info)
    if _libproc().proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, ctypes.byref(info), size) != size:
        return None
    if info.pbi_pid != pid or info.pbi_status == SZOMB:
        return None
    return {"pid": pid, "start": [info.pbi_start_tvsec, info.pbi_start_tvusec], "uid": info.pbi_uid,
            "pgid": info.pbi_pgid, "ppid": info.pbi_ppid, "comm": info.pbi_comm.decode(errors="replace")}


def same_process(current, retained):
    """Ownership needs an exact match: pid, kernel start time and uid."""
    return (isinstance(current, dict) and isinstance(retained, dict)
            and current.get("pid") == retained.get("pid")
            and list(current.get("start") or []) == list(retained.get("start") or [None])
            and current.get("uid") == retained.get("uid"))


def _pid_list(call, *args):
    size = 256
    while True:
        buffer = (ctypes.c_int * size)()
        count = call(*args, buffer, ctypes.sizeof(buffer))
        if count < 0:
            raise OSError(ctypes.get_errno(), "libproc pid listing failed")
        if count < size:
            return sorted(p for p in buffer[:count] if p > 0)
        size *= 4


def group_pids(pgid):
    """Every process in the group, unreaped zombies included."""
    return _pid_list(_libproc().proc_listpgrppids, pgid)


def all_pids():
    return _pid_list(_libproc().proc_listallpids)


def pid_path(pid):
    buffer = ctypes.create_string_buffer(4096)
    length = _libproc().proc_pidpath(pid, buffer, ctypes.sizeof(buffer))
    return buffer.value.decode(errors="replace") if length > 0 else None


def still_alive(retained):
    """Retained identities whose exact process is still running."""
    return [r for r in retained if same_process(identity(r["pid"]), r)]


# ------------------------------------------------------------------ owned child

def _exited(pid):
    """True once our child has exited. It stays unreaped (waitid WNOWAIT)."""
    return os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None


def _wait(predicate, seconds, poll_s=0.02):
    end = time.monotonic() + seconds
    while not predicate():
        if time.monotonic() >= end:
            return False
        time.sleep(poll_s)
    return True


def _signal_group(pgid, sig, outcome):
    """Only called while our leader is unreaped, so the group number is ours."""
    try:
        os.killpg(pgid, sig)
    except ProcessLookupError:
        return False
    outcome["signals_sent"].append(signal.Signals(sig).name)
    return True


def _observe(pgid, seen, on_member, outcome):
    for pid in group_pids(pgid):
        ident = identity(pid)
        if ident is None:
            continue
        key = (ident["pid"], tuple(ident["start"]))
        if key in seen:
            continue
        seen.add(key)
        outcome["members_seen"].append(ident)
        if on_member is not None:
            on_member(ident)


def _write_all(fd, data):
    view = memoryview(data)
    while view:
        view = view[os.write(fd, view):]


def _unblock_in_child():
    signal.pthread_sigmask(signal.SIG_UNBLOCK, CLEANUP_SIGNALS)


def _cleanup(proc, outcome, seen, on_member, grace_s, absent_wait_s):
    previous = signal.pthread_sigmask(signal.SIG_BLOCK, CLEANUP_SIGNALS)
    try:
        pid = pgid = proc.pid

        def others():
            return [p for p in group_pids(pgid) if p != pid]

        def observe():
            try:
                _observe(pgid, seen, on_member, outcome)
            except BaseException as error:  # recording must never abandon the kill
                outcome.setdefault("observe_errors", []).append(repr(error))

        observe()
        if not _exited(pid):
            _signal_group(pgid, signal.SIGTERM, outcome)
            if not _wait(lambda: _exited(pid), grace_s):
                observe()
                _signal_group(pgid, signal.SIGKILL, outcome)
                _wait(lambda: _exited(pid), absent_wait_s)
        if others():
            observe()
            _signal_group(pgid, signal.SIGKILL, outcome)
            _wait(lambda: not others(), absent_wait_s)
        # Survivors are identified while the unreaped leader still reserves the
        # number; after the reap nothing in this group is attributed to us.
        leftover = others()
        outcome["survivors"] = [identity(p) or {"pid": p, "identity": None} for p in leftover]
        if _exited(pid):
            code = proc.wait()
            outcome["exit"] = code if code >= 0 else None
            outcome["signal"] = signal.Signals(-code).name if code < 0 else None
            outcome["reaped"] = True
        else:
            outcome["leader_unreaped"] = True  # still alive after SIGKILL
        outcome["group_absent"] = not leftover and outcome["reaped"]
    except BaseException as error:  # never lose the original exception
        outcome["cleanup_error"] = repr(error)
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, previous)


def run(argv, *, env, cwd, stdout_path, stderr_path, timeout_s, on_spawn, on_member=None, inherit_fds=(),
        grace_s=2.0, absent_wait_s=10.0, poll_s=0.02, observe_every_s=0.5, outcome=None):
    """Run one child to completion in its own group; always kill and reap it.

    `on_spawn(pid, pgid, identity)` must record the identity durably; the gate
    opens only after it returns. Fills and returns `outcome` (a caller-owned
    dict, so it survives a re-raise), or re-raises the interrupting exception
    after the group has been terminated, verified and reaped. Output files are
    created new, never replaced.
    """
    outcome = {} if outcome is None else outcome
    outcome.update({"argv0": str(argv[0]), "pid": None, "pgid": None, "leader": None, "gate_opened": False,
                    "exit": None, "signal": None, "timed_out": False, "interrupted": None, "signals_sent": [],
                    "members_seen": [], "group_absent": None, "survivors": [], "reaped": False,
                    "leader_unreaped": False, "timeout_s": timeout_s})
    message = b"g" + json.dumps(env, sort_keys=True).encode()
    with open(stdout_path, "xb") as out, open(stderr_path, "xb") as err:
        gate_read, gate_write = os.pipe()
        # Interrupts stay blocked across the spawn so none can land between
        # Popen and the try below; the child unblocks them before exec.
        previous = signal.pthread_sigmask(signal.SIG_BLOCK, CLEANUP_SIGNALS)
        try:
            spawned_epoch = time.time()
            proc = subprocess.Popen([sys.executable, "-I", "-S", "-B", "-c", GATE, str(gate_read),
                                     *(str(a) for a in argv)],
                                    env=env, cwd=cwd, stdin=subprocess.DEVNULL, stdout=out, stderr=err,
                                    start_new_session=True, close_fds=True,
                                    pass_fds=(gate_read, *inherit_fds), preexec_fn=_unblock_in_child)
        except BaseException:
            os.close(gate_write)
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            raise
        finally:
            os.close(gate_read)
        outcome["pid"] = outcome["pgid"] = proc.pid
        outcome["started"] = datetime.datetime.fromtimestamp(spawned_epoch, datetime.UTC).isoformat()
        seen, pending = set(), None
        try:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            # The trampoline is alive and blocked on the gate, so this identity
            # is exact and its number cannot be reused yet.
            leader = outcome["leader"] = identity(proc.pid)
            if leader is not None:
                seen.add((leader["pid"], tuple(leader["start"])))
            on_spawn(proc.pid, proc.pid, leader)
            if leader is None:
                raise OSError("leader identity unavailable before the gate opened; nothing was executed")
            _write_all(gate_write, message)
            os.close(gate_write)
            gate_write = None
            outcome["gate_opened"] = True
            deadline = time.monotonic() + timeout_s
            next_observe = 0.0
            while not _exited(proc.pid):
                now = time.monotonic()
                if now >= deadline:
                    outcome["timed_out"] = True
                    break
                if now >= next_observe:
                    _observe(proc.pid, seen, on_member, outcome)
                    next_observe = now + observe_every_s
                time.sleep(poll_s)
        except BaseException as error:
            outcome["interrupted"] = f"{type(error).__name__}: {error}"
            pending = error
        finally:
            if gate_write is not None:
                os.close(gate_write)  # a gate that never opened: the trampoline exits 125
            _cleanup(proc, outcome, seen, on_member, grace_s, absent_wait_s)
            outcome["ended"] = datetime.datetime.now(datetime.UTC).isoformat()
        if pending is not None:
            raise pending
    return outcome
