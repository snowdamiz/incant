#!/usr/bin/env python3
"""0065 guard, ledger and bounded child runner. Every build/GPU child goes through here.

  lease.py acquire                  record this packet's lease on Astra's grant (create-only)
  lease.py status                   show grant/lease/release state
  lease.py cargo SUB ARGS...        ./tools/cargo SUB --locked -j2 ARGS (ARGS must select --profile ci)
  lease.py run --label L --timeout S [--gpu] -- ARGV...   bounded child, own process group
  lease.py release                  confirm no owned child/target user remains, write the marker

Refuses (exit 3, nothing started) after `released`, without the lease, or when Astra's
review-ready.json is missing, foreign, or differs from the hash recorded at acquisition.
Only Astra writes review-ready.json; this tool never edits or deletes it. GPU children
take an exclusive non-blocking lock so GPU groups stay serial. Timeouts kill the whole
process group and the ledger records whether anything survived.
"""
import datetime
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time

PACKET = '0065-core-live-hud'
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
EVIDENCE = Path('/Users/andreyyurlov/.codex/incant-evidence/0065')
GRANT = EVIDENCE / 'review-ready.json'
LEASE = EVIDENCE / 'lease.json'
RELEASED = EVIDENCE / 'released'
LEDGER = EVIDENCE / 'ledger.jsonl'
GPU_LOCK = EVIDENCE / 'gpu.lock'
LOGS = EVIDENCE / 'logs'
TARGET = Path('/Users/andreyyurlov/.codex/incant-core-presentation-target')
RUSTUP = '/opt/homebrew/opt/rustup/bin'
BRIEF = ROOT / 'handoffs' / PACKET / 'brief.md'
CARGO_TIMEOUT = 3600


class Refused(Exception):
    pass


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()


def ledger(row):
    with LEDGER.open('a', encoding='utf-8') as stream:
        stream.write(json.dumps({'at': now(), **row}) + '\n')


def grant():
    if not GRANT.is_file():
        raise Refused('no review-ready.json from Astra; not granted')
    value = json.loads(GRANT.read_text(encoding='utf-8'))
    if value.get('packet') != PACKET or value.get('target') != str(TARGET):
        raise Refused('review-ready.json does not grant this packet and target')
    return value


def guard():
    """Every child calls this immediately before it starts."""
    if RELEASED.exists():
        raise Refused('0065 lease was released; refusing')
    if not LEASE.is_file():
        raise Refused('no 0065 lease; run acquire first')
    lease = json.loads(LEASE.read_text(encoding='utf-8'))
    grant()
    if digest(GRANT) != lease['grant_sha256'] or lease.get('packet') != PACKET:
        raise Refused('review-ready.json changed since acquisition; stop and ask Astra')
    return lease


def target_users():
    """Processes (other than this tool) whose command line names the granted target."""
    rows = subprocess.run(['ps', '-axo', 'pid=,command='], capture_output=True, text=True, check=True).stdout
    users = []
    for line in rows.splitlines():
        pid, _, command = line.strip().partition(' ')
        if str(TARGET) in command and int(pid) != os.getpid() and 'lease.py' not in command:
            users.append({'pid': int(pid), 'command': command[:300]})
    return users


def acquire():
    if RELEASED.exists():
        raise Refused('0065 lease was released; refusing')
    value = grant()
    if LEASE.exists():
        raise Refused('lease already recorded')
    users = target_users()
    if users:
        raise Refused(f'target in use: {users}')
    lease = {'packet': PACKET, 'holder': 'claude (Opus 5.5, max thinking) for handoff 0065',
             'grant': value, 'grant_sha256': digest(GRANT), 'target_dir': str(TARGET),
             'brief_sha256': digest(BRIEF), 'worktree': str(ROOT), 'worktree_head': git('rev-parse', 'HEAD'),
             'acquired_at': now(), 'processes_on_target_at_acquire': users,
             'rules': ['./tools/cargo only', '--locked', '--profile ci', 'at most -j2',
                       'serial offscreen GPU groups under an exclusive lock', 'no native window or desktop capture',
                       '<= 24 retained frames, each <= 960x540', 'route children <= 300 s, capture <= 120 s',
                       'evidence and binaries only under ' + str(EVIDENCE)]}
    with LEASE.open('x', encoding='utf-8') as stream:
        stream.write(json.dumps(lease, indent=1) + '\n')
    ledger({'event': 'acquire', 'grant_sha256': lease['grant_sha256'], 'head': lease['worktree_head']})
    return lease


def reap(group):
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return True
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(group, sig)
        except ProcessLookupError:
            return True
        time.sleep(1)
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return True
    return False


def evidence_env(env):
    """Record only build inputs, never inherited account or agent credentials."""
    allowed = {'CARGO_TARGET_DIR', 'INCANT_NATIVE_PROJECT', 'RUSTFLAGS',
               'RUST_BACKTRACE', 'CARGO_BUILD_TARGET', 'CARGO_BUILD_JOBS'}
    return {key: value for key, value in (env or {}).items() if key in allowed}


def run(argv, *, label, timeout, gpu=False, env=None, cwd=ROOT):
    """Run one bounded child in its own process group; returns (exit, stdout path, stderr path)."""
    guard()
    LOGS.mkdir(exist_ok=True)
    index = sum(1 for _ in LEDGER.open(encoding='utf-8')) if LEDGER.exists() else 0
    stem = LOGS / f'{index:04d}-{label}'
    lock = None
    if gpu:
        lock = GPU_LOCK.open('a')
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            lock.close()
            raise Refused('another 0065 GPU group holds the lock')
    started = time.monotonic()
    try:
        with open(f'{stem}.stdout.txt', 'wb') as out, open(f'{stem}.stderr.txt', 'wb') as err:
            child = subprocess.Popen([str(a) for a in argv], cwd=cwd, env=env, stdout=out, stderr=err,
                                     start_new_session=True)
            ledger({'event': 'start', 'label': label, 'pid': child.pid, 'gpu': gpu, 'timeout': timeout,
                    'argv': [str(a) for a in argv], 'cwd': str(cwd),
                    'env': evidence_env(env)})
            timed_out = False
            try:
                status = child.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                reap(child.pid)
                status = child.wait()
        reaped = reap(child.pid)
        ledger({'event': 'exit', 'label': label, 'pid': child.pid, 'status': status, 'timed_out': timed_out,
                'seconds': round(time.monotonic() - started, 3), 'group_reaped': reaped,
                'stdout': f'{stem}.stdout.txt', 'stdout_sha256': digest(f'{stem}.stdout.txt'),
                'stderr': f'{stem}.stderr.txt', 'stderr_sha256': digest(f'{stem}.stderr.txt')})
        if timed_out:
            raise Refused(f'{label} exceeded {timeout} s and was killed')
        return status, Path(f'{stem}.stdout.txt'), Path(f'{stem}.stderr.txt')
    finally:
        if lock is not None:
            fcntl.flock(lock, fcntl.LOCK_UN)
            lock.close()


def cargo(sub, args, *, label, extra_env=None):
    if sub not in ('build', 'clippy', 'check'):
        raise Refused(f'unsupported cargo subcommand {sub}')
    joined = ' '.join(args)
    if '--profile ci' not in joined or any(a.startswith('-j') or a == '--jobs' for a in args):
        raise Refused('cargo calls must pass --profile ci and leave jobs to this tool')
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_NATIVE_', 'INCANT_MOBILE_', 'CARGO_PROFILE_', 'CARGO_TARGET_DIR'))
           and k not in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER')}
    env.update(CARGO_TARGET_DIR=str(TARGET), PATH=RUSTUP + ':' + env.get('PATH', ''))
    env.update(extra_env or {})
    argv = ['./tools/cargo', sub, '--locked', '-j2', *args]
    return run(argv, label=label, timeout=CARGO_TIMEOUT, env=env)


def release():
    lease = guard()
    alive = []
    starts = {}
    for line in LEDGER.read_text(encoding='utf-8').splitlines():
        row = json.loads(line)
        if row['event'] == 'start':
            starts[row['pid']] = row['label']
    for pid, label in starts.items():
        try:
            os.killpg(pid, 0)
            alive.append({'pid': pid, 'label': label})
        except ProcessLookupError:
            pass
    users = target_users()
    if alive or users:
        raise Refused(f'cannot release: owned groups {alive}, target users {users}')
    frames = sorted(str(p.relative_to(EVIDENCE)) for p in (EVIDENCE / 'captures').rglob('*.rgba')) \
        if (EVIDENCE / 'captures').exists() else []
    record = {'packet': PACKET, 'released_at': now(), 'grant_sha256': lease['grant_sha256'],
              'lease_sha256': digest(LEASE), 'ledger_sha256': digest(LEDGER),
              'children': len(starts), 'retained_frames': len(frames), 'frames': frames,
              'owned_processes_alive': alive, 'target_users': users,
              'note': 'GPU and Cargo target released by 0065; target left in place as warm cache; '
                      'review-ready.json left untouched (only Astra writes it).'}
    (EVIDENCE / 'release.json').write_text(json.dumps(record, indent=1) + '\n', encoding='utf-8')
    with RELEASED.open('x', encoding='utf-8') as stream:
        stream.write(record['released_at'] + '\n')
    ledger({'event': 'release', 'release_sha256': digest(EVIDENCE / 'release.json')})
    return record


def main():
    args = sys.argv[1:]
    try:
        if args == ['acquire']:
            print(json.dumps(acquire(), indent=1))
        elif args == ['status']:
            print(json.dumps({'grant': GRANT.is_file() and digest(GRANT), 'lease': LEASE.is_file(),
                              'released': RELEASED.exists(), 'target_users': target_users()}, indent=1))
        elif args and args[0] == 'cargo' and len(args) >= 2:
            status, out, err = cargo(args[1], args[2:], label='cargo-' + args[1])
            sys.stdout.write(out.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.stderr.write(err.read_text(encoding='utf-8', errors='replace')[-4000:])
            sys.exit(status)
        elif args and args[0] == 'run' and '--' in args:
            split = args.index('--')
            options, argv = args[1:split], args[split + 1:]
            label = options[options.index('--label') + 1]
            timeout = int(options[options.index('--timeout') + 1])
            status, out, err = run(argv, label=label, timeout=timeout, gpu='--gpu' in options)
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
