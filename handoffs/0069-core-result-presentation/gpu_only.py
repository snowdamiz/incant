#!/usr/bin/env python3
"""Fresh GPU-only authority for the immutable 0069 generation-5 products.

`check` only rehashes. `run` needs a new root-owned authority outside the fresh
output directory; it never clears or reopens the released CPU lease. It runs the
8 pinned regressions and all 10 pinned capture plans once, then releases on every
outcome. No Cargo, authoring, target use, window, or automatic retry is exposed.
The imported frozen helpers retain their supervision, locking and debit logic.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / 'tools'))
import lease
import capture
import regression

CPU = Path('/Users/andreyyurlov/.codex/incant-evidence/0069')
SOURCE = '15a59060c06ea249c66740b5940b92721d032970'
FREEZE_SHA = 'acb68a621b4b74acd4c4e14280171451de81f53d2feaa4f8585a9fc6d12d1f1d'
MANIFEST_SHA = '1d5f8a8922e42a1bdda534c598633f7afce1c093bc6ebc615e8d61e7b70b4d27'
MANIFEST = HERE / 'cpu-evidence/manifest.json'
PHASE = 'gpu-only-generation-5'
ROOT = lease.ROOT
ROUTES = {
    'gamepad-restart-lose': '65ba294f07c345f97c97d19ad4fed9163ecd5314c772367fb8fcfc815b399a72',
    'lose-restart-win': '97f048f0fdc59b1182884cb3028d4caa6528d834bc8d8256dc91caa63bc6c777',
    'lose-short': 'e4707335c2d5116ef604c1976dbd864eeafab305cecd583a20795dd0d2976227',
}


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def require(ok, why):
    if not ok:
        raise lease.Refused(why)


def checked_freeze(new=False):
    """Read-only content admission, not a new build or a relabelled generation.

    A later packet-only commit may change HEAD. Every originally inventoried
    source and every frozen tool must remain exact, and no extra engine path may
    be dirty. The original generation file and build receipts are never edited.
    """
    require(not new, 'GPU-only execution cannot create a source generation')
    require(lease.digest(MANIFEST) == MANIFEST_SHA, 'CPU manifest changed')
    m = read(MANIFEST)
    require(lease.digest(CPU / 'freeze/generation-5.json') == FREEZE_SHA, 'generation 5 changed')
    frozen = read(CPU / 'freeze/generation-5.json')
    source = Path(m['source_freeze']['path'])
    require(lease.digest(source) == m['source_freeze']['sha256'], 'source inventory changed')
    inventory = read(source)
    require(inventory['head'] == SOURCE, 'source inventory head changed')
    for row in inventory['files']:
        p = ROOT / row['path']
        # Root's execution instructions are not an input to any frozen binary.
        # Keep all other original files exact; bind this one document to the
        # fresh authority instead, so scheduling cannot invalidate the build.
        if p == HERE / 'brief.md':
            continue
        require(p.is_file() and p.stat().st_size == row['bytes'] and lease.digest(p) == row['sha256'],
                f'frozen source changed: {row["path"]}')
    # New review/helper files may only be inside this packet, never engine paths.
    for path in lease.status_entries(ROOT):
        require(path.startswith('handoffs/0069-core-result-presentation/'), f'extra dirty source: {path}')
    changed = subprocess.run(['git', 'diff', '--name-only', SOURCE, 'HEAD', '--', '.',
                              ':(exclude)handoffs/0069-core-result-presentation/**'],
                             cwd=ROOT, env={'PATH': '/usr/bin:/bin'}, capture_output=True,
                             text=True, check=True).stdout
    require(not changed, 'committed engine source differs from generation 5')
    current = lease.identity()
    expected = dict(frozen['identity'])
    # Content is checked above; only packet-only Git ancestry can differ.
    current['source'] = expected['source']
    require(current == expected, 'frozen tools, lock, host or toolchain changed')
    return {'generation': 5, 'sha256': FREEZE_SHA, 'identity': expected}


def inputs():
    frozen = checked_freeze()
    m = read(MANIFEST)
    release = CPU / 'release.json'
    require(lease.digest(release) == read(CPU / 'released')['release_sha256'], 'CPU release latch changed')
    require(lease.digest(CPU / 'ledger.jsonl') == read(release)['ledger_sha256'], 'CPU ledger changed')
    require(read(release) == m['release'], 'CPU release differs from reviewed manifest')
    plans = read(capture.PLANS)
    cases = regression.manifest()
    require(len(cases['cases']) == 8 and cases['total_frames'] == 39, 'regression plan changed')
    require(len(plans['runs']) == 10 and sum(s['max_frames'] for s in plans['runs'].values()) == 22,
            'appearance plan changed')
    built = read(CPU / 'regression/frozen-build-g5.json')
    require(built == m['regression_compile'], 'frozen executable record changed')
    for row in built['executables'].values():
        p = Path(row['path'])
        require(p.parent == CPU / 'frozen-binaries/generation-5', 'test uses a live target executable')
        require(lease.digest(p) == row['sha256'] and os.access(p, os.X_OK), 'test copy changed or is not executable')
    for variant in {s['variant'] for s in plans['runs'].values()}:
        package = CPU / 'packages' / variant
        record = read(package / 'package.json')
        require(record['freeze'] == {'generation': 5, 'sha256': FREEZE_SHA}, 'package generation changed')
        require(capture.package_inventory(package) == record['inventory'], f'{variant}: package changed')
        require(os.access(package / 'adapter' / capture.EXAMPLE, os.X_OK), f'{variant}: adapter is not executable')
    require({s['route'] for s in plans['runs'].values()} == set(ROUTES), 'route list changed')
    for name, digest in ROUTES.items():
        require(lease.digest(CPU / 'shipping-002' / (name + '-input.json')) == digest, 'accepted route changed')
    return frozen, plans, built


def authority(path, evidence):
    require(path.is_absolute() and evidence.is_absolute(), 'authority and evidence paths must be absolute')
    require(path.is_file() and not path.is_symlink(), 'root authority is missing or a symlink')
    require(evidence.parent == CPU.parent and evidence != CPU and evidence.name.startswith('0069-gpu-'),
            'GPU evidence must be a fresh sibling 0069-gpu-* directory')
    require(not evidence.is_symlink() and path.parent != evidence, 'authority must be outside the GPU output')
    value = read(path)
    expected = {'packet': lease.PACKET, 'phase': PHASE, 'gpu_only': True, 'cpu_only': False,
                'source_commit': SOURCE, 'freeze_sha256': FREEZE_SHA, 'cpu_manifest_sha256': MANIFEST_SHA,
                'wrapper_sha256': lease.digest(__file__), 'evidence': str(evidence),
                'brief_sha256': lease.digest(HERE / 'brief.md'),
                'target': str(evidence / 'unused-cargo-target'), 'ceilings': lease.CEILINGS,
                'planned_processes': 18, 'cpu_release_sha256': lease.digest(CPU / 'release.json')}
    for key, wanted in expected.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, f'authority mismatch: {key}')
    require(isinstance(value.get('holder'), str) and bool(value['holder'].strip()), 'authority lacks actual holder')
    require(isinstance(value.get('predecessor_release'), dict), 'root must bind the preceding GPU release')
    predecessor = value['predecessor_release']
    require(lease.digest(predecessor['path']) == predecessor['sha256'], 'preceding GPU release changed')
    return value


def seed(evidence, plans):
    """New namespace: immutable input aliases/copies, no writes to the CPU packet."""
    evidence.mkdir()
    (evidence / 'packages').symlink_to(CPU / 'packages', target_is_directory=True)
    (evidence / 'freeze').mkdir()
    shutil.copy2(CPU / 'freeze/generation-5.json', evidence / 'freeze/generation-5.json')
    (evidence / 'regression').mkdir()
    # run_all's established filename is build-g5.json. Its exact bytes now come
    # from frozen-build-g5.json and point only to the retained executable copies.
    shutil.copy2(CPU / 'regression/frozen-build-g5.json', evidence / 'regression/build-g5.json')
    (evidence / 'routes').mkdir()
    routes = {}
    for name in {s['route'] for s in plans['runs'].values()}:
        original = CPU / 'shipping-002' / (name + '-input.json')
        destination = evidence / 'routes' / (name + '.json')
        require(lease.digest(original) == ROUTES[name], 'accepted route changed before copying')
        shutil.copy2(original, destination)
        require(lease.digest(destination) == ROUTES[name], 'route copy changed')
        routes[name] = {'sha256': lease.digest(destination), 'equals_accepted': True,
                        'accepted_path': str(original), 'accepted_sha256': lease.digest(original)}
    (evidence / 'routes/routes.json').write_text(json.dumps(routes, indent=2) + '\n')


def configure(evidence, grant_path, plans, built):
    """Redirect only runtime evidence globals of the pinned helper modules."""
    for attr, relative in {'EVIDENCE': '.', 'LEASE': 'lease.json', 'RELEASED': 'released',
                           'RELEASE': 'release.json', 'LEDGER': 'ledger.jsonl', 'FRAMES': 'frames.json',
                           'GROUP_LOCK': 'group.lock', 'LOGS': 'logs', 'SCRATCH': 'scratch',
                           'FREEZES': 'freeze', 'TARGET': 'unused-cargo-target'}.items():
        setattr(lease, attr, evidence / relative)
    lease.GRANT = grant_path
    original_guard, original_run = lease.guard, lease.run

    def guard():
        authority(grant_path, evidence)
        return original_guard()

    allowed = {}
    cases = regression.manifest()['cases']
    for case in cases:
        name = f'regression-g5-{case["id"]}'
        executable = built['executables'][case['binary']]
        allowed[name] = ([executable['path'], '--exact', case['test'], '--ignored', '--test-threads=1', '--nocapture'],
                         ('regression', name, case['frames']), ROOT / 'crates' / executable['package'])
    for name, spec in plans['runs'].items():
        package = evidence / 'packages' / spec['variant']
        allowed['adapter-' + name] = ([str(package / 'adapter' / capture.EXAMPLE), str(package / 'player/data'),
                                      str(evidence / 'routes' / (spec['route'] + '.json')),
                                      str(evidence / 'plans' / (name + '.json')), str(evidence / 'captures' / name)],
                                     ('appearance', name, spec['max_frames']), evidence / 'scratch' / name)

    def run(argv, *, label, timeout, env, overrides=None, cwd=ROOT, debit=None, gpu=False):
        require(gpu is True and label in allowed, 'GPU-only wrapper refuses an unplanned or CPU child')
        expected, expected_debit, expected_cwd = allowed[label]
        require(list(map(str, argv)) == expected and debit == expected_debit and Path(cwd) == expected_cwd,
                'child argv, cwd or debit differs from the frozen plan')
        require(timeout == 120, 'GPU child timeout must be 120 seconds')
        require(set(env) <= {'PATH', 'HOME', 'TMPDIR', 'INCANT_UI_EVIDENCE', 'INCANT_HDR_EVIDENCE'},
                'unexpected child environment key')
        require(env.get('PATH') == '/usr/bin:/bin' and Path(env['TMPDIR']).is_relative_to(evidence / 'scratch'),
                'unexpected child environment path')
        clean = {k: v for k, v in env.items() if k != 'HOME'}
        # Preserve the real user's HOME by omission; never fabricate an account.
        for key in ('INCANT_UI_EVIDENCE', 'INCANT_HDR_EVIDENCE'):
            if key in clean:
                require(clean[key] == str(evidence / 'regression/images-g5'), 'unexpected image output')
        return original_run(argv, label=label, timeout=timeout, env=clean, overrides=overrides,
                            cwd=cwd, debit=debit, gpu=True)

    def freeze(new=False):
        guard()
        return checked_freeze(new)

    lease.guard, lease.freeze, lease.run = guard, freeze, run
    capture.CAPTURES, capture.GPU = evidence / 'captures', evidence / 'gpu.json'
    regression.OUT, regression.RESULTS = evidence / 'regression', evidence / 'regression/results.jsonl'


def execute(evidence, grant_path):
    frozen, plans, built = inputs()
    authority(grant_path, evidence)
    require(not evidence.exists(), 'GPU evidence directory already exists; no retry or reopening')
    seed(evidence, plans)
    configure(evidence, grant_path, plans, built)
    acquired = False
    failure = None
    try:
        lease.acquire()
        acquired = True
        results = regression.run_all()
        require(all(row['passed'] for row in results), 'one or more exact GPU regressions failed')
        for name, spec in plans['runs'].items():
            capture.run(name, spec)
        require(inputs()[0] == frozen, 'source or packaged inputs changed during GPU execution')
        state = lease.frames_state()
        require(state['debited'] == {'appearance': 22, 'regression': 39} and len(state['runs']) == 18,
                'unexpected final GPU accounting')
    except BaseException as error:
        failure = error
        raise
    finally:
        if acquired or lease.LEASE.is_file():
            try:
                lease.release()
            except BaseException as cleanup:
                if failure is None:
                    raise
                failure.add_note(f'GPU lease release also failed: {cleanup!r}; authority remains held')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('check', 'run'))
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--authority', type=Path)
    args = parser.parse_args()
    if args.action == 'check':
        inputs()
        print(json.dumps({'passed': True, 'source': SOURCE, 'generation': 5, 'gpu_children_launched': 0,
                          'planned_children': 18, 'regression_frames': 39, 'appearance_frames': 22}))
    else:
        require(args.evidence is not None and args.authority is not None, 'run needs explicit authority and evidence')
        execute(args.evidence, args.authority)


if __name__ == '__main__':
    main()
