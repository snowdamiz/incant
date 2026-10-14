"""CPU-only exact fixture build. No test execution beyond ignored-name enumeration."""
from pathlib import Path
import datetime
import fcntl
import hashlib
import json
import os
import shutil
import signal
import subprocess
import time

E = Path(__file__).resolve().parent
R = Path('/Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant')
P = Path('/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review')
T = '/Users/andreyyurlov/.codex/incant-gpu-timestamps-target'
SOURCE = '825ff047284e84252d00f786a7b14cb6ecb44293'
FREEZE = '9e36e8c3a8d7253a3619a85f8cb8e3e313c663bb37c9f15dab5946528c164f62'
FIXTURE = '650316a21f30bfb4f7cbe8c99b67f9b5be269b51eb62b4d017b519bd5130d997'
QUALITY = 'ff0f0f488faa3f6d4d6cc1321d5c4e87c35cbb4160130dddd050697bb824dbc4'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
now = lambda: datetime.datetime.now(datetime.timezone.utc).isoformat()
checks = []
started = now()
dst = R / 'crates/incant_render/tests/upscaling_owner_review.rs'
injected = False
lock = (E.parent / 'r4-quality-build-target.lock').open('a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)

def save(name, obj):
    (E / name).write_text(json.dumps(obj, indent=2) + '\n')

def interrupted(number, _frame):
    raise InterruptedError(f'signal {number}')

signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGINT, interrupted)

assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=R, text=True).strip() == SOURCE
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=R)
assert not dst.exists()
for src, name, digest in [
    (P / 'r4_quality/production-freeze.json', 'production-freeze.json', FREEZE),
    (P / 'r4_quality/review/upscaling_owner_review.rs', 'fixture.rs', FIXTURE),
    (P / 'owner_quality/plan.json', 'plan.json', QUALITY),
]:
    assert sha(src) == digest
    assert not (E / name).exists()
    shutil.copy2(src, E / name)
freeze = json.loads((E / 'production-freeze.json').read_text())
assert freeze['source_commit'] == SOURCE and len(freeze['files']) == 2839

def verify():
    for row in freeze['files']:
        p = R / row['path']
        assert p.stat().st_size == row['bytes'] and sha(p) == row['sha256'], row['path']
    return len(freeze['files'])

verify()
env = {
    'HOME': str(Path.home()),
    'PATH': str(Path.home() / '.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin') + ':/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin',
    'CARGO_TARGET_DIR': T, 'CARGO_BUILD_JOBS': '1', 'CARGO_NET_OFFLINE': 'true', 'LC_ALL': 'C',
}
save('authority.json', dict(authority='root collaboration task', scope='CPU fixture build, scoped Clippy, ignored-name enumeration only', source=SOURCE, target=T, jobs=1, granted_ceiling=2, gpu_executions=0, started=started, environment=env))

def group_exists(pid):
    try:
        os.killpg(pid, 0)
        return True
    except ProcessLookupError:
        return False

def clean_group(p):
    if group_exists(p.pid):
        os.killpg(p.pid, signal.SIGTERM)
        until = time.monotonic() + 5
        while time.monotonic() < until and group_exists(p.pid):
            p.poll()
            time.sleep(0.05)
        if group_exists(p.pid):
            os.killpg(p.pid, signal.SIGKILL)
    p.wait()
    return not group_exists(p.pid)

def run(name, argv, timeout=900):
    print('START ' + name, flush=True)
    t = time.monotonic()
    p = None
    rc = None
    error = None
    empty = True
    try:
        with (E / (name + '.log')).open('xb') as out:
            p = subprocess.Popen(argv, cwd=R, env=env, stdout=out, stderr=subprocess.STDOUT, start_new_session=True)
            rc = p.wait(timeout=timeout)
    except BaseException as exc:
        error = repr(exc)
        raise
    finally:
        if p is not None:
            empty = clean_group(p)
        entry = dict(name=name, argv=argv, exit_code=rc, seconds=round(time.monotonic()-t, 3), pid=p.pid if p else None,
                     reaped=p is not None and p.poll() is not None, group_empty=empty, error=error,
                     log_sha256=sha(E / (name + '.log')))
        checks.append(entry)
        save('checks.json', checks)
        print('DONE ' + name + ' ' + str(rc), flush=True)
    assert rc == 0 and empty, entry
    verify()

error = None
try:
    shutil.copy2(E / 'fixture.rs', dst)
    injected = True
    q = json.loads((E / 'plan.json').read_text())
    argv = lambda role: ['-j1' if x == '-j2' else x for x in q['build'][role]]
    run('build', argv('review'))
    artifacts = []
    library = []
    for line in (E / 'build.log').read_text().splitlines():
        try:
            a = json.loads(line)
        except ValueError:
            continue
        if a.get('reason') != 'compiler-artifact':
            continue
        if a.get('target', {}).get('name') == 'upscaling_owner_review' and a.get('executable'):
            artifacts.append(a)
        if a.get('target', {}).get('name') == 'incant_render' and 'lib' in a['target']['kind']:
            library.append(a)
    assert len(artifacts) == len(library) == 1
    a = artifacts[0]
    assert a['features'] == library[0]['features'] == ['default', 'physics-deterministic']
    b = E / 'upscaling_owner_review'
    shutil.copy2(a['executable'], b)
    b.chmod(0o555)
    save('artifact.json', a)
    save('library-artifact.json', library[0])
    run('clippy', argv('lint'))
    run('listing', [str(b), '--list', '--ignored'], 30)
    names = [x[:-6] for x in (E / 'listing.log').read_text().splitlines() if x.endswith(': test')]
    assert sorted(names) == sorted(q['build']['expected_ignored_tests']), names
    receipt = dict(schema='incant-owner-quality-build-v1', at=now(), source_commit=SOURCE, working_head=SOURCE,
                   production_freeze_sha256=FREEZE, verified_production_files=verify(), fixture_sha256=sha(dst),
                   plan_sha256=QUALITY, executable=str(b), executable_sha256=sha(b), features=a['features'],
                   checks=checks, gpu_executions=0, target=T)
    save('receipt.json', receipt)
    print(json.dumps(receipt), flush=True)
except BaseException as exc:
    error = repr(exc)
    raise
finally:
    cleanup_error = None
    try:
        if injected:
            assert sha(dst) == FIXTURE, 'injected fixture changed; preserve it'
            dst.unlink()
        verify()
        assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=R)
    except BaseException as exc:
        cleanup_error = repr(exc)
    save('release.json', dict(at=now(), started=started, target=T, source=SOURCE, error=error, cleanup_error=cleanup_error,
                              children=checks, all_reaped=all(x['reaped'] and x['group_empty'] for x in checks),
                              fixture_removed=not dst.exists(), gpu_executions=0))
    fcntl.flock(lock, fcntl.LOCK_UN)
    lock.close()
    if cleanup_error is not None and error is None:
        raise RuntimeError(cleanup_error)
