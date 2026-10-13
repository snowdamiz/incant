"""Owned process-group lifecycle for exactly one child at a time.

Standard library only. `run` starts the child in a new session (its own process
group), records the identity through `on_spawn`, and waits with a watchdog.
Whatever happens afterwards -- normal exit, timeout, KeyboardInterrupt, SIGTERM
or SIGHUP delivered to the runner, or an exception from `on_spawn` -- the
`finally` path terminates the whole group, reaps the leader and waits for the
group to disappear before returning or re-raising. Signals are blocked during
that cleanup so a second interrupt cannot abandon it.

Limitation (explicit): SIGKILL of the runner itself cannot be caught. The child
group then survives; the ledger keeps its reservation and spawned PID/PGID, and
runner.py refuses further children until `reconcile` has killed and verified
the absence of that recorded group.
"""

import datetime
import os
import signal
import subprocess
import time

CLEANUP_SIGNALS = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}


class Terminated(BaseException):
    """SIGTERM or SIGHUP delivered to the runner (converted so `finally` runs)."""


def install_signal_handlers():
    def raise_terminated(signum, _frame):
        raise Terminated(signal.Signals(signum).name)

    signal.signal(signal.SIGTERM, raise_terminated)
    signal.signal(signal.SIGHUP, raise_terminated)


def group_alive(pgid):
    """True while any process (including an unreaped zombie) is in the group."""
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        # A same-numbered group owned by someone else: never ours to signal.
        return True


def group_members(pgid):
    """[(pid, start, command)] for processes in `pgid`, from one `ps` snapshot."""
    listing = subprocess.run(["/bin/ps", "-axo", "pid=,pgid=,lstart=,comm="], capture_output=True,
                             text=True, env={"PATH": "/usr/bin:/bin", "LC_ALL": "C"}).stdout
    members = []
    for line in listing.splitlines():
        parts = line.split(None, 7)
        if len(parts) < 8 or not parts[1].isdigit() or int(parts[1]) != pgid:
            continue
        start = " ".join(parts[2:7])
        members.append((int(parts[0]), start, parts[7]))
    return members


def _started_after(start_text, spawned_epoch):
    try:
        start = datetime.datetime.strptime(start_text, "%a %b %d %H:%M:%S %Y")
    except ValueError:
        return True  # Unparseable: treat as ours only when the group was ours.
    return start.timestamp() >= spawned_epoch - 1.0


def _kill_group(pgid, sig, spawned_epoch, outcome, verify):
    """Signal the group. While our unreaped leader is alive the number is ours.
    After the leader is reaped (`verify`), signal only while every live member
    started after our spawn, excluding an unrelated group that reused it."""
    if verify:
        members = group_members(pgid)
        if members and not all(_started_after(start, spawned_epoch) for _, start, _ in members):
            outcome["foreign_group_skipped"] = [m[0] for m in members]
            return False
    try:
        os.killpg(pgid, sig)
        outcome["signals_sent"].append(signal.Signals(sig).name)
        return True
    except ProcessLookupError:
        return False


def _unblock_in_child():
    signal.pthread_sigmask(signal.SIG_UNBLOCK, CLEANUP_SIGNALS)


def _cleanup(proc, outcome, spawned_epoch, grace_s, absent_wait_s):
    previous = signal.pthread_sigmask(signal.SIG_BLOCK, CLEANUP_SIGNALS)
    try:
        pgid = proc.pid
        if proc.poll() is None:
            _kill_group(pgid, signal.SIGTERM, spawned_epoch, outcome, verify=False)
            end = time.monotonic() + grace_s
            while proc.poll() is None and time.monotonic() < end:
                time.sleep(0.02)
            if proc.poll() is None:
                _kill_group(pgid, signal.SIGKILL, spawned_epoch, outcome, verify=False)
        # Blocking reap of the leader; it is dead or dying after SIGKILL.
        code = proc.wait()
        outcome["exit"] = code if code >= 0 else None
        outcome["signal"] = signal.Signals(-code).name if code < 0 else None
        # Grandchildren keep the group alive after the leader is reaped.
        end = time.monotonic() + absent_wait_s
        while group_alive(pgid) and time.monotonic() < end:
            if not _kill_group(pgid, signal.SIGKILL, spawned_epoch, outcome, verify=True):
                break
            time.sleep(0.05)
        while group_alive(pgid) and time.monotonic() < end:
            time.sleep(0.05)  # launchd reaps reparented members.
        outcome["group_absent"] = not group_alive(pgid)
        if not outcome["group_absent"]:
            outcome["survivors"] = [m[0] for m in group_members(pgid)]
    except BaseException as error:  # never lose the original exception
        outcome["cleanup_error"] = repr(error)
    finally:
        signal.pthread_sigmask(signal.SIG_SETMASK, previous)


def run(argv, *, env, cwd, stdout_path, stderr_path, timeout_s, on_spawn,
        grace_s=2.0, absent_wait_s=10.0, outcome=None):
    """Run one child to completion in its own group; always kill and reap it.

    Fills and returns `outcome` (a caller-owned dict, so it survives a re-raise),
    or re-raises the interrupting exception after the group has been terminated,
    reaped and checked. Output files are created new, never replaced.
    """
    outcome = {} if outcome is None else outcome
    outcome.update({"argv0": str(argv[0]), "pid": None, "pgid": None, "exit": None, "signal": None,
                    "timed_out": False, "interrupted": None, "signals_sent": [], "group_absent": None,
                    "survivors": [], "timeout_s": timeout_s})
    with open(stdout_path, "xb") as out, open(stderr_path, "xb") as err:
        # Block interrupts across the spawn so none can land between Popen and
        # the try below; the child unblocks them before exec.
        previous = signal.pthread_sigmask(signal.SIG_BLOCK, CLEANUP_SIGNALS)
        try:
            spawned_epoch = time.time()
            proc = subprocess.Popen(argv, env=env, cwd=cwd, stdin=subprocess.DEVNULL, stdout=out,
                                    stderr=err, start_new_session=True, close_fds=True,
                                    preexec_fn=_unblock_in_child)
        except BaseException:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            raise
        outcome["pid"] = outcome["pgid"] = proc.pid
        outcome["started"] = datetime.datetime.fromtimestamp(spawned_epoch, datetime.UTC).isoformat()
        pending = None
        try:
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
            on_spawn(proc.pid, proc.pid)
            try:
                proc.wait(timeout=timeout_s)
            except subprocess.TimeoutExpired:
                outcome["timed_out"] = True
        except BaseException as error:
            outcome["interrupted"] = f"{type(error).__name__}: {error}"
            pending = error
        finally:
            _cleanup(proc, outcome, spawned_epoch, grace_s, absent_wait_s)
            outcome["ended"] = datetime.datetime.now(datetime.UTC).isoformat()
        if pending is not None:
            raise pending
    return outcome
