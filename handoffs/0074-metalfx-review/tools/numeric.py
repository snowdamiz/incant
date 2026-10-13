"""One precompiled, bounded MetalFX ownership test. No builds or images."""
from pathlib import Path
import datetime
import fcntl
import hashlib
import importlib.util
import json
import os
import platform
import signal
import sys

sys.dont_write_bytecode = True
R = Path(__file__).resolve().parents[3]
E = Path.home() / '.codex/incant-evidence/0074-metalfx-numeric'
TEST = 'models::retained::metalfx_tests::actual_native_batch_and_fallback_preserve_retained_submission_ownership'
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
read = lambda p: json.loads(Path(p).read_text())
now = lambda: datetime.datetime.now(datetime.timezone.utc).isoformat()


def write_new(path, value):
    with path.open('x') as out:
        json.dump(value, out, indent=2)
        out.write('\n')
        out.flush()
        os.fsync(out.fileno())


def append(value):
    with (E / 'ledger.jsonl').open('a') as out:
        out.write(json.dumps({'at': now(), **value}) + '\n')
        out.flush()
        os.fsync(out.fileno())


def main():
    grant = read(E / 'review-ready.json')
    assert grant['packet'] == '0074-metalfx-numeric' and sha(__file__) == grant['runner_sha256']
    assert not (E / 'attempt.json').exists(), 'one-shot grant already consumed'
    assert sha(grant['freeze']['path']) == grant['freeze']['sha256']
    freeze = read(grant['freeze']['path'])

    def source():
        for item in freeze['sources'] + freeze['unchanged_baselines']:
            assert sha(R / item['path']) == item['sha256'], item['path']
        assert sha(R / 'docs/spikes/evidence/native-metalfx.json') == grant['manifest_sha256']

    source()
    binary = grant['binary']
    assert sha(binary['path']) == binary['sha256'] and os.access(binary['path'], os.X_OK)
    assert Path(binary['path']).read_bytes().count(TEST.encode()) == 1
    previous_path = Path(grant['predecessor']['path'])
    assert sha(previous_path) == grant['predecessor']['sha256']
    previous = read(previous_path)
    assert not previous['owned_processes'] and not previous['ledger_pids_alive']
    assert sha(previous_path.parent / 'gpu-ledger.jsonl') == previous['ledger_sha256']
    helper = Path(grant['lifecycle']['path'])
    assert sha(helper) == grant['lifecycle']['sha256']
    spec = importlib.util.spec_from_file_location('owned_lifecycle', helper)
    lifecycle = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(lifecycle)
    old_rows = [json.loads(line) for line in (previous_path.parent / 'gpu-ledger.jsonl').read_text().splitlines()]
    assert len(old_rows) == previous['children']
    for row in old_rows:
        assert lifecycle.identity(row['pid']) is None and not lifecycle.group_pids(row['pid'])
    if '--preflight' in sys.argv:
        print('Source, manifest, binary, exact test and predecessor release verified; no GPU execution.')
        return
    lifecycle.install_signal_handlers()
    fd = os.open(E.parent / 'native-gpu.lock', os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        write_new(E / 'attempt.json', {'at': now(), 'runner_sha256': sha(__file__), 'frames_reserved': 6,
                                     'command_buffers_reserved': 16, 'extent': [64, 64], 'test': TEST})
        outcome = {}
        error = None
        stdout, stderr = E / 'native.stdout', E / 'native.stderr'
        (E / 'tmp').mkdir()
        env = {'PATH': '/usr/bin:/bin:/usr/sbin:/sbin', 'HOME': str(Path.home()),
               'TMPDIR': str(E / 'tmp'), 'RUST_BACKTRACE': '1'}
        try:
            lifecycle.run([binary['path'], TEST, '--ignored', '--exact', '--nocapture', '--test-threads=1'],
                          env=env, cwd=R, stdout_path=stdout, stderr_path=stderr, timeout_s=60,
                          inherit_fds=(fd,), outcome=outcome,
                          on_spawn=lambda pid, pgid, ident: append({'event': 'spawn', 'pid': pid, 'pgid': pgid, 'identity': ident}),
                          on_member=lambda ident: append({'event': 'member', 'identity': ident}))
            assert outcome['exit'] == 0 and outcome['reaped'] and outcome['group_absent']
            assert not outcome['timed_out'] and not outcome['survivors']
            text = stdout.read_text()
            assert '1 passed; 0 failed; 0 ignored' in text
            assert 'backend: Metal' in text and 'admission: Some(Ok(()))' in text
            assert 'native submissions=5; portable submissions=1; rejected submissions=1' in text
            assert sha(binary['path']) == binary['sha256']
            source()
        except BaseException as exc:
            error = f'{type(exc).__name__}: {exc}'
            raise
        finally:
            logs = {p.name: {'path': str(p), 'sha256': sha(p), 'bytes': p.stat().st_size}
                    for p in (stdout, stderr) if p.exists()}
            append({'event': 'terminal', 'outcome': outcome, 'error': error, 'logs': logs,
                    'binary_sha256': binary['sha256'], 'source_commit': grant['source_commit']})
            live = lifecycle.still_alive(outcome.get('members_seen', []))
            write_new(E / 'gpu-released.json', {'packet': grant['packet'], 'at': now(), 'outcome': outcome,
                       'error': error, 'owned_processes': live, 'ledger_sha256': sha(E / 'ledger.jsonl'),
                       'attempt_sha256': sha(E / 'attempt.json'), 'system': platform.platform(),
                       'success': error is None, 'scope': 'six tiny ownership/fallback/reset frames; no pixel or performance acceptance'})
            assert not live and outcome.get('group_absent') is True
    finally:
        os.close(fd)
    print('Exact MetalFX ownership regression passed; GPU released.')


if __name__ == '__main__':
    main()
