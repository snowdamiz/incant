#!/usr/bin/env python3
"""Packet-local lease guard for handoff 0061 builds and runs.

Every build or run goes through this wrapper. It refuses unless the packet
lease exists, is unexpired, names this worktree and the assigned target, and the
release marker is absent. Each invocation is appended to the ledger with argv,
exit status and SHA-256 of every existing file argument and produced binary.

  guard.py acquire                 create a fresh 12-hour lease
  guard.py check                   validate the lease
  guard.py cargo <cargo args...>   ./tools/cargo <args> --locked --profile ci -j 2
  guard.py run -- <argv...>        run one command (cwd = worktree)
  guard.py release                 write the release marker; later calls refuse
"""
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

WORKTREE = Path(__file__).resolve().parents[3]
STATE = WORKTREE / 'artifacts/0061'
LEASE = STATE / 'lease.json'
RELEASED = STATE / 'released'
LEDGER = STATE / 'ledger.jsonl'
TARGET = Path('/Users/andreyyurlov/.codex/incant-core-presentation-target')
RUSTUP_BIN = '/opt/homebrew/opt/rustup/bin'
PACKET = '0061-planar-sample-presentation'


class Refused(Exception):
    pass


def now():
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0)


def digest(path):
    h = hashlib.sha256()
    with open(path, 'rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            h.update(block)
    return h.hexdigest()


def git(*args):
    return subprocess.run(['git', *args], cwd=WORKTREE, capture_output=True, text=True,
                          check=True).stdout.strip()


def check():
    """Return the lease or raise Refused. Called before every child process."""
    if RELEASED.exists():
        raise Refused(f'lease released ({RELEASED}); no further builds or runs')
    if not LEASE.is_file():
        raise Refused('no packet lease; run guard.py acquire')
    lease = json.loads(LEASE.read_text())
    if lease.get('packet') != PACKET or lease.get('worktree') != str(WORKTREE):
        raise Refused('lease belongs to another packet or worktree')
    if lease.get('cargo_target_dir') != str(TARGET):
        raise Refused('lease names a different target')
    if now() >= dt.datetime.fromisoformat(lease['expires_at']):
        raise Refused('lease expired')
    return lease


def record(entry):
    STATE.mkdir(parents=True, exist_ok=True)
    with open(LEDGER, 'a') as file:
        file.write(json.dumps(entry, sort_keys=True) + '\n')


def file_hashes(argv):
    hashes = {}
    for arg in argv:
        path = Path(arg)
        if path.is_file() and path.stat().st_size <= 256 * 1024 * 1024:
            hashes[str(path)] = digest(path)
    return hashes


def acquire():
    if LEASE.exists() or RELEASED.exists():
        raise Refused('a lease or release marker already exists; leases are never reused')
    if not TARGET.is_dir():
        raise Refused(f'assigned target is missing: {TARGET}')
    started = now()
    lease = {
        'packet': PACKET,
        'owner': 'Claude Opus 5.5 (claude-opus-5-5), thinking effort max, ACP handoff session',
        'granted_by': f'handoffs/{PACKET}/brief.md, "Builds, evidence and desktop coordination"',
        'brief_sha256': digest(WORKTREE / f'handoffs/{PACKET}/brief.md'),
        'worktree': str(WORKTREE),
        'source_head': git('rev-parse', 'HEAD'),
        'cargo_target_dir': str(TARGET),
        'cargo_policy': './tools/cargo, --locked, Rust 1.99.0, --profile ci, -j 2',
        'started_at': started.isoformat(),
        'expires_at': (started + dt.timedelta(hours=12)).isoformat(),
        'release_marker': str(RELEASED),
        'ledger': str(LEDGER),
        'scope': 'exclusive warm cargo target and offscreen renderer runs; no native window '
                 'or screen capture before 0060 releases the desktop',
    }
    STATE.mkdir(parents=True, exist_ok=True)
    with open(LEASE, 'x') as file:
        file.write(json.dumps(lease, indent=2) + '\n')
    record({'event': 'acquire', 'at': started.isoformat(), 'lease_sha256': digest(LEASE)})
    print(json.dumps(lease, indent=2))


def execute(argv, environment=None):
    check()
    started = now()
    before = file_hashes(argv)
    result = subprocess.run(argv, cwd=WORKTREE, env=environment)
    entry = {'event': 'run', 'argv': argv, 'started_at': started.isoformat(),
             'ended_at': now().isoformat(), 'exit': result.returncode, 'inputs': before}
    record(entry)
    return result.returncode


def cargo(args):
    for forbidden in ('--target-dir', '-j', '--jobs', '--profile', '--release', '--locked'):
        if any(a == forbidden or a.startswith(forbidden + '=') for a in args):
            raise Refused(f'{forbidden} is fixed by the guard')
    environment = dict(os.environ)
    environment['PATH'] = RUSTUP_BIN + os.pathsep + environment.get('PATH', '')
    environment['CARGO_TARGET_DIR'] = str(TARGET)
    command = ['./tools/cargo', *args, '--locked', '--profile', 'ci', '-j', '2']
    code = execute(command, environment)
    outputs = {}
    for name in ('incant_headless', 'examples/native_frames'):
        path = TARGET / 'ci' / name
        if path.is_file():
            outputs[str(path)] = digest(path)
    record({'event': 'cargo-outputs', 'at': now().isoformat(), 'exit': code, 'outputs': outputs})
    return code


def release():
    lease = check()
    busy = subprocess.run(['pgrep', '-fl', str(TARGET)], capture_output=True, text=True).stdout
    owned = [line for line in busy.splitlines() if 'guard.py' not in line]
    if owned:
        raise Refused('processes still use the target:\n' + '\n'.join(owned))
    released = now().isoformat()
    RELEASED.write_text(json.dumps({'packet': PACKET, 'released_at': released,
                                    'lease_started_at': lease['started_at']}) + '\n')
    record({'event': 'release', 'at': released})
    print(released)


def main(argv):
    try:
        if argv[:1] == ['acquire']:
            acquire()
        elif argv[:1] == ['check']:
            print(json.dumps(check(), indent=2))
        elif argv[:1] == ['cargo']:
            return cargo(argv[1:])
        elif argv[:2] == ['run', '--'] and len(argv) > 2:
            return execute(argv[2:])
        elif argv[:1] == ['release']:
            release()
        else:
            print(__doc__, file=sys.stderr)
            return 2
    except Refused as error:
        print(f'guard refused: {error}', file=sys.stderr)
        return 3
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
