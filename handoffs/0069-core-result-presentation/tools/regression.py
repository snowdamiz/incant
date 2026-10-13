#!/usr/bin/env python3
"""UI GPU regression cases for 0069, each case once. DRAFT for root review.

  regression.py build    build the two pinned test binaries (cargo test --no-run; CPU only)
  regression.py run      run every case in regression.json once, in order

regression.json pins both test sources by SHA-256 and lists every ignored UI GPU
case with a frame count derived from that source: the seven existing historical
tests (present at `historical_base`, main196) and one case of newly added halo
coverage, labelled `coverage: new`; the totals include both. Both commands refuse
unless the sources still hash as pinned and the latest freeze generation still
holds, so the counts describe the code that runs. `build` runs
`./tools/cargo test --no-run` for each binary through lease.cargo and records the
executables, their digests and the freeze generation in
regression/build-g{generation}.json (create-only).

`run` re-checks the sources, the freeze generation and the executable digests
before every case, then runs it through lease.run as a GPU child: lease.run takes
the global native GPU lock and then the packet lock, refusing at once if either
is busy, debits the case's frames to the `regression` ceiling (128; the 24
appearance frames are a separate ceiling), launches
`EXE --exact TEST --ignored --test-threads=1 --nocapture` from the package
directory, as `cargo test` would, in a constructed environment (scratch HOME and
TMPDIR, so no cache is written elsewhere) bounded to 120 s, kills and reaps its
group and records its exit before releasing either lock. A case's run id is
`regression-g{generation}-{id}` and is debited once, so no case runs twice in one
generation and nothing is retried. A failing test is recorded and the next case
runs; a timeout, an unreaped group, an interruption or a refusal stops the run.
Each outcome is read from libtest's own summary and must show exactly one test
run. Test images go to regression/images-g{generation} through INCANT_UI_EVIDENCE
and INCANT_HDR_EVIDENCE.
"""
import json
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import lease  # noqa: E402

MANIFEST = HERE / 'regression.json'
OUT = lease.EVIDENCE / 'regression'
RESULTS = OUT / 'results.jsonl'
SUMMARY = re.compile(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;',
                     re.MULTILINE)


def manifest():
    """The pinned case list, refusing if a source no longer matches its counts."""
    value = json.loads(MANIFEST.read_text(encoding='utf-8'))
    cases = value['cases']
    total = sum(case['frames'] for case in cases)
    totals = {coverage: sum(case['frames'] for case in cases if case.get('coverage') == coverage)
              for coverage in ('existing', 'new')}
    if (value['ceiling'] != lease.CEILINGS['regression'] or total != value['total_frames']
            or total > value['ceiling'] or totals != value['totals']):
        raise lease.Refused('regression.json frame totals disagree with its cases or lease.py')
    if len({case['id'] for case in cases}) != len(cases):
        raise lease.Refused('regression.json repeats a case id')
    for case in cases:
        if (case['source'] not in value['sources'] or case['binary'] not in value['binaries']
                or case.get('coverage') not in ('existing', 'new')
                or type(case['frames']) is not int or case['frames'] < 1):
            raise lease.Refused(f'{case["id"]}: malformed case')
    for path, sha256 in value['sources'].items():
        if lease.digest(lease.ROOT / path) != sha256:
            raise lease.Refused(f'{path} differs from the source its frame counts were derived from')
    return value


def build():
    value = manifest()
    frozen = lease.freeze()
    record_path = OUT / f'build-g{frozen["generation"]}.json'
    if record_path.exists():
        raise lease.Refused(f'{record_path.name} exists; this generation is already built')
    executables = {}
    for name, binary in value['binaries'].items():
        args = ['--offline', '--profile', 'ci', *binary['cargo'], '--no-run', '--message-format=json']
        status, stdout, stderr = lease.cargo('test', args, label=f'cargo-test-{name}')
        if status != 0:
            raise lease.Refused(f'{name}: build failed:\n' + stderr.read_text(encoding='utf-8', errors='replace')[-6000:])
        messages = [json.loads(line) for line in stdout.read_text(encoding='utf-8').splitlines()
                    if line.startswith('{')]
        artifact = binary['artifact']
        found = [m['executable'] for m in messages if m.get('reason') == 'compiler-artifact' and m.get('executable')
                 and m['profile']['test'] and m['target']['name'] == artifact['name']
                 and artifact['kind'] in m['target']['kind']]
        if len(found) != 1:
            raise lease.Refused(f'{name}: expected one test executable, found {found}')
        executables[name] = {'path': found[0], 'sha256': lease.digest(found[0]), 'package': binary['package']}
    after = lease.freeze()
    if (after['generation'], after['sha256']) != (frozen['generation'], frozen['sha256']):
        raise lease.Refused('the freeze generation changed during the build')
    record = {'format': 'incant-0069-regression-build', 'freeze': {'generation': frozen['generation'],
                                                                   'sha256': frozen['sha256']},
              'manifest_sha256': lease.digest(MANIFEST), 'sources': value['sources'],
              'executables': executables, 'profile': 'ci', 'cargo_target_dir': str(lease.TARGET)}
    OUT.mkdir(parents=True, exist_ok=True)
    lease.write_new(record_path, json.dumps(record, indent=1) + '\n')
    return record


def outcome(stdout):
    """libtest's own verdict; anything but exactly one test run is a failure."""
    summaries = SUMMARY.findall(stdout)
    if len(summaries) != 1:
        return {'passed': False, 'reason': f'{len(summaries)} libtest summaries'}
    verdict, passed, failed, ignored, measured = summaries[0]
    counts = {'passed': int(passed), 'failed': int(failed), 'ignored': int(ignored), 'measured': int(measured)}
    ran = 'running 1 test' in stdout
    return {'passed': verdict == 'ok' and counts['passed'] == 1 and counts['failed'] == 0 and ran,
            'libtest': counts, 'ran_one': ran}


def run_all():
    value = manifest()
    frozen = lease.freeze()
    generation = frozen['generation']
    record_path = OUT / f'build-g{generation}.json'
    if not record_path.is_file():
        raise lease.Refused('build the test binaries under the current freeze generation first')
    built = json.loads(record_path.read_text(encoding='utf-8'))
    if (built['freeze'] != {'generation': generation, 'sha256': frozen['sha256']}
            or built['manifest_sha256'] != lease.digest(MANIFEST)):
        raise lease.Refused('the test binaries were built for another freeze generation or case list')
    images = OUT / f'images-g{generation}'
    images.mkdir(parents=True, exist_ok=True)
    rows = []
    for case in value['cases']:
        manifest()
        if lease.freeze()['sha256'] != frozen['sha256']:
            raise lease.Refused('the freeze generation changed during the regression run')
        executable = built['executables'][case['binary']]
        if lease.digest(executable['path']) != executable['sha256']:
            raise lease.Refused(f'{case["id"]}: test executable differs from its build record')
        run_id = f'regression-g{generation}-{case["id"]}'
        scratch = lease.scratch(run_id)
        overrides = {'INCANT_UI_EVIDENCE': str(images), 'INCANT_HDR_EVIDENCE': str(images)}
        env = {'PATH': '/usr/bin:/bin', 'HOME': str(scratch), 'TMPDIR': str(scratch), **overrides}
        status, stdout, stderr = lease.run(
            [executable['path'], '--exact', case['test'], '--ignored', '--test-threads=1', '--nocapture'],
            label=run_id, timeout=lease.GPU_TIMEOUT, env=env, overrides=overrides,
            cwd=lease.ROOT / 'crates' / executable['package'],
            debit=('regression', run_id, case['frames']), gpu=True)
        text = stdout.read_text(encoding='utf-8', errors='replace')
        row = {'run': run_id, 'case': case['id'], 'coverage': case['coverage'], 'test': case['test'],
               'frames_debited': case['frames'],
               'status': status, **outcome(text), 'stdout': str(stdout), 'stderr': str(stderr),
               'executable_sha256': executable['sha256'], 'freeze': built['freeze']}
        row['passed'] = row['passed'] and status == 0
        with RESULTS.open('a', encoding='utf-8') as stream:
            stream.write(json.dumps(row) + '\n')
        rows.append(row)
        print(json.dumps(row))
    return rows


def main():
    args = sys.argv[1:]
    try:
        if args == ['build']:
            print(json.dumps(build(), indent=1))
        elif args == ['run']:
            rows = run_all()
            sys.exit(0 if all(row['passed'] for row in rows) else 1)
        else:
            print(__doc__)
            sys.exit(2)
    except lease.Refused as error:
        print(f'refused: {error}', file=sys.stderr)
        sys.exit(3)


if __name__ == '__main__':
    main()
