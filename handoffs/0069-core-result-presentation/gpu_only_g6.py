#!/usr/bin/env python3
"""Fresh 0069 GPU authority: g6 test oracle, unchanged g5 production packages.

`check` only hashes. `run` requires root's separate authority for 0069-gpu-003.
The original wrapper/helpers/receipts remain unchanged. Regression receipts use
actual g6 identity; capture receipts retain actual g5 package identity, after
checking current source against the sole reviewed cfg(test) change. No build,
recook, target access, automatic retry or GPU authority creation is exposed.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

import gpu_only as old

lease, capture, regression = old.lease, old.capture, old.regression
HERE, ROOT, CPU = old.HERE, old.ROOT, old.CPU
read, require = old.read, old.require
CORRECTION = Path('/Users/andreyyurlov/.codex/incant-evidence/0069-oracle-correction-002')
EVIDENCE = CPU.parent / '0069-gpu-003'
SOURCE = 'e193af4df19ca4762441aec5957a9a40357e6437'
PHASE = 'gpu-only-generation-6-oracle'
TEST_SOURCE = 'crates/incant_render/src/ui_draw_tests.rs'
TEST_SHA = '9b51e6448f4203902baa6985fc79b7c2b3d8181ee1cab85394c9fdbac795ddf0'
FREEZE_SHA = '255c7f15d28268e80c494102fc1a1e5c5dde2105551241a0451e76d57da90793'
BUILD_SHA = 'c9c125462437aa1d1f212d4c00538867bd7c84eeca48ffd6d2ecc1e84348b12e'
CASES_SHA = 'fffb7857e5c9dd3e9c4d21fc9ca0bdc09319ac93fd042db40152f3340a0b01d0'
OLD_WRAPPER_SHA = '0cf379927af6ace42f8ef5d76b5249f455aa759690e0cdffa00911710bd77914'


def source_inputs(new=False):
    require(not new, 'GPU execution cannot create a source generation')
    require(lease.digest(old.__file__) == OLD_WRAPPER_SHA, 'original wrapper changed')
    require(lease.digest(old.MANIFEST) == old.MANIFEST_SHA, 'original CPU manifest changed')
    m = read(old.MANIFEST)
    require(lease.digest(CPU / 'freeze/generation-5.json') == old.FREEZE_SHA, 'g5 freeze changed')
    require(lease.digest(CORRECTION / 'freeze-g6.json') == FREEZE_SHA, 'g6 freeze changed')
    inventory = Path(m['source_freeze']['path'])
    require(lease.digest(inventory) == m['source_freeze']['sha256'], 'original source inventory changed')
    require(read(inventory)['head'] == old.SOURCE, 'original source head changed')
    for row in read(inventory)['files']:
        p = ROOT / row['path']
        if p == HERE / 'brief.md':  # instructions bound separately to fresh authority
            continue
        expected = TEST_SHA if row['path'] == TEST_SOURCE else row['sha256']
        require(p.is_file() and lease.digest(p) == expected, f'frozen source changed: {row["path"]}')
    for row in read(CORRECTION / 'freeze-g6.json')['engine_inputs']:
        require(lease.digest(ROOT / row['path']) == row['sha256'], f'g6 input changed: {row["path"]}')
    for path in lease.status_entries(ROOT):
        require(path.startswith('handoffs/0069-core-result-presentation/'), f'extra dirty source: {path}')
    git = lambda args: subprocess.run(['git', *args], cwd=ROOT, env={'PATH': '/usr/bin:/bin'},
                                     capture_output=True, text=True, check=True).stdout
    git(['merge-base', '--is-ancestor', SOURCE, 'HEAD'])
    require(not git(['diff', '--name-only', SOURCE, 'HEAD', '--', '.',
                     ':(exclude)handoffs/0069-core-result-presentation/**']),
            'committed source differs from reviewed correction')
    frozen5 = read(CPU / 'freeze/generation-5.json')
    current = lease.identity()
    current['source'] = frozen5['identity']['source']
    require(current == frozen5['identity'], 'original helpers, lock, host or toolchain changed')
    return {'generation': 6, 'sha256': FREEZE_SHA, 'identity': frozen5['identity']}


def inputs():
    frozen = source_inputs()
    m = read(old.MANIFEST)
    release = CPU / 'release.json'
    require(lease.digest(release) == read(CPU / 'released')['release_sha256'], 'CPU release latch changed')
    require(lease.digest(CPU / 'ledger.jsonl') == read(release)['ledger_sha256'], 'CPU ledger changed')
    require(read(release) == m['release'], 'CPU release differs from reviewed manifest')
    require(lease.digest(CORRECTION / 'frozen-build-g6.json') == BUILD_SHA, 'g6 build changed')
    require(lease.digest(CORRECTION / 'regression-g6.json') == CASES_SHA, 'g6 case manifest changed')
    built = read(CORRECTION / 'frozen-build-g6.json')
    require(built['freeze'] == {'generation': 6, 'sha256': FREEZE_SHA}, 'g6 build/source mismatch')
    cases = read(CORRECTION / 'regression-g6.json')
    original = read(HERE / 'tools/regression.json')
    original['sources'][TEST_SOURCE] = TEST_SHA
    require(cases == original, 'regression case or budget changed')
    require(built['manifest_sha256'] == CASES_SHA and built['sources'] == cases['sources'], 'build case mismatch')
    old_build = read(CPU / 'regression/frozen-build-g5.json')
    require(old_build == m['regression_compile'], 'original build record changed')
    for name, row in built['executables'].items():
        p = Path(row['path'])
        if name == 'render-lib':
            require(p == CORRECTION / 'frozen-binaries/generation-6/render-lib', 'g6 test must use frozen copy')
        else:
            require(name == 'gpu-host-ui' and all(row[k] == v for k,v in old_build['executables'][name].items()),
                    'unchanged host executable differs from g5')
        require(lease.digest(p) == row['sha256'] and os.access(p, os.X_OK), 'test copy changed or non-executable')
    require(set(built['executables']) == {'render-lib', 'gpu-host-ui'}, 'executable list changed')
    plans = read(capture.PLANS)
    require(len(cases['cases']) == 8 and cases['total_frames'] == 39, 'regression budget changed')
    require(len(plans['runs']) == 10 and sum(s['max_frames'] for s in plans['runs'].values()) == 22,
            'appearance budget changed')
    for variant in {s['variant'] for s in plans['runs'].values()}:
        package = CPU / 'packages' / variant
        record = read(package / 'package.json')
        require(record['freeze'] == {'generation': 5, 'sha256': old.FREEZE_SHA}, 'package is not original g5')
        require(capture.package_inventory(package) == record['inventory'], f'{variant}: package changed')
        require(os.access(package / 'adapter' / capture.EXAMPLE, os.X_OK), 'adapter non-executable')
    require({s['route'] for s in plans['runs'].values()} == set(old.ROUTES), 'route list changed')
    for name,digest in old.ROUTES.items():
        require(lease.digest(CPU / 'shipping-002' / (name+'-input.json')) == digest, 'accepted route changed')
    return frozen, plans, built


def authority(path, evidence):
    require(path.is_absolute() and path.is_file() and not path.is_symlink(), 'missing absolute root authority')
    require(evidence == EVIDENCE and not evidence.is_symlink(), 'only fresh 0069-gpu-003 is admitted')
    require(not path.is_relative_to(evidence), 'authority must be outside GPU output')
    value = read(path)
    expected = {'packet':lease.PACKET,'phase':PHASE,'gpu_only':True,'cpu_only':False,
                'source_commit':SOURCE,'freeze_sha256':FREEZE_SHA,'build_sha256':BUILD_SHA,
                'case_manifest_sha256':CASES_SHA,'package_freeze_sha256':old.FREEZE_SHA,
                'cpu_manifest_sha256':old.MANIFEST_SHA,'wrapper_sha256':lease.digest(__file__),
                'original_wrapper_sha256':OLD_WRAPPER_SHA,'evidence':str(evidence),
                'brief_sha256':lease.digest(HERE/'brief.md'),'target':str(evidence/'unused-cargo-target'),
                'ceilings':lease.CEILINGS,'planned_processes':18,
                'cpu_release_sha256':lease.digest(CPU/'release.json'),
                'correction_release_sha256':lease.digest(CORRECTION/'cpu-release.json')}
    for key,wanted in expected.items():
        require(type(value.get(key)) is type(wanted) and value[key] == wanted, f'authority mismatch: {key}')
    require(isinstance(value.get('holder'),str) and bool(value['holder'].strip()), 'authority lacks actual holder')
    predecessor = value.get('predecessor_release')
    require(isinstance(predecessor,dict), 'root must bind prior GPU release')
    require(lease.digest(predecessor['path']) == predecessor['sha256'], 'prior GPU release changed')
    return value


def seed(evidence, plans):
    old.seed(evidence, plans)  # preserves exact g5 freeze/packages/build/routes
    shutil.copy2(CORRECTION/'freeze-g6.json', evidence/'freeze/generation-6.json')
    shutil.copy2(CORRECTION/'frozen-build-g6.json', evidence/'regression/build-g6.json')
    shutil.copy2(CORRECTION/'regression-g6.json', evidence/'regression/manifest-g6.json')


def configure(evidence, grant_path, plans, built):
    for attr, relative in {'EVIDENCE':'.','LEASE':'lease.json','RELEASED':'released','RELEASE':'release.json',
                           'LEDGER':'ledger.jsonl','FRAMES':'frames.json','GROUP_LOCK':'group.lock','LOGS':'logs',
                           'SCRATCH':'scratch','FREEZES':'freeze','TARGET':'unused-cargo-target'}.items():
        setattr(lease,attr,evidence/relative)
    lease.GRANT = grant_path
    original_guard, original_run = lease.guard, lease.run
    def guard():
        authority(grant_path,evidence)
        return original_guard()
    allowed = {}
    for case in read(CORRECTION/'regression-g6.json')['cases']:
        name = f'regression-g6-{case["id"]}'
        exe = built['executables'][case['binary']]
        allowed[name] = ([exe['path'],'--exact',case['test'],'--ignored','--test-threads=1','--nocapture'],
                         ('regression',name,case['frames']), ROOT/'crates'/exe['package'])
    for name,spec in plans['runs'].items():
        package = evidence/'packages'/spec['variant']
        allowed['adapter-'+name] = ([str(package/'adapter'/capture.EXAMPLE),str(package/'player/data'),
                                     str(evidence/'routes'/(spec['route']+'.json')),
                                     str(evidence/'plans'/(name+'.json')),str(evidence/'captures'/name)],
                                    ('appearance',name,spec['max_frames']),evidence/'scratch'/name)
    def run(argv, *, label, timeout, env, overrides=None, cwd=ROOT, debit=None, gpu=False):
        require(gpu is True and label in allowed, 'unplanned or CPU child')
        expected, expected_debit, expected_cwd = allowed[label]
        require(list(map(str,argv)) == expected and debit == expected_debit and Path(cwd) == expected_cwd,
                'child argv, cwd or debit changed')
        require(timeout == 120, 'GPU child timeout must be120 seconds')
        require(set(env) <= {'PATH','HOME','TMPDIR','INCANT_UI_EVIDENCE','INCANT_HDR_EVIDENCE'}, 'extra child environment')
        require(env.get('PATH') == '/usr/bin:/bin' and Path(env['TMPDIR']).resolve().is_relative_to((evidence/'scratch').resolve()),
                'unexpected child environment path')
        clean = {k:v for k,v in env.items() if k != 'HOME'}
        for key in ('INCANT_UI_EVIDENCE','INCANT_HDR_EVIDENCE'):
            if key in clean: require(clean[key] == str(evidence/'regression/images-g6'), 'unexpected image output')
        return original_run(argv,label=label,timeout=timeout,env=clean,overrides=overrides,cwd=cwd,debit=debit,gpu=True)
    def freeze(new=False):
        guard()
        return source_inputs(new)
    lease.guard, lease.freeze, lease.run = guard, freeze, run
    capture.CAPTURES, capture.GPU = evidence/'captures', evidence/'gpu.json'
    regression.MANIFEST = evidence/'regression/manifest-g6.json'
    regression.OUT, regression.RESULTS = evidence/'regression', evidence/'regression/results.jsonl'


def capture_original(name, spec):
    """Check current g6 source, then let the original capture verify genuine g5 artifacts."""
    original = lease.freeze
    def package_freeze(new=False):
        original(new)  # same source/authority checks, never bypassed for appearance
        frozen = read(CPU/'freeze/generation-5.json')
        return {'generation':5,'sha256':old.FREEZE_SHA,'identity':frozen['identity']}
    lease.freeze = package_freeze
    try:
        return capture.run(name,spec)
    finally:
        lease.freeze = original


def execute(evidence, grant_path):
    frozen,plans,built = inputs()
    authority(grant_path,evidence)
    require(not evidence.exists(), 'output already exists; no retries')
    seed(evidence,plans)
    configure(evidence,grant_path,plans,built)
    acquired, failure = False, None
    try:
        lease.acquire(); acquired = True
        require(all(r['passed'] for r in regression.run_all()), 'one or more exact GPU regressions failed')
        for name,spec in plans['runs'].items(): capture_original(name,spec)
        require(inputs()[0] == frozen, 'source/package changed during GPU execution')
        state = lease.frames_state()
        require(state['debited'] == {'appearance':22,'regression':39} and len(state['runs']) == 18,
                'unexpected final GPU accounting')
    except BaseException as error:
        failure = error
        raise
    finally:
        if acquired or lease.LEASE.is_file():
            try: lease.release()
            except BaseException as cleanup:
                if failure is None: raise
                failure.add_note(f'GPU lease release also failed: {cleanup!r}; authority remains held')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=('check','run'))
    parser.add_argument('--authority',type=Path)
    parser.add_argument('--evidence',type=Path,default=EVIDENCE)
    args = parser.parse_args()
    if args.action == 'check':
        inputs()
        print(json.dumps({'passed':True,'generation':6,'package_generation':5,'gpu_children_launched':0,
                          'planned_children':18,'regression_frames':39,'appearance_frames':22}))
    else:
        require(args.authority is not None, 'run needs explicit authority')
        execute(args.evidence,args.authority)

if __name__ == '__main__':main()
