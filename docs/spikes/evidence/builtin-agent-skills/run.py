import hashlib, json, os, re, shutil, signal, subprocess, sys, time
from pathlib import Path

root = Path('/Users/andreyyurlov/.codex/worktrees/physics-failure-isolation/incant')
out = Path(__file__).parent
target = Path('/private/tmp/incant-agent-font-target')
node = '/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin/node'
sys.path.insert(0, str(root / 'tools'))
import native_process

freeze = json.loads((out / 'source.json').read_text())
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def source_check():
    assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip() == freeze['source']
    assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=root).strip()
    for row in freeze['files']:
        assert sha(root / row['path']) == row['sha256'], row['path']

for key, value in os.environ.items():
    override = key in ('RUSTC', 'RUSTDOC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'RUSTFLAGS', 'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'CARGO_ENCODED_RUSTDOCFLAGS', 'CARGO_BUILD_RUSTC', 'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER') or key.startswith(('CARGO_PROFILE_', 'CARGO_FEATURE_', 'INCANT_')) or key.endswith('_RUSTFLAGS')
    assert not (value and override), key
source_check()
def size():
    total = 0
    for parent, dirs, files in os.walk(target, followlinks=False):
        for name in files:
            try:
                p = Path(parent) / name
                if not p.is_symlink(): total += p.stat().st_size
            except FileNotFoundError: pass
    return total

initial = size()
started = time.monotonic()
last = 0
stop = []
for sig in (signal.SIGINT, signal.SIGTERM):
    signal.signal(sig, lambda number, frame: stop.append(number))
logpath = None
def bounds():
    global last
    now = time.monotonic()
    if stop: raise RuntimeError('cancelled')
    if now - started > 2400: raise RuntimeError('overall deadline')
    if now - last < 3: return
    last = now
    assert shutil.disk_usage(target).free >= 6 * 1024**3, 'free floor'
    assert size() - initial <= 4 * 1024**3, 'target growth'
    if logpath: assert logpath.stat().st_size <= 32 * 1024**2, 'log ceiling'

env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='2', RAYON_NUM_THREADS='2', CARGO_INCREMENTAL='0', CARGO_NET_OFFLINE='true', RUSTUP_TOOLCHAIN='1.99.0-aarch64-apple-darwin')
env['PATH'] = str(Path(node).parent) + os.pathsep + env.get('PATH', '')
common = ['--target', 'aarch64-apple-darwin', '--profile', 'ci', '--locked', '--offline', '-j', '2']
steps = [
    ('agent-tests', ['./tools/cargo', 'test', '-p', 'incant_agent', '--all-targets', *common, '--', '--test-threads=1'], root, 900),
    ('strict-lint', ['./tools/cargo', 'clippy', '-p', 'incant_agent', '--all-targets', *common, '--', '-D', 'warnings'], root, 900),
    ('fmt', ['./tools/cargo', 'fmt', '-p', 'incant_agent', '--', '--check'], root, 60),
    ('conventions', ['python3', 'tools/generate_conventions.py', '--check'], root, 60),
]
results = {'source': freeze['source'], 'source_manifest_sha256': sha(out / 'source.json'), 'runner_sha256': sha(Path(__file__)), 'target': str(target), 'target_initial_bytes': initial, 'screen_capture': False, 'native_window': False, 'steps': []}
for name, argv, cwd, timeout in steps:
    source_check()
    before = time.monotonic()
    logpath = out / (name + '.log')
    row = {'name': name, 'argv': argv, 'cwd': str(cwd), 'timeout': timeout}
    results['steps'].append(row)
    try:
        with logpath.open('xb') as log:
            r = native_process.run(argv, cwd=cwd, env=env, timeout=timeout, stdout=log, stderr=subprocess.STDOUT, cancel=bounds)
        row.update(exit_code=r.returncode, terminal_reaped=True)
        assert r.returncode == 0, 'first failed stage'
        if name in ('agent-tests', 'editor-agent-tests'):
            counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed;', logpath.read_text())
            assert counts and sum(int(n) for n, _ in counts) > 0, 'no tests executed'
            row['passed_tests'] = sum(int(n) for n, _ in counts)
    except BaseException as error:
        row.update(error=repr(error), terminal_reaped=True)
    row.update(elapsed_seconds=time.monotonic() - before, log_sha256=sha(logpath), log_bytes=logpath.stat().st_size)
    results.update(elapsed_seconds=time.monotonic() - started, target_growth_bytes=size() - initial)
    (out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    print(json.dumps(row), flush=True)
    if row.get('exit_code') != 0 or 'error' in row: sys.exit(1)
source_check()
results['complete'] = True
(out / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
