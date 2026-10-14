#!/usr/bin/env python3
"""Root-authorized frozen R4 scalar tests; no build, retry or images."""
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import sys

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
E = HERE.parent
LIFECYCLE_SHA = 'b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34'
assert hashlib.sha256((HERE / 'lifecycle.py').read_bytes()).hexdigest() == LIFECYCLE_SHA
spec = importlib.util.spec_from_file_location('lifecycle', HERE / 'lifecycle.py')
life = importlib.util.module_from_spec(spec)
spec.loader.exec_module(life)
ORDINARY = 'upscaling::system::device_tests::actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay'
OWNER = 'upscaling::system::owner_device_tests::actual_owner_history_obeys_provenance_and_preserves_ordinary_fallback'
MARKER = ('OWNER_HISTORY cases=12 owner_renders=12 current_only_renders=1 baseline_renders=2 '
          'queue_submits=12 readback_maps=12 max_extent=16x16 no_images=true')
JOBS = [('ordinary-default', 0, ORDINARY, 19), ('owner-default', 0, OWNER, 12),
        ('ordinary-cooked', 1, ORDINARY, 19), ('owner-cooked', 1, OWNER, 12)]
EXPECTED = ['103027db6cba5333af940fba57f29678b339680ea0a757ab39fff140c7155ebc',
            '7a629aca2ff959ece83b79019f88857ae45c59f72965cd870329cdbb1911f506']


def require(value, reason):
    if not value:
        raise RuntimeError(reason)


def check():
    plan = json.loads((HERE / 'plan.json').read_text())
    grant = json.loads((E / 'grant.json').read_text())
    require(grant['plan_sha256'] == life.sha(HERE / 'plan.json'), 'grant plan identity')
    require(grant['runner_sha256'] == life.sha(__file__), 'grant runner identity')
    require(plan['submissions'] == plan['maps'] == 62 and plan['attempts'] == 4, 'budget')
    require(plan['timeout_each'] == 90 and plan['max_extent'] == [16, 16], 'bounds')
    require(plan['images'] == plan['builds'] == plan['retries'] == 0, 'scope')
    require([r['sha256'] for r in plan['binaries']] == EXPECTED, 'binary identities')
    for row in plan['binaries']:
        life.check_binary(row)
    for row in plan['identities']:
        require(life.sha(row['path']) == row['sha256'], 'source/review identity')
    return plan


def judge(stdout, stderr, record, test):
    lines = [line.removeprefix('test ' + test + ' ... ') for line in stdout.splitlines()]
    results = life.RESULT.findall(stdout)
    checks = dict(exit=record.get('exit') == 0, reaped=record.get('reaped') is True,
                  empty_group=record.get('group_empty_after') is True,
                  uninterrupted=not record.get('timed_out') and not record.get('interrupted_by'),
                  no_exception='exception' not in record and 'launch_error' not in record,
                  one_test='running 1 test' in lines and len(results) == 1
                  and results[0][:5] == ('ok', '1', '0', '0', '0'),
                  no_panic='panicked at' not in stderr)
    prefix = 'upscaling adapter: ' if test == ORDINARY else 'OWNER_HISTORY adapter='
    adapters = [line[len(prefix):] for line in lines if line.startswith(prefix)]
    checks['metal'] = len(adapters) == 1 and 'backend: Metal' in adapters[0]
    if test == ORDINARY:
        observed = [re.fullmatch(r'upscaling mode(\d+) attachment(\d+): \[.*\]', line)
                    for line in lines if line.startswith('upscaling mode')]
        checks['full_sequence'] = (len(observed) == 38 and all(observed)
            and [(int(m[1]), int(m[2])) for m in observed if m] == [(m, a) for m in range(19) for a in range(2)])
    else:
        observed = [re.fullmatch(r'OWNER_HISTORY case=(\d+) center=.* probes=(\d+) baseline=(true|false) current_only=(true|false)', line)
                    for line in lines if line.startswith('OWNER_HISTORY case=')]
        checks['full_sequence'] = len(observed) == 12 and all(observed) and [int(m[1]) for m in observed if m] == list(range(1, 13))
        checks['baselines'] = all(observed) and [int(m[1]) for m in observed if m and m[3] == 'true'] == [7, 8]
        checks['current_only'] = all(observed) and [int(m[1]) for m in observed if m and m[4] == 'true'] == [3]
        checks['marker'] = lines.count(MARKER) == 1
    return checks, adapters[0] if len(adapters) == 1 else None


def execute():
    plan = check()
    with open(E.parent / 'native-gpu.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require(set(os.listdir(E)) <= {'grant.json', 'preparation', 'supervisor.log', 'supervisor.stderr'}, 'no retry')
        for binary in plan['binaries']:
            require(not life.owned_processes(binary['path']), 'binary already live')
        (E / 'tmp').mkdir()
        env = life.child_env({'PATH': '/usr/bin:/bin:/usr/sbin:/sbin', 'RUST_BACKTRACE': '0',
                              'TMPDIR': '<fresh evidence root>/tmp',
                              'INCANT_EXPECT_GPU_BACKEND': 'Metal'}, E / 'tmp')
        rows, caught = [], None
        with life.blocked():
            try:
                for name, index, test, limit in JOBS:
                    row = plan['binaries'][index]
                    life.check_binary(row)
                    directory = E / name
                    directory.mkdir()
                    life.append_synced(E / 'debits.jsonl', dict(job=name, attempts=1, submits=limit, maps=limit, refundable=False))
                    argv = [row['path'], '--ignored', '--exact', test, '--nocapture', '--test-threads=1']
                    with open(directory / 'stdout.txt', 'x') as out, open(directory / 'stderr.txt', 'x') as err:
                        record, failure = life.launch(argv, directory, env, out, err, 90)
                    result = dict(record, job=name, argv=argv, environment=life.env_receipt(env), accepted=False)
                    rows.append(result)
                    checks, adapter = judge((directory / 'stdout.txt').read_text(), (directory / 'stderr.txt').read_text(), record, test)
                    checks['unchanged'] = life.sha(row['path']) == row['sha256']
                    checks['same_adapter'] = len(rows) == 1 or adapter == rows[0].get('adapter')
                    checks['no_images'] = not life.images_under(directory)
                    result.update(checks=checks, adapter=adapter, accepted=all(checks.values()),
                                  stdout_sha256=life.sha(directory / 'stdout.txt'), stderr_sha256=life.sha(directory / 'stderr.txt'))
                    life.append_synced(E / 'ledger.jsonl', result)
                    if failure:
                        raise failure
                    if not result['accepted']:
                        break
            except BaseException as error:
                caught = error
            finally:
                survivors = sum((life.owned_processes(b['path']) for b in plan['binaries']), [])
                groups = [r['pgid'] for r in rows if r.get('pgid') and life.group_alive(r['pgid'])]
                clean = not survivors and not groups and all(r.get('reaped') for r in rows)
                result = dict(released=clean, released_at=life.now() if clean else None,
                              accepted=len(rows) == 4 and all(r['accepted'] for r in rows), children=rows,
                              owned_processes=survivors, process_groups_alive=groups, images=life.images_under(E),
                              exception=repr(caught) if caught else None, runner_sha256=life.sha(__file__),
                              plan_sha256=life.sha(HERE / 'plan.json'), grant_sha256=life.sha(E / 'grant.json'),
                              lifecycle_sha256=LIFECYCLE_SHA, os=dict(uname=list(os.uname())))
                result['accepted'] &= clean and not result['images'] and not caught
                life.write_new_json(E / ('release.json' if clean else 'release-blocked.json'), result)
        if caught:
            raise caught
        print(json.dumps(result, indent=2))
        return 0 if result['accepted'] else 1


if __name__ == '__main__':
    for sig in (signal.SIGTERM, signal.SIGHUP):
        signal.signal(sig, life.raise_interrupt)
    require(__debug__, 'Python optimization forbidden')
    if sys.argv[1:] == ['check']:
        print(json.dumps(check(), indent=2))
    elif sys.argv[1:] == ['run']:
        raise SystemExit(execute())
    else:
        raise SystemExit('use check|run')
