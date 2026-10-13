"""Synthetic controls for lease.py and regression.py: child lifecycle, locks,
debits, release, identity, freeze and the regression case list.

Run from this directory: python3 -B -m unittest test_lease

Each lifecycle control starts a tiny /bin/sh child that records its own pid, leaves
a background grandchild (`sleep 30`) in its process group, then exits normally,
exits nonzero, waits past its timeout, ignores SIGTERM, or is interrupted by a
KeyboardInterrupt or SystemExit raised in this supervising process, or by a real
SIGINT, SIGTERM or SIGHUP sent to it. Afterwards every recorded pid and the whole
group must be gone. Further controls parse porcelain status from a throwaway git
repository, take the global GPU lock (a scratch stand-in) before the packet lock
and refuse at once when either is busy, charge frames to the two ceilings under
both locks, race a run against a release that holds the packet lock until its
latch is written, check that the released ledger hash stays final, step freeze
generations, check that a grant must name the reserved CPU target, and check the
regression case list and its coverage labels against its pinned sources and the
historical base (read only). Temporary files live in a fresh
directory under tools/test-scratch and are removed. Nothing here builds, uses a
GPU, touches the Cargo target, the shared GPU lock, the grant or the evidence
area: the guard, ledger, logs, both locks, frame ledger, freezes and release
files point at the scratch.
"""
import contextlib
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lease  # noqa: E402
import regression  # noqa: E402

SCRATCH = HERE / 'test-scratch'
SCRIPT = 'echo $$ >> "$1"; {prelude} sleep 30 & echo $! >> "$1"; {tail}'
ENV = {'PATH': '/usr/bin:/bin'}


def gone(pid, seconds=3.0):
    """True once `pid` no longer exists (killed grandchildren are reaped by launchd)."""
    deadline = time.monotonic() + seconds
    while True:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return True
        except PermissionError:
            pass
        if time.monotonic() > deadline:
            return False
        time.sleep(0.02)


class Interrupt:
    """Raise `exception` in this process `after` seconds, via a one-shot SIGALRM."""

    def __init__(self, exception, after):
        self.exception, self.after = exception, after

    def __enter__(self):
        def handler(signum, frame):
            raise self.exception()
        self.previous = signal.signal(signal.SIGALRM, handler)
        signal.setitimer(signal.ITIMER_REAL, self.after)
        return self

    def __exit__(self, *exc):
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, self.previous)
        return False


class Deliver(Interrupt):
    """Send the real signal `sig` to this process `after` seconds."""

    def __init__(self, sig, after):
        super().__init__(None, after)
        self.sig = sig

    def __enter__(self):
        def handler(signum, frame):
            os.kill(os.getpid(), self.sig)
        self.previous = signal.signal(signal.SIGALRM, handler)
        signal.setitimer(signal.ITIMER_REAL, self.after)
        return self


@contextlib.contextmanager
def patched(**values):
    """Point lease module globals at scratch values for one control."""
    saved = {name: getattr(lease, name) for name in values}
    for name, value in values.items():
        setattr(lease, name, value)
    try:
        yield
    finally:
        for name, value in saved.items():
            setattr(lease, name, value)


def scratch_guard():
    """The real guard's refusal after release, without a grant or lease."""
    if lease.RELEASED.exists():
        raise lease.Refused('0069 lease was released; refusing')
    return {'packet': lease.PACKET, 'grant_sha256': '0' * 64}


# A child that reports whether `path` (argv[1]) and `path2` (argv[2]) are locked
# by someone else, into argv[3]; it never blocks.
HOLDS = """
import fcntl, sys
held = []
for path in sys.argv[1:3]:
    with open(path, 'a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held.append(False)
        except BlockingIOError:
            held.append(True)
open(sys.argv[3], 'w').write(repr(held))
"""


def holder(path):
    """An exclusive flock on `path` through a separate open file description."""
    lock = open(path, 'a')
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    return lock


def git(root, *args):
    subprocess.run(['git', '-c', 'user.name=control', '-c', 'user.email=control@invalid', '-c', 'commit.gpgsign=false',
                    '-c', 'core.hooksPath=/dev/null', *args], cwd=root, check=True, capture_output=True,
                   env=lease.child_env(None))


class Lifecycle(unittest.TestCase):
    def setUp(self):
        SCRATCH.mkdir(exist_ok=True)
        self.dir = Path(tempfile.mkdtemp(dir=SCRATCH))

    def tearDown(self):
        shutil.rmtree(self.dir)

    @classmethod
    def tearDownClass(cls):
        if SCRATCH.exists() and not any(SCRATCH.iterdir()):
            SCRATCH.rmdir()

    def argv(self, tail, prelude=''):
        return ['/bin/sh', '-c', SCRIPT.format(prelude=prelude, tail=tail), 'control', str(self.dir / 'pids')]

    def assert_nothing_survives(self, record):
        pids = [int(line) for line in (self.dir / 'pids').read_text().split()]
        self.assertEqual(len(pids), 2, pids)  # the shell and its background grandchild
        self.assertEqual(pids[0], record['pid'])
        self.assertTrue(record['group_reaped'], record)
        self.assertFalse(lease.group_alive(record['pid']), record)
        for pid in pids:
            self.assertTrue(gone(pid), pid)

    def supervise(self, argv, timeout, record):
        with open(self.dir / 'out', 'wb') as out, open(self.dir / 'err', 'wb') as err:
            return lease.supervise(argv, timeout=timeout, env=ENV, cwd=self.dir, stdout=out, stderr=err,
                                   record=record)

    def test_normal_exit_still_kills_the_left_behind_grandchild(self):
        record = self.supervise(self.argv('exit 0'), 10, {})
        self.assertEqual((record['status'], record['timed_out'], record['interrupted']), (0, False, None))
        self.assert_nothing_survives(record)

    def test_nonzero_exit(self):
        record = self.supervise(self.argv('exit 7'), 10, {})
        self.assertEqual((record['status'], record['timed_out']), (7, False))
        self.assert_nothing_survives(record)

    def test_timeout_kills_the_group(self):
        begun = time.monotonic()
        record = self.supervise(self.argv('wait'), 1, {})
        self.assertTrue(record['timed_out'])
        self.assertEqual(record['status'], -signal.SIGTERM)
        self.assertLess(time.monotonic() - begun, 1 + lease.GRACE + 1)
        self.assert_nothing_survives(record)

    def test_a_group_ignoring_sigterm_is_killed(self):
        record = self.supervise(self.argv('wait', prelude="trap '' TERM;"), 1, {})
        self.assertTrue(record['timed_out'])
        self.assertEqual(record['status'], -signal.SIGKILL)
        self.assertGreaterEqual(record['seconds'], 1 + lease.GRACE)
        self.assert_nothing_survives(record)

    def test_keyboard_interrupt_and_system_exit_kill_the_group_then_propagate(self):
        for exception in (KeyboardInterrupt, SystemExit):
            with self.subTest(exception=exception.__name__):
                (self.dir / 'pids').unlink(missing_ok=True)
                record = {}
                with self.assertRaises(exception), Interrupt(exception, 0.5):
                    self.supervise(self.argv('wait'), 30, record)
                self.assertEqual(record['interrupted'], exception.__name__)
                self.assertFalse(record['timed_out'])
                self.assert_nothing_survives(record)

    def test_real_termination_signals_kill_the_group_then_propagate(self):
        for sig, exception, code in ((signal.SIGINT, KeyboardInterrupt, None),
                                     (signal.SIGTERM, SystemExit, 128 + signal.SIGTERM),
                                     (signal.SIGHUP, SystemExit, 128 + signal.SIGHUP)):
            with self.subTest(signal=sig.name):
                (self.dir / 'pids').unlink(missing_ok=True)
                before = {s: signal.getsignal(s) for s in (signal.SIGTERM, signal.SIGHUP)}
                record = {}
                with self.assertRaises(exception) as caught, Deliver(sig, 0.5):
                    self.supervise(self.argv('wait'), 30, record)
                if code is not None:
                    self.assertEqual(caught.exception.code, code)
                self.assertEqual(record['interrupted'], exception.__name__)
                self.assert_nothing_survives(record)
                # The previous handlers are back once the child is gone.
                self.assertEqual({s: signal.getsignal(s) for s in before}, before)

    def scratch_lease(self, **extra):
        (self.dir / 'lease.json').write_text('{"synthetic": true}\n')
        return patched(guard=scratch_guard, target_users=lambda: [], EVIDENCE=self.dir,
                       LEASE=self.dir / 'lease.json', LEDGER=self.dir / 'ledger.jsonl', LOGS=self.dir / 'logs',
                       GROUP_LOCK=self.dir / 'group.lock', FRAMES=self.dir / 'frames.json',
                       FREEZES=self.dir / 'freeze', RELEASE=self.dir / 'release.json',
                       RELEASED=self.dir / 'released', NATIVE_GPU_LOCK=self.dir / 'native-gpu.lock', **extra)

    def events(self):
        if not lease.LEDGER.exists():
            return []
        return [json.loads(line)['event'] for line in lease.LEDGER.read_text().splitlines()]

    def test_cpu_only_grant_refuses_before_locks_debit_or_launch(self):
        with self.scratch_lease():
            with patched(guard=lambda: {'grant': {'cpu_only': True}}):
                with self.assertRaisesRegex(lease.Refused, 'CPU-only grant forbids GPU'):
                    lease.run(['/bin/sh', '-c', 'exit 99'], label='forbidden', timeout=1,
                              env=ENV, gpu=True, debit=('appearance', 'forbidden', 1))
                self.assertEqual(self.events(), [])
                for path in (lease.NATIVE_GPU_LOCK, lease.GROUP_LOCK, lease.FRAMES, lease.LOGS):
                    self.assertFalse(path.exists(), path)

    def test_run_records_the_outcome_and_unlocks_only_after_the_group_is_gone(self):
        with self.scratch_lease():
            status, out, err = lease.run(self.argv('exit 3'), label='normal', timeout=10, env=ENV, cwd=self.dir)
            self.assertEqual(status, 3)
            (self.dir / 'pids').unlink()
            with self.assertRaises(KeyboardInterrupt), Interrupt(KeyboardInterrupt, 0.5):
                lease.run(self.argv('wait'), label='interrupted', timeout=30, env=ENV, cwd=self.dir,
                          overrides={'INCANT_NATIVE_PROJECT': '/synthetic/core.json'})
            rows = [json.loads(line) for line in lease.LEDGER.read_text().splitlines()]
            self.assertEqual([r['event'] for r in rows], ['start', 'exit', 'start', 'exit'])
            self.assertEqual(rows[2]['env'], {'INCANT_NATIVE_PROJECT': '/synthetic/core.json'})
            exit_row = rows[3]
            self.assertEqual((exit_row['interrupted'], exit_row['group_reaped']), ('KeyboardInterrupt', True))
            self.assert_nothing_survives(exit_row)
            # The lock was released: another holder can take it at once.
            with lease.GROUP_LOCK.open('a') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                fcntl.flock(lock, fcntl.LOCK_UN)
            # A child without a constructed environment is refused before it starts.
            with self.assertRaises(lease.Refused):
                lease.run(['/usr/bin/true'], label='inherited', timeout=10, env=None, cwd=self.dir)

    def test_frames_are_debited_to_two_ceilings_atomically_under_the_locks_before_any_launch(self):
        with self.scratch_lease():
            gpu = lambda label, debit: lease.run(['/usr/bin/true'], label=label, timeout=10, env=ENV, cwd=self.dir,
                                                 debit=debit, gpu=True)
            gpu('first', ('appearance', 'first', 4))
            state = json.loads(lease.FRAMES.read_text())
            self.assertEqual(state['debited'], {'appearance': 4, 'regression': 0})
            self.assertEqual(self.events(), ['debit', 'start', 'exit'])
            # The regression ceiling is separate: 39 tiny frames leave appearance alone.
            gpu('regression', ('regression', 'regression', 39))
            for debit in (('appearance', 'over', 21), ('regression', 'big', 90), ('appearance', 'zero', 0),
                          ('appearance', 'negative', -1), ('appearance', 'flag', True),
                          ('appearance', 'fraction', 1.0), ('appearance', 'first', 1),
                          ('regression', 'first', 1), ('appearance', '', 1), ('other', 'unknown', 1)):
                with self.subTest(debit=debit), self.assertRaises(lease.Refused):
                    gpu('refused', debit)
            # CPU children never debit frames.
            with self.assertRaises(lease.Refused):
                lease.run(['/usr/bin/true'], label='cpu', timeout=10, env=ENV, cwd=self.dir,
                          debit=('appearance', 'cpu', 1))
            # While another holder has the packet lock, nothing is debited or started.
            with holder(lease.GROUP_LOCK):
                with self.assertRaises(lease.Refused):
                    gpu('busy', ('appearance', 'busy', 1))
            state = json.loads(lease.FRAMES.read_text())
            self.assertEqual(state['debited'], {'appearance': 4, 'regression': 39})
            self.assertEqual([(r['category'], r['run']) for r in state['runs']],
                             [('appearance', 'first'), ('regression', 'regression')])
            self.assertEqual(self.events().count('start'), 2)
            self.assertEqual([p.name for p in self.dir.iterdir() if p.name.startswith('.frames')], [])
            # Up to each ceiling exactly, then nothing more.
            gpu('rest', ('appearance', 'rest', 20))
            gpu('regression-rest', ('regression', 'regression-rest', 89))
            for debit in (('appearance', 'past', 1), ('regression', 'past-regression', 1)):
                with self.subTest(debit=debit), self.assertRaises(lease.Refused):
                    gpu('past', debit)
            self.assertEqual(json.loads(lease.FRAMES.read_text())['debited'], {'appearance': 24, 'regression': 128})

    def test_gpu_children_take_the_global_lock_first_and_hold_both_through_accounting(self):
        taken = []
        original = lease.exclusive

        def recording(path, **options):
            taken.append(Path(path).name)
            return original(path, **options)

        with self.scratch_lease(exclusive=recording):
            report = self.dir / 'held'
            status, _, _ = lease.run([sys.executable, '-I', '-B', '-c', HOLDS, lease.NATIVE_GPU_LOCK,
                                      lease.GROUP_LOCK, report], label='holds', timeout=10, env=ENV, cwd=self.dir,
                                     debit=('appearance', 'holds', 1), gpu=True)
            self.assertEqual(status, 0)
            self.assertEqual(taken, ['native-gpu.lock', 'group.lock'])  # the global lock first
            self.assertEqual(report.read_text(), '[True, True]')  # both held while the child ran
            rows = [json.loads(line) for line in lease.LEDGER.read_text().splitlines()]
            self.assertEqual([r['event'] for r in rows], ['debit', 'start', 'exit'])
            self.assertEqual(rows[1]['gpu_lock'], str(lease.NATIVE_GPU_LOCK))
            # Both are free again once the exit is recorded.
            for path in (lease.NATIVE_GPU_LOCK, lease.GROUP_LOCK):
                holder(path).close()
            # A CPU child neither needs nor takes the global lock.
            taken.clear()
            with holder(lease.NATIVE_GPU_LOCK):
                lease.run([sys.executable, '-I', '-B', '-c', HOLDS, lease.NATIVE_GPU_LOCK, lease.GROUP_LOCK,
                           report], label='cpu', timeout=10, env=ENV, cwd=self.dir)
            self.assertEqual(taken, ['group.lock'])
            self.assertEqual(report.read_text(), '[True, True]')  # the test's own hold, and the packet lock
            self.assertIsNone(json.loads(lease.LEDGER.read_text().splitlines()[-2])['gpu_lock'])

    def test_a_busy_lock_refuses_at_once_without_side_effects(self):
        with self.scratch_lease():
            # Another user holds the global lock: refused at once, with nothing
            # debited, started or written, and the packet lock left free.
            with holder(lease.NATIVE_GPU_LOCK):
                begun = time.monotonic()
                with self.assertRaises(lease.Refused):
                    lease.run(['/usr/bin/true'], label='busy-gpu', timeout=10, env=ENV, cwd=self.dir,
                              debit=('appearance', 'busy-gpu', 1), gpu=True)
                self.assertLess(time.monotonic() - begun, 0.5)
                holder(lease.GROUP_LOCK).close()
            # The packet lock is busy: the global lock is taken, then given back at once.
            with holder(lease.GROUP_LOCK):
                begun = time.monotonic()
                with self.assertRaises(lease.Refused):
                    lease.run(['/usr/bin/true'], label='busy', timeout=10, env=ENV, cwd=self.dir,
                              debit=('appearance', 'busy', 1), gpu=True)
                self.assertLess(time.monotonic() - begun, 0.5)
                holder(lease.NATIVE_GPU_LOCK).close()
            self.assertFalse(lease.FRAMES.exists() or lease.LEDGER.exists())
            # Once both are free, the same run proceeds.
            lease.run(['/usr/bin/true'], label='free', timeout=10, env=ENV, cwd=self.dir,
                      debit=('appearance', 'free', 1), gpu=True)
            self.assertEqual(self.events(), ['debit', 'start', 'exit'])

    def test_release_holds_the_packet_lock_until_its_latch_is_durable(self):
        with self.scratch_lease():
            lease.run(['/usr/bin/true'], label='child', timeout=10, env=ENV, cwd=self.dir,
                      debit=('appearance', 'child', 2), gpu=True)
            original = lease.write_new
            probed = []

            def probe(path, text):
                # Between the final ledger row and the latch: a racing run and a
                # racing lock holder are both refused, and the ledger is unchanged.
                if path == lease.RELEASE:
                    ledger = lease.LEDGER.read_bytes()
                    with self.assertRaises(lease.Refused):
                        lease.run(['/usr/bin/true'], label='racing', timeout=10, env=ENV, cwd=self.dir,
                                  debit=('appearance', 'racing', 1), gpu=True)
                    with open(lease.GROUP_LOCK, 'a') as lock, self.assertRaises(BlockingIOError):
                        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    self.assertEqual(lease.LEDGER.read_bytes(), ledger)
                    probed.append(path.name)
                return original(path, text)

            with patched(write_new=probe):
                record = lease.release()
            self.assertEqual(probed, ['release.json'])
            self.assertEqual(self.events(), ['debit', 'start', 'exit', 'release'])
            self.assertEqual(json.loads(lease.FRAMES.read_text())['debited'], {'appearance': 2, 'regression': 0})
            latch = json.loads(lease.RELEASED.read_text())
            self.assertEqual(latch['release_sha256'], lease.digest(lease.RELEASE))
            self.assertEqual(json.loads(lease.RELEASE.read_text()), record)
            holder(lease.GROUP_LOCK).close()

    def test_the_released_ledger_hash_is_final(self):
        with self.scratch_lease():
            lease.run(['/usr/bin/true'], label='child', timeout=10, env=ENV, cwd=self.dir)
            record = lease.release()
            rows = [json.loads(line) for line in lease.LEDGER.read_text().splitlines()]
            self.assertEqual(rows[-1]['event'], 'release')
            self.assertEqual((record['ledger_sha256'], record['ledger_rows']),
                             (lease.digest(lease.LEDGER), len(rows)))
            frozen = {path: path.read_bytes() for path in (lease.LEDGER, lease.RELEASE, lease.RELEASED)}
            # Every writer refuses afterwards, so the hash stays the ledger's.
            for attempt in (lambda: lease.run(['/usr/bin/true'], label='late', timeout=10, env=ENV, cwd=self.dir),
                            lambda: lease.run(['/usr/bin/true'], label='late-gpu', timeout=10, env=ENV,
                                              cwd=self.dir, debit=('appearance', 'late', 1), gpu=True),
                            lambda: lease.freeze(), lambda: lease.freeze(new=True), lease.release):
                with self.assertRaises(lease.Refused):
                    attempt()
            self.assertEqual({path: path.read_bytes() for path in frozen}, frozen)
            self.assertFalse(lease.FRAMES.exists())

    def test_release_refuses_with_a_live_group_a_held_lock_or_a_partial_release(self):
        with self.scratch_lease():
            live = subprocess.Popen(['/bin/sleep', '30'], start_new_session=True)
            try:
                lease.ledger({'event': 'start', 'label': 'synthetic', 'pid': live.pid})
                with self.assertRaises(lease.Refused):
                    lease.release()
            finally:
                live.kill()
                live.wait()
            with holder(lease.GROUP_LOCK), self.assertRaises(lease.Refused):
                lease.release()
            lease.RELEASE.write_text('{}\n')
            with self.assertRaises(lease.Refused):
                lease.release()
            lease.RELEASE.unlink()
            self.assertEqual(self.events(), ['start'])
            self.assertFalse(lease.RELEASED.exists())
            lease.release()  # the group is gone now
            self.assertEqual(self.events(), ['start', 'release'])

    def test_regression_cases_match_their_pinned_sources(self):
        value = regression.manifest()  # the real case list and sources, read only
        self.assertEqual(sum(case['frames'] for case in value['cases']), value['total_frames'])
        self.assertLessEqual(value['total_frames'], lease.CEILINGS['regression'])
        self.assertEqual((value['totals'], value['total_frames']), ({'existing': 37, 'new': 2}, 39))
        ignored = re.compile(r'#\[ignore = "requires a native GPU[^"]*"\]\s*fn (\w+)\(')
        for path in value['sources']:
            listed = {case['test'].rsplit('::', 1)[-1]: case['coverage'] for case in value['cases']
                      if case['source'] == path}
            self.assertEqual(sorted(ignored.findall((lease.ROOT / path).read_text(encoding='utf-8'))),
                             sorted(listed), path)
            # `existing` means present in the historical base; `new` means added here.
            historical = subprocess.run(['git', 'show', f'{value["historical_base"]}:{path}'], cwd=lease.ROOT,
                                        capture_output=True, text=True, check=True, env=lease.child_env(None)).stdout
            before = set(ignored.findall(historical))
            for name, coverage in listed.items():
                self.assertEqual(coverage == 'existing', name in before, name)
        # A changed source, total, repeated case or empty case is refused.
        copy = self.dir / 'regression.json'
        saved = regression.MANIFEST
        try:
            regression.MANIFEST = copy
            for number, mutate in enumerate((
                    lambda v: v['sources'].update({next(iter(v['sources'])): '0' * 64}),
                    lambda v: v.update(total_frames=v['total_frames'] + 1),
                    lambda v: v['cases'].append(dict(v['cases'][0])),
                    lambda v: v['cases'][0].update(frames=0),
                    lambda v: v['cases'][2].update(coverage='existing'),
                    lambda v: v['cases'][0].update(coverage='historical'))):
                changed = json.loads(json.dumps(value))
                mutate(changed)
                copy.write_text(json.dumps(changed))
                with self.subTest(mutation=number), self.assertRaises(lease.Refused):
                    regression.manifest()
        finally:
            regression.MANIFEST = saved

    def test_a_grant_must_name_the_reserved_cpu_target(self):
        # Configuration only: the target directory itself is never touched here.
        self.assertEqual(lease.TARGET, Path('/tmp/incant-agent-font-target'))
        grant = self.dir / 'review-ready.json'
        with patched(GRANT=grant):
            grant.write_text(json.dumps({'packet': lease.PACKET, 'target': '/tmp/incant-agent-font-target'}))
            self.assertEqual(lease.grant()['target'], str(lease.TARGET))
            for value in ({'packet': lease.PACKET, 'target': '/Users/andreyyurlov/.codex/incant-core-presentation-target'},
                          {'packet': '0072', 'target': str(lease.TARGET)}, {'packet': lease.PACKET}):
                grant.write_text(json.dumps(value))
                with self.subTest(grant=value), self.assertRaises(lease.Refused):
                    lease.grant()

    def test_libtest_outcomes_need_exactly_one_passing_test(self):
        ok = 'running 1 test\ntest a ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out\n'
        self.assertTrue(regression.outcome(ok)['passed'])
        for text in (ok.replace('1 passed; 0 failed', '0 passed; 1 failed').replace('result: ok', 'result: FAILED'),
                     'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out\n',
                     ok + ok, 'no summary at all', ok.replace('running 1 test', 'running 2 tests')):
            with self.subTest(text=text):
                self.assertFalse(regression.outcome(text)['passed'])

    def test_porcelain_status_keeps_every_path_rename_and_deletion(self):
        repo = self.dir / 'repo'
        repo.mkdir()
        git(repo, 'init', '-q')
        for name in ('a-first.txt', 'b.txt', 'd.txt', 'with space.txt'):
            (repo / name).write_text(name)
        git(repo, 'add', '.')
        git(repo, 'commit', '-q', '-m', 'synthetic')
        (repo / 'a-first.txt').write_text('changed')  # first row: " M a-first.txt"
        git(repo, 'mv', 'b.txt', 'c.txt')
        (repo / 'd.txt').unlink()
        (repo / 'with space.txt').write_text('changed too')
        (repo / 'new.txt').write_text('untracked')
        entries = lease.status_entries(repo)
        digest = lambda name: lease.digest(repo / name)
        self.assertEqual(entries, {
            'a-first.txt': {'status': ' M', 'from': None, 'sha256': digest('a-first.txt')},
            'c.txt': {'status': 'R ', 'from': 'b.txt', 'sha256': digest('c.txt')},
            'd.txt': {'status': ' D', 'from': None, 'sha256': None},
            'with space.txt': {'status': ' M', 'from': None, 'sha256': digest('with space.txt')},
            'new.txt': {'status': '??', 'from': None, 'sha256': digest('new.txt')},
        })
        identity = lease.source_identity(repo)
        self.assertEqual(set(identity['uncommitted']), set(entries))
        self.assertEqual(len(identity['head']), 40)

    def test_grant_identities_must_match_before_acquisition(self):
        (self.dir / 'tool.py').write_text('reviewed')
        release = self.dir / 'released.json'
        release.write_text('{}')
        good = {'sources': [{'path': 'tool.py', 'sha256': lease.digest(self.dir / 'tool.py')}],
                'predecessor_release': {'path': str(release), 'sha256': lease.digest(release)}}
        with patched(ROOT=self.dir):
            lease.reviewed_identities(good)
            lease.reviewed_identities({})
            (self.dir / 'tool.py').write_text('changed after review')
            with self.assertRaises(lease.Refused):
                lease.reviewed_identities(good)
            release.write_text('{"changed": true}')
            with self.assertRaises(lease.Refused):
                lease.reviewed_identities(dict(good, sources=[]))

    def test_freeze_generations_bind_to_the_state_and_step_only_explicitly(self):
        state = {'source': {'head': 'a' * 40}, 'tools': {'result_hud.rs': '1' * 64}}
        with self.scratch_lease(), patched(identity=lambda: json.loads(json.dumps(state))):
            first = lease.freeze()
            self.assertEqual(first['generation'], 1)
            self.assertEqual(lease.freeze()['sha256'], first['sha256'])
            state['tools']['result_hud.rs'] = '2' * 64
            with self.assertRaises(lease.Refused):
                lease.freeze()
            second = lease.freeze(new=True)
            self.assertEqual(second['generation'], 2)
            recorded = json.loads((lease.FREEZES / 'generation-2.json').read_text())
            self.assertEqual(recorded['previous_sha256'], first['sha256'])
            self.assertEqual(lease.freeze()['sha256'], second['sha256'])

    def test_child_environment_is_constructed_and_only_allowlisted_values_are_recorded(self):
        os.environ['INCANT_0069_SYNTHETIC_SECRET'] = 'must-not-pass'
        try:
            env = lease.child_env(self.dir, {'CARGO_TARGET_DIR': '/synthetic/target'})
        finally:
            del os.environ['INCANT_0069_SYNTHETIC_SECRET']
        self.assertEqual(set(env), {'PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'PYTHONUTF8', 'TMPDIR',
                                    'CARGO_TARGET_DIR'})
        self.assertEqual(env['PATH'], ':'.join(lease.CHILD_PATH))
        with self.assertRaises(lease.Refused):
            lease.child_env(self.dir, {'RUSTFLAGS': '-C debuginfo=0'})
        self.assertEqual(lease.recorded_env({'CARGO_TARGET_DIR': 't', 'PATH': 'p', 'HOME': 'h'}),
                         {'CARGO_TARGET_DIR': 't'})


if __name__ == '__main__':
    unittest.main()
