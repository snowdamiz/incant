#!/usr/bin/env python3
"""0069 guard, ledger, frame debit and bounded child runner. DRAFT for root review.

  lease.py status                   read-only: grant presence/hash, lease, release, target users
  lease.py acquire                  record this packet's lease on root's grant (create-only)
  lease.py freeze [--new]           check the current freeze generation, or start the next one
  lease.py cargo SUB ARGS...        ./tools/cargo SUB --locked -j2 ARGS (--profile ci --offline)
  lease.py run --label L --timeout S -- ARGV...   bounded CPU child in its own process group
  lease.py release                  confirm no owned child/target user remains, write the latch

Every child runs under `supervise`, which owns its whole lifetime. The child starts
in a new session and process group with an empty signal mask while this process
blocks termination signals. On every outcome (normal exit, nonzero status, timeout,
SIGINT, SIGTERM, SIGHUP, SystemExit or any other exception) a `finally` kills
whatever remains of the group (SIGTERM, then SIGKILL), waits for the child and
confirms the group is gone, with those signals blocked so the cleanup cannot be
interrupted; the original exception then propagates. SIGTERM and SIGHUP become
SystemExit(128 + signal) while a child runs, so they reach that `finally` too.

Locks, always in this order and released in reverse, both without waiting:
1. GPU and adapter children (capture.py, regression.py) first take the global
   NATIVE_GPU_LOCK, shared with every other native GPU user. A busy lock refuses
   at once; root schedules GPU ownership explicitly. CPU children (Cargo, the
   generic `run`, the CPU startup check) do not take it.
2. Every child then takes this packet's group lock; a busy one refuses too.
Both are held across the frame debit, the guarded launch, the kill and reap of the
group and the ledger accounting of its exit, so exactly one managed process group
exists at a time and concurrent invocations cannot lose a debit. Frames are
debited, atomically and before the child starts, from one of two ceilings and are
never refunded: 24 `appearance` frames (capture.py) and 128 tiny `regression`
frames (regression.py: the existing UI GPU tests plus this packet's newly added
halo coverage, at counts derived from their source).

`release` holds the group lock from its process check until the latch is durable:
it rechecks every recorded child group and target user under the lock, appends the
ledger's final row, hashes the finished ledger into release.json (create-only,
fsynced), then creates `released` (create-only, fsynced) and syncs the directory.
Every writer refuses through `guard` once `released` exists, so the ledger never
changes after its hash is taken.

Children get a constructed minimal environment (`child_env`), never the inherited
one. The ledger records only explicit overrides in ALLOWED_ENV. GPU/adapter
children are bounded to 120 s and other non-Cargo children to 300 s.

A freeze generation records HEAD, the bytes of every uncommitted source path (from
unstripped NUL-delimited porcelain status, with renames and deletions explicit),
this packet's tools, Cargo.lock, the child tool chain and the host. Builds and
captures bind to the current generation; a reviewed source change after review
frames needs an explicit new generation, and nothing built under an earlier one is
accepted afterwards.

Refuses (exit 3, nothing started or written) after `released`, without the lease, or
when root's review-ready.json is missing, does not grant this packet and target, or
differs from the hash recorded at acquisition. This tool never edits or deletes
review-ready.json and never waits for it.
"""
import contextlib
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tempfile
import time

PACKET = '0069-core-result-presentation'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
EVIDENCE = Path('/Users/andreyyurlov/.codex/incant-evidence/0069')
GRANT = EVIDENCE / 'review-ready.json'
LEASE = EVIDENCE / 'lease.json'
RELEASED = EVIDENCE / 'released'
RELEASE = EVIDENCE / 'release.json'
LEDGER = EVIDENCE / 'ledger.jsonl'
FRAMES = EVIDENCE / 'frames.json'
GROUP_LOCK = EVIDENCE / 'group.lock'
LOGS = EVIDENCE / 'logs'
SCRATCH = EVIDENCE / 'scratch'
FREEZES = EVIDENCE / 'freeze'
# Shared by every native GPU user on this host; only ever flocked, never written.
NATIVE_GPU_LOCK = Path('/Users/andreyyurlov/.codex/incant-evidence/native-gpu.lock')
# Root reserved this idle target for 0069's CPU-only grant; the grant must name
# it. 0072 owns incant-core-presentation-target.
TARGET = Path('/tmp/incant-agent-font-target')
BRIEF = ROOT / 'handoffs' / PACKET / 'brief.md'
# Constructed child PATH: rustup proxies, the host Node the TypeScript cook
# requires (resolved fnm install, not a per-shell link), Homebrew Python 3.14,
# then the system. Frozen with every generation.
RUSTUP_BIN = '/opt/homebrew/opt/rustup/bin'
NODE_BIN = '/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin'
CHILD_PATH = (RUSTUP_BIN, NODE_BIN, '/opt/homebrew/bin', '/usr/bin', '/bin')
# New appearance captures; UI GPU regression tests (tiny frames): the existing
# ones plus this packet's newly added halo coverage.
CEILINGS = {'appearance': 24, 'regression': 128}
CARGO_TIMEOUT = 3600
REGRESSION_TIMEOUT = 300
GPU_TIMEOUT = 120
ALLOWED_ENV = ('CARGO_TARGET_DIR', 'INCANT_NATIVE_PROJECT', 'INCANT_UI_EVIDENCE', 'INCANT_HDR_EVIDENCE')
TERMINATION = {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}
GRACE = 2.0


class Refused(Exception):
    pass


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def child_env(scratch, overrides=None):
    """A constructed, minimal child environment; nothing is inherited.

    HOME/CARGO_HOME/RUSTUP_HOME let rustup and Cargo find the installed pinned
    toolchain and the offline registry; PYTHONUTF8 fixes text encoding for the
    sample's Python tools. `overrides` may only add ALLOWED_ENV keys.
    """
    home = Path.home()
    env = {'PATH': ':'.join(CHILD_PATH), 'HOME': str(home), 'CARGO_HOME': str(home / '.cargo'),
           'RUSTUP_HOME': str(home / '.rustup'), 'PYTHONUTF8': '1'}
    if scratch is not None:
        env['TMPDIR'] = str(scratch)
    extra = dict(overrides or {})
    if set(extra) - set(ALLOWED_ENV):
        raise Refused('child overrides outside the documented allowlist')
    env.update(extra)
    return env


def recorded_env(overrides):
    """Only the documented, nonsecret overrides this tool sets explicitly."""
    return {key: value for key, value in (overrides or {}).items() if key in ALLOWED_ENV}


def git_value(root, *args):
    """One single-line git value (a hash); never used for status rows."""
    return subprocess.run(['git', *args], cwd=root, capture_output=True, text=True, check=True,
                          env=child_env(None)).stdout.strip()


def status_entries(root):
    """Every uncommitted path from unstripped `git status --porcelain=v1 -z`.

    Rows are `XY PATH\\0`, with a rename or copy followed by its source path as
    one more NUL-terminated field. Returns {path: {status, from, sha256}}, the
    hash being None for a deleted path.
    """
    raw = subprocess.run(['git', 'status', '--porcelain=v1', '-z', '--untracked-files=all'], cwd=root,
                         capture_output=True, check=True, env=child_env(None)).stdout
    fields = raw.split(b'\0')
    if fields and fields[-1] == b'':
        fields.pop()
    entries = {}
    index = 0
    while index < len(fields):
        row = fields[index].decode('utf-8')
        index += 1
        if len(row) < 4 or row[2] != ' ':
            raise Refused(f'unexpected git status row {row!r}')
        code, path = row[:2], row[3:]
        source = None
        if 'R' in code or 'C' in code:
            if index >= len(fields):
                raise Refused(f'git status rename without its source: {row!r}')
            source = fields[index].decode('utf-8')
            index += 1
        file = Path(root) / path
        entries[path] = {'status': code, 'from': source,
                         'sha256': digest(file) if file.is_file() else None}
    return entries


def source_identity(root=ROOT):
    """HEAD and tree plus every uncommitted source path and its bytes.

    This packet's own handoff directory (root's brief and reviews, the result,
    retained evidence copies) is excluded; its tools are frozen separately.
    """
    outputs = f'handoffs/{PACKET}/'
    changed = {path: entry for path, entry in status_entries(root).items()
               if not path.startswith(outputs) and not (entry['from'] or '').startswith(outputs)}
    return {'head': git_value(root, 'rev-parse', 'HEAD'), 'tree': git_value(root, 'rev-parse', 'HEAD^{tree}'),
            'uncommitted': dict(sorted(changed.items()))}


def ledger(row):
    """Append one row and make it durable before returning."""
    with LEDGER.open('a', encoding='utf-8') as stream:
        stream.write(json.dumps({'at': now(), **row}) + '\n')
        stream.flush()
        os.fsync(stream.fileno())


def write_atomic(path, value):
    """Replace `path` with JSON in one rename, after the bytes reach the disk."""
    handle, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    try:
        with os.fdopen(handle, 'w', encoding='utf-8') as stream:
            stream.write(json.dumps(value, indent=1) + '\n')
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def write_new(path, text):
    """Create `path` with `text` only if it does not exist, fully written and
    synced before it appears (a hard link of a synced temporary never replaces)."""
    handle, temporary = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    try:
        with os.fdopen(handle, 'w', encoding='utf-8') as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            raise Refused(f'{path.name} already exists; not replaced') from None
    finally:
        Path(temporary).unlink(missing_ok=True)


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


@contextlib.contextmanager
def exclusive(path, *, busy):
    """Hold an exclusive flock on `path` for the block; never waits for it, and
    refuses with `busy` when another holder has it."""
    lock = Path(path).open('a')
    try:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise Refused(busy) from None
        try:
            yield
        finally:
            fcntl.flock(lock, fcntl.LOCK_UN)
    finally:
        lock.close()


def grant():
    if not GRANT.is_file():
        raise Refused('no review-ready.json from root; not granted')
    value = json.loads(GRANT.read_text(encoding='utf-8'))
    if value.get('packet') != PACKET or value.get('target') != str(TARGET):
        raise Refused('review-ready.json does not grant this packet and target')
    return value


def guard():
    """Every child and every ledger writer calls this before it starts."""
    if RELEASED.exists():
        raise Refused('0069 lease was released; refusing')
    if not LEASE.is_file():
        raise Refused('no 0069 lease; run acquire first')
    lease = json.loads(LEASE.read_text(encoding='utf-8'))
    grant()
    if digest(GRANT) != lease['grant_sha256'] or lease.get('packet') != PACKET:
        raise Refused('review-ready.json changed since acquisition; stop and ask root')
    return lease


def group_alive(group):
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def terminate_group(child, grace=GRACE):
    """Kill whatever remains of the child's process group, reap the child, and
    report whether the group is gone. SIGTERM first, SIGKILL after `grace`."""
    group = child.pid  # a new session's leader: its pid is the group id
    for sig in (signal.SIGTERM, signal.SIGKILL):
        child.poll()
        if not group_alive(group):
            break
        try:
            os.killpg(group, sig)
        except ProcessLookupError:
            break
        deadline = time.monotonic() + grace
        while time.monotonic() < deadline:
            child.poll()  # reap the direct child as soon as it exits
            if not group_alive(group):
                break
            time.sleep(0.02)
    child.wait()
    deadline = time.monotonic() + grace
    while group_alive(group) and time.monotonic() < deadline:
        time.sleep(0.02)  # killed grandchildren are reaped by their new parent
    return not group_alive(group)


def clear_signal_mask():
    """In the forked child before exec: this tool blocks termination signals while
    it spawns, and a blocked mask would otherwise survive exec into the child."""
    signal.pthread_sigmask(signal.SIG_SETMASK, ())


def exit_on_signal(signum, frame):
    raise SystemExit(128 + signum)


def supervise(argv, *, timeout, env, cwd, stdout, stderr, record, started=None):
    """Run one child to completion or timeout in its own session and group.

    `record` is filled as the run proceeds (pid, status, timed_out, interrupted,
    group_reaped, seconds) so a caller can log it even when this re-raises.
    Whatever the outcome, the group is killed and the child waited for before
    this returns or re-raises. Termination signals are blocked in this process
    while the child is spawned and while it is cleaned up (the child itself starts
    with an empty mask), so neither step can be interrupted half way; while the
    child runs, SIGTERM and SIGHUP raise SystemExit like SIGINT raises
    KeyboardInterrupt. Previous handlers are restored afterwards.
    """
    if env is None:
        raise Refused('children need a constructed environment')
    record.update(pid=None, status=None, timed_out=False, interrupted=None, group_reaped=False, seconds=None)
    begun = time.monotonic()
    previous = signal.pthread_sigmask(signal.SIG_BLOCK, TERMINATION)
    handlers = {sig: signal.signal(sig, exit_on_signal) for sig in (signal.SIGTERM, signal.SIGHUP)}
    child = None
    try:
        child = subprocess.Popen([str(a) for a in argv], cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                 stdout=stdout, stderr=stderr, start_new_session=True,
                                 preexec_fn=clear_signal_mask)
        record['pid'] = child.pid
        if started is not None:
            started(child.pid)
        signal.pthread_sigmask(signal.SIG_SETMASK, previous)  # interruptible while waiting
        try:
            child.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            record['timed_out'] = True
    except BaseException as error:
        record['interrupted'] = type(error).__name__
        raise
    finally:
        signal.pthread_sigmask(signal.SIG_BLOCK, TERMINATION)
        try:
            if child is not None:
                record['group_reaped'] = terminate_group(child)
                record['status'] = child.returncode
            else:
                record['group_reaped'] = True
            record['seconds'] = round(time.monotonic() - begun, 3)
        finally:
            for sig, handler in handlers.items():
                signal.signal(sig, signal.SIG_DFL if handler is None else handler)
            signal.pthread_sigmask(signal.SIG_SETMASK, previous)
    return record


def target_users():
    """Processes (other than this tool) whose command line names the granted target."""
    rows = subprocess.run(['ps', '-axo', 'pid=,command='], capture_output=True, text=True, check=True,
                          env=child_env(None)).stdout
    users = []
    for line in rows.splitlines():
        pid, _, command = line.strip().partition(' ')
        if str(TARGET) in command and int(pid) != os.getpid() and 'lease.py' not in command:
            users.append({'pid': int(pid), 'command': command[:300]})
    return users


def reviewed_identities(value):
    """The grant's reviewed identities, checked once at acquisition: `sources` lists
    {path (relative to the worktree), sha256} and `predecessor_release` is one
    {path (absolute), sha256}, the shape of root's review receipts. Later changes
    (the planned font regeneration) are bound by freeze generations instead."""
    for row in value.get('sources', []):
        path = ROOT / row['path']
        if not path.is_file() or digest(path) != row['sha256']:
            raise Refused(f'{row["path"]} differs from its reviewed identity in review-ready.json')
    predecessor = value.get('predecessor_release')
    if predecessor is not None:
        path = Path(predecessor['path'])
        if not path.is_file() or digest(path) != predecessor['sha256']:
            raise Refused('the predecessor release differs from review-ready.json')


def acquire():
    if RELEASED.exists():
        raise Refused('0069 lease was released; refusing')
    value = grant()
    reviewed_identities(value)
    if LEASE.exists():
        raise Refused('lease already recorded')
    users = target_users()
    if users:
        raise Refused(f'target in use: {users}')
    lease = {'packet': PACKET, 'holder': 'claude (Opus 5.5, max thinking) for handoff 0069',
             'grant': value, 'grant_sha256': digest(GRANT), 'target_dir': str(TARGET),
             'brief_sha256': digest(BRIEF), 'worktree': str(ROOT), 'source': source_identity(),
             'acquired_at': now(), 'processes_on_target_at_acquire': users,
             'rules': ['./tools/cargo only', '--locked', '--offline', '--profile ci', 'at most -j2',
                       f'GPU and adapter children first take {NATIVE_GPU_LOCK}, then the packet group '
                       'lock, neither waiting (a busy lock refuses); both held across debit, launch, '
                       'kill/reap and accounting',
                       'one managed process group at a time',
                       'every child in its own session; its group killed and reaped on every outcome',
                       'constructed minimal child environment; nothing inherited',
                       'builds and captures bound to one freeze generation',
                       'no native window, desktop capture or device audio',
                       f'<= {CEILINGS["appearance"]} appearance frames, each <= 960x540, and <= '
                       f'{CEILINGS["regression"]} tiny frames of UI GPU regression tests (existing ones plus '
                       'newly added halo coverage) at source-derived counts, each case once; debited before '
                       'drawing, never refunded',
                       f'GPU/adapter children <= {GPU_TIMEOUT} s, other non-Cargo children <= {REGRESSION_TIMEOUT} s',
                       'release holds the packet lock until its latch is durable; the ledger is final '
                       'before release.json hashes it',
                       'evidence and binaries only under ' + str(EVIDENCE),
                       'ledger records only explicit overrides in ' + ', '.join(ALLOWED_ENV)]}
    with LEASE.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(lease, indent=1) + '\n')
    ledger({'event': 'acquire', 'grant_sha256': lease['grant_sha256'], 'head': lease['source']['head']})
    return lease


def identity():
    """Everything a build or capture depends on, as it is now."""
    node = Path(NODE_BIN) / 'node'
    version = lambda flag: subprocess.run(['sw_vers', flag], capture_output=True, text=True, check=True,
                                          env=child_env(None)).stdout.strip()
    return {'source': source_identity(), 'tools': {p.name: digest(p) for p in sorted(HERE.iterdir()) if p.is_file()},
            'cargo_lock_sha256': digest(ROOT / 'Cargo.lock'),
            'os': {'system': platform.system(), 'release': platform.release(), 'machine': platform.machine(),
                   'product_version': version('-productVersion'), 'build_version': version('-buildVersion')},
            'child_path': list(CHILD_PATH), 'node_sha256': digest(node), 'python': sys.version.split()[0]}


def generations():
    return sorted(FREEZES.glob('generation-*.json'), key=lambda p: int(p.stem.split('-')[1]))


def freeze(new=False):
    """Check the current state against the latest freeze generation and return
    {'generation', 'sha256', 'identity'}; `new` records the next generation from
    the current state (the first call always records generation 1)."""
    guard()
    current = identity()
    existing = generations()
    if existing and not new:
        latest = existing[-1]
        recorded = json.loads(latest.read_text(encoding='utf-8'))
        if recorded['identity'] != current:
            raise Refused('source, tools, lockfile, child tools or host changed after freeze '
                          f'generation {recorded["generation"]}')
        return {'generation': recorded['generation'], 'sha256': digest(latest), 'identity': current}
    number = len(existing) + 1
    FREEZES.mkdir(parents=True, exist_ok=True)
    path = FREEZES / f'generation-{number}.json'
    value = {'generation': number, 'frozen_at': now(), 'identity': current,
             'previous_sha256': digest(existing[-1]) if existing else None}
    with path.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(value, indent=1) + '\n')
    ledger({'event': 'freeze', 'generation': number, 'freeze_sha256': digest(path)})
    return {'generation': number, 'sha256': digest(path), 'identity': current}


def frames_state():
    if FRAMES.exists():
        return json.loads(FRAMES.read_text(encoding='utf-8'))
    return {'ceilings': dict(CEILINGS), 'debited': {category: 0 for category in CEILINGS}, 'runs': []}


def debit_locked(category, run_id, frames):
    """Charge a run's planned GPU frames to one ceiling; the caller holds the locks."""
    if category not in CEILINGS:
        raise Refused(f'unknown frame category {category!r}')
    if type(frames) is not int or frames < 1 or not run_id:
        raise Refused('a frame debit needs a run id and a positive whole number of frames')
    state = frames_state()
    if any(row['run'] == run_id for row in state['runs']):
        raise Refused(f'{run_id} was already debited; choose a new run id')
    ceiling = CEILINGS[category]
    if state['debited'][category] + frames > ceiling:
        raise Refused(f'{run_id}: {state["debited"][category]} {category} frames debited + {frames} planned '
                      f'exceeds {ceiling}')
    state['debited'][category] += frames
    state['runs'].append({'run': run_id, 'category': category, 'frames': frames, 'at': now()})
    write_atomic(FRAMES, state)
    ledger({'event': 'debit', 'category': category, 'run': run_id, 'frames': frames,
            'total': state['debited'][category]})
    return state


def run(argv, *, label, timeout, env, overrides=None, cwd=ROOT, debit=None, gpu=False):
    """One guarded, bounded child; returns (exit status, stdout path, stderr path).

    `env` must be a constructed environment (see child_env) and is never recorded;
    `overrides` names the explicit values set in it, of which only ALLOWED_ENV keys
    are recorded. A `gpu` child first takes the global GPU lock; every child then
    takes the packet lock; a busy lock refuses. `debit=(category, run_id, frames)`
    charges a GPU child's frames under both locks, before the launch. Both locks
    are held until the child's group is gone and its exit is in the ledger.
    """
    guard()
    if not 0 < timeout <= (GPU_TIMEOUT if gpu else CARGO_TIMEOUT):
        raise Refused(f'{label}: timeout outside bounds')
    if debit is not None and not gpu:
        raise Refused(f'{label}: only GPU children draw frames')
    gpu_lock = (exclusive(NATIVE_GPU_LOCK, busy=f'{label}: the native GPU lock is held by another user')
                if gpu else contextlib.nullcontext())
    with gpu_lock, exclusive(GROUP_LOCK, busy='another 0069 process group is running'):
        guard()  # again under the locks: a release cannot slip in between
        if debit is not None:
            debit_locked(*debit)
        LOGS.mkdir(exist_ok=True)
        index = 0
        if LEDGER.exists():
            with LEDGER.open(encoding='utf-8') as stream:
                index = sum(1 for _ in stream)
        stem = LOGS / f'{index:04d}-{label}'
        record = {}
        start = {'event': 'start', 'label': label, 'timeout': timeout, 'argv': [str(a) for a in argv],
                 'cwd': str(cwd), 'env': recorded_env(overrides),
                 'gpu_lock': str(NATIVE_GPU_LOCK) if gpu else None}
        with open(f'{stem}.stdout.txt', 'wb') as out, open(f'{stem}.stderr.txt', 'wb') as err:
            try:
                supervise(argv, timeout=timeout, env=env, cwd=cwd, stdout=out, stderr=err, record=record,
                          started=lambda pid: ledger({**start, 'pid': pid}))
            finally:
                ledger({'event': 'exit', 'label': label, **record,
                        'stdout': f'{stem}.stdout.txt', 'stderr': f'{stem}.stderr.txt'})
    if record['timed_out']:
        raise Refused(f'{label} exceeded {timeout} s and was killed')
    if not record['group_reaped']:
        raise Refused(f'{label} left a process group that could not be reaped')
    return record['status'], Path(f'{stem}.stdout.txt'), Path(f'{stem}.stderr.txt')


def scratch(name):
    """A fresh per-child temporary directory inside the evidence area."""
    path = SCRATCH / name
    path.mkdir(parents=True)
    return path


def cargo(sub, args, *, label, extra_env=None):
    if sub not in ('build', 'clippy', 'check', 'test'):
        raise Refused(f'unsupported cargo subcommand {sub}')
    if sub == 'test' and '--no-run' not in args:
        raise Refused('cargo test only builds here (--no-run); regression.py runs the test binaries')
    joined = ' '.join(args)
    if ('--profile ci' not in joined or '--offline' not in args
            or any(a.startswith('-j') or a == '--jobs' for a in args)):
        raise Refused('cargo calls must pass --profile ci and --offline and leave jobs to this tool')
    guard()
    overrides = {'CARGO_TARGET_DIR': str(TARGET), **(extra_env or {})}
    env = child_env(scratch(label + '-' + str(time.time_ns())), overrides)
    argv = ['./tools/cargo', sub, '--locked', '-j2', *args]
    return run(argv, label=label, timeout=CARGO_TIMEOUT, env=env, overrides=overrides)


def release():
    guard()
    with exclusive(GROUP_LOCK, busy='cannot release: a 0069 process group holds the lock'):
        lease = guard()  # under the lock: nothing can start, debit or freeze from here on
        if RELEASE.exists():
            raise Refused('release.json exists without the released latch; stop and ask root')
        starts = {}
        for line in (LEDGER.read_text(encoding='utf-8') if LEDGER.exists() else '').splitlines():
            row = json.loads(line)
            if row['event'] == 'start':
                starts[row['pid']] = row['label']
        alive = [{'pid': pid, 'label': label} for pid, label in starts.items() if group_alive(pid)]
        users = target_users()
        if alive or users:
            raise Refused(f'cannot release: owned groups {alive}, target users {users}')
        frames = frames_state()
        # The ledger's final row; every later writer refuses once the latch exists.
        ledger({'event': 'release', 'children': len(starts), 'frames_debited': frames['debited']})
        record = {'packet': PACKET, 'released_at': now(), 'grant_sha256': lease['grant_sha256'],
                  'lease_sha256': digest(LEASE), 'ledger_sha256': digest(LEDGER),
                  'ledger_rows': len(LEDGER.read_text(encoding='utf-8').splitlines()),
                  'frames_sha256': digest(FRAMES) if FRAMES.exists() else None,
                  'children': len(starts), 'gpu_frames_debited': frames['debited'], 'frame_runs': frames['runs'],
                  'freeze_generations': [digest(p) for p in generations()],
                  'owned_processes_alive': alive, 'target_users': users,
                  'note': 'GPU and Cargo target released by 0069; target left in place as warm cache; '
                          'review-ready.json left untouched (only root writes it). The ledger is final: '
                          'its last row is the release event and nothing appends after it.'}
        write_new(RELEASE, json.dumps(record, indent=1) + '\n')
        write_new(RELEASED, json.dumps({'released_at': record['released_at'],
                                        'release_sha256': digest(RELEASE)}) + '\n')
        sync_directory(EVIDENCE)
    return record


def main():
    args = sys.argv[1:]
    try:
        if args == ['status']:
            print(json.dumps({'grant': GRANT.is_file() and digest(GRANT), 'lease': LEASE.is_file(),
                              'released': RELEASED.exists(), 'target_users': target_users()}, indent=1))
        elif args == ['acquire']:
            print(json.dumps(acquire(), indent=1))
        elif args in (['freeze'], ['freeze', '--new']):
            print(json.dumps(freeze(new=args[1:] == ['--new']), indent=1))
        elif args and args[0] == 'cargo' and len(args) >= 2:
            status, out, err = cargo(args[1], args[2:], label='cargo-' + args[1])
            sys.stdout.write(out.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.stderr.write(err.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.exit(status)
        elif args and args[0] == 'run' and '--' in args:
            # CPU children only: GPU and adapter children go through capture.py
            # or regression.py, which take the global GPU lock and debit frames.
            split = args.index('--')
            options, argv = args[1:split], args[split + 1:]
            label = options[options.index('--label') + 1]
            timeout = int(options[options.index('--timeout') + 1])
            if timeout > REGRESSION_TIMEOUT:
                raise Refused(f'non-Cargo children are bounded to {REGRESSION_TIMEOUT} s')
            guard()
            env = child_env(scratch(label + '-' + str(time.time_ns())))
            status, out, err = run(argv, label=label, timeout=timeout, env=env)
            sys.stdout.write(out.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.stderr.write(err.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.exit(status)
        elif args == ['release']:
            print(json.dumps(release(), indent=1))
        else:
            print(__doc__)
            sys.exit(2)
    except Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        sys.exit(3)


if __name__ == '__main__':
    main()
