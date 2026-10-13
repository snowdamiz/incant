from pathlib import Path
import datetime, hashlib, json, os, shutil, subprocess, time

E = Path(__file__).resolve().parent
F = json.loads((E / 'freeze.json').read_text())
R = Path(F['checkout'])
env = os.environ.copy()
env.update(CARGO_TARGET_DIR=F['target'], CARGO_BUILD_JOBS='2', CARGO_NET_OFFLINE='true')
env['PATH'] = '/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:' + env['PATH']
results = []
binaries = []

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def unchanged():
    for row in F['files']:
        assert digest(R / row['path']) == row['sha256'], row['path']

def run(name, args):
    unchanged()
    stdout, stderr = E / (name + '.stdout'), E / (name + '.stderr')
    start = time.monotonic()
    with stdout.open('xb') as out, stderr.open('xb') as err:
        child = subprocess.Popen(args, cwd=R, env=env, stdout=out, stderr=err)
        try:
            code = child.wait()
        except BaseException:
            child.terminate()
            try:
                child.wait(timeout=10)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait()
            raise
    result = dict(name=name, argv=args, cwd=str(R), pid=child.pid, exit=code,
                  reaped=True, seconds=time.monotonic() - start,
                  stdout=dict(path=str(stdout), sha256=digest(stdout)),
                  stderr=dict(path=str(stderr), sha256=digest(stderr)))
    results.append(result)
    (E / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    print(name, code, flush=True)
    unchanged()
    assert code == 0, result
    return stdout

def messages(path):
    for line in path.read_text().splitlines():
        if line.startswith('{'):
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                pass

def copy_binaries(name, log):
    for item in messages(log):
        if item.get('reason') == 'compiler-artifact' and item.get('executable') and item['target']['name'] == 'trusted_manifolds':
            p = Path(item['executable'])
            dest = E / 'binaries' / (name + '-' + p.name)
            dest.parent.mkdir(exist_ok=True)
            shutil.copy2(p, dest)
            binaries.append(dict(path=str(dest), original=str(p), sha256=digest(dest), bytes=dest.stat().st_size, target=item['target'], profile=item['profile']))
    (E / 'binaries.json').write_text(json.dumps(binaries, indent=2) + '\n')

unchanged()
run('scoped-clean', ['./tools/cargo','clean','-p','incant_physics_snapshot','-p','rapier2d','-p','rapier3d'])
base=['./tools/cargo']
common=['--locked','--offline','--profile','ci','-j2','-p','incant_physics_snapshot']
for label, flags in [('default',[]), ('fast8',['--no-default-features','--features','physics-fast-simd8'])]:
    log=run(label+'-tests', base+['test',*common,*flags,'--all-targets','--message-format=json','--','--nocapture'])
    copy_binaries(label,log)
    run(label+'-clippy', base+['clippy',*common,*flags,'--all-targets','--','-D','warnings'])
for label, flags in [('default',['--features','rapier2d/parallel,rapier3d/parallel']),('fast8',['--no-default-features','--features','physics-fast-simd8,rapier2d/parallel,rapier3d/parallel'])]:
    run(label+'-two-worker-world',['env','RAYON_NUM_THREADS=2']+base+['test',*common,*flags,'--test','trusted_manifolds','--message-format=json','world_boundary_','--','--nocapture','--test-threads=1'])
run('wasm',base+['check',*common,'--lib','--target','wasm32-unknown-unknown'])
run('normal-graph',base+['tree','--locked','--offline','-p','incant_physics_snapshot','--edges','normal','--prefix','none'])
run('runtime-guard',['python3','tools/check_runtime_dependencies.py','incant_physics_snapshot'])
run('fmt',base+['fmt','--all','--','--check'])
run('conventions',['python3','tools/generate_conventions.py','--check'])
run('diff',['git','diff','--check','d62976cc5adc740b5b0541abcbfd59d4069b4a45','HEAD'])
unchanged()
print('complete',datetime.datetime.now(datetime.UTC).isoformat(),flush=True)
