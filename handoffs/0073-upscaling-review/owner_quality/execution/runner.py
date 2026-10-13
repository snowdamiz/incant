#!/usr/bin/env python3
"""Bounded frozen quality fixture orchestration; check is CPU-only, run needs root grant.
No build, enumeration, retry, image interpretation or implicit GPU authority.
The immutable owner lifecycle helper supplies process-group cleanup and signal handling.
"""
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import struct
import sys
import zlib

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve()
PACKET = HERE.parents[1]
LIFECYCLE = PACKET.parent / 'owner/owner_runner.py'
LIFECYCLE_SHA = 'b9e12895389a046e1c0a0bd4c43f01a215505c793be4106b31fafc18aa2aff34'
if hashlib.sha256(LIFECYCLE.read_bytes()).hexdigest() != LIFECYCLE_SHA:
    raise SystemExit('refused: reviewed lifecycle changed')
spec = importlib.util.spec_from_file_location('quality_lifecycle', LIFECYCLE)
life = importlib.util.module_from_spec(spec)
spec.loader.exec_module(life)
QUALITY = PACKET / 'plan.json'
QUALITY_SHA = 'ff0f0f488faa3f6d4d6cc1321d5c4e87c35cbb4160130dddd050697bb824dbc4'
FIXTURE = PACKET / 'review/upscaling_owner_review.rs'
FIXTURE_SHA = '650316a21f30bfb4f7cbe8c99b67f9b5be269b51eb62b4d017b519bd5130d997'
PRODUCTION = 'fd5b86d03a2cabfcc9bd5a8441a137645fb858ae'
PRODUCTION_FREEZE_SHA = '4143417b02a286cb9417c75df2791c34801d43fd2cedc7a37cffb9e1ce6d77c5'
E = Path.home() / '.codex/incant-evidence/0073-owner-quality-gpu-001'
GLOBAL_LOCK = E.parent / 'native-gpu.lock'
ORDER = ['stationary', 'motion', 'transitions', 'c7_disocclusion', 'p1_crossing']
SELECTIONS = [ORDER]
TIMEOUT = 90
ENV = {'PATH': '/usr/bin:/bin:/usr/sbin:/sbin', 'TMPDIR': '<fresh evidence root>/tmp/<group>',
       'INCANT_EXPECT_GPU_BACKEND': 'Metal', 'RUST_BACKTRACE': '0',
       'INCANT_0073_OWNER_REVIEW_OUT': '<fresh evidence root>/<group>/artifacts'}
PLAN_KEYS = {'schema', 'state', 'producer', 'evidence_dir', 'groups', 'budget', 'environment',
             'quality_plan', 'fixture', 'production_freeze', 'build', 'enumeration', 'build_review', 'binary'}
GRANT_KEYS = {'schema', 'authority', 'holder', 'issued_at', 'plan_sha256', 'runner_sha256',
              'lifecycle_sha256', 'binary_sha256', 'evidence_dir', 'groups', 'budget',
              'predecessor_release_path', 'predecessor_release_sha256'}


def require(ok, message):
    if not ok:
        raise life.Refused('refused: ' + message)


def bounded(path, maximum):
    p = Path(path)
    require(p.is_file() and not p.is_symlink() and p.stat().st_size <= maximum, 'bounded regular file ' + str(p))
    return p.read_bytes()


def json_file(path, maximum=4_000_000):
    def invalid(value):
        raise life.Refused('refused: nonfinite JSON ' + value)
    def pairs(rows):
        out = {}
        for key, value in rows:
            require(key not in out, 'duplicate JSON key')
            out[key] = value
        return out
    return json.loads(bounded(path, maximum), parse_constant=invalid, object_pairs_hook=pairs)


def identity(row):
    require(isinstance(row, dict) and set(row) == {'path', 'sha256'}, 'identity fields')
    p = Path(row['path'])
    require(p.is_absolute() and p.is_file() and not p.is_symlink(), 'regular absolute identity')
    require(life.HEX64.fullmatch(row['sha256']) and life.sha(p) == row['sha256'], 'identity hash')
    return p


def quality():
    require(life.sha(QUALITY) == QUALITY_SHA and life.sha(FIXTURE) == FIXTURE_SHA, 'reviewed quality plan/fixture changed')
    q = json_file(QUALITY)
    require(q['run']['order'] == ORDER and q['grant'] is None, 'quality preparation contract')
    return q


def budget(q, groups):
    total = {key: 0 for key in q['groups'][groups[0]]['budget']}
    for group in groups:
        for key, value in q['groups'][group]['budget'].items():
            total[key] += value
    return {'processes': len(groups), **total, 'maximum_extent': [960, 540],
            'timeout_seconds_each': TIMEOUT, 'builds': 0, 'retries': 0}


def read_plan(path, allow_unbuilt=False):
    p = Path(path)
    require(p.is_absolute() and not p.is_symlink(), 'absolute regular plan')
    data = json_file(p)
    require(set(data) == PLAN_KEYS and data['schema'] == 'incant-owner-quality-execution-v1', 'plan schema/fields')
    require(data['state'] in ('AWAITING_ROOT_BINARY', 'FROZEN_READY_FOR_ROOT_GRANT'), 'plan state')
    require(data['producer'] == PRODUCTION and data['evidence_dir'] == str(E), 'producer/evidence scope')
    q = quality()
    require(data['groups'] in SELECTIONS, 'primary-first closed group selection')
    require(data['budget'] == budget(q, data['groups']) and data['environment'] == ENV, 'exact budget/environment')
    for role, expected in [('quality_plan', QUALITY_SHA), ('fixture', FIXTURE_SHA), ('production_freeze', PRODUCTION_FREEZE_SHA)]:
        identity(data[role])
        require(data[role]['sha256'] == expected, role + ' identity')
    if data['state'] == 'AWAITING_ROOT_BINARY':
        require(allow_unbuilt and all(data[k] is None for k in ['binary', 'build', 'enumeration', 'build_review']),
                'unbuilt preparation cannot execute')
        return data, q
    for role in ['build', 'enumeration', 'build_review']:
        identity(data[role])
    lines = Path(data['enumeration']['path']).read_text().splitlines()
    require(all(lines.count(name + ': test') == 1 for name in ORDER), 'five exact names enumerated once')
    row = data['binary']
    build = json_file(data['build']['path'])
    require(build['schema'] == 'incant-owner-quality-build-v1'
            and build['source_commit'] == PRODUCTION
            and build['production_freeze_sha256'] == PRODUCTION_FREEZE_SHA
            and build['fixture_sha256'] == FIXTURE_SHA and build['plan_sha256'] == QUALITY_SHA
            and build['features'] == ['default', 'physics-deterministic']
            and build['executable'] == row['path'] and build['executable_sha256'] == row['sha256']
            and build['verified_production_files'] == 2835 and build['gpu_executions'] == 0,
            'actual build/source/default-feature binding')
    require([r['name'] for r in build['checks']] == ['build', 'clippy', 'listing']
            and all(r['exit_code'] == 0 for r in build['checks']), 'three successful build checks')
    require(build['checks'][0]['argv'] == q['build']['review']
            and build['checks'][1]['argv'] == q['build']['lint']
            and build['checks'][2]['argv'] == [row['path'], '--list', '--ignored'], 'exact build/check commands')
    for check in build['checks']:
        log = Path(data['build']['path']).parent / (check['name'] + '.log')
        require(life.sha(log) == check['log_sha256'], 'build raw log binding')
    require(data['enumeration']['sha256'] == build['checks'][2]['log_sha256'], 'enumeration belongs to build')
    require(isinstance(row, dict) and set(row) == {'path', 'sha256', 'bytes', 'mode'} and row['mode'] == '0555', 'binary fields/mode')
    bp = Path(row['path'])
    require(bp.is_absolute() and E.parent in bp.parents and not any('target' in part for part in bp.parts), 'outside-target copied binary')
    life.check_binary(row)
    return data, q


def grant(plan_path, plan):
    require(plan['state'] == 'FROZEN_READY_FOR_ROOT_GRANT', 'binary not frozen')
    gp = E / 'grant.json'
    g = json_file(gp)
    require(set(g) == GRANT_KEYS and g['schema'] == 'incant-owner-quality-grant-v1', 'grant schema/fields')
    require(g['authority'] == 'root' and g['holder'] == 'Claude0073 Opus5.5 Max', 'root authority and exact holder')
    require(isinstance(g['issued_at'], str) and g['issued_at'].startswith('20'), 'grant issue time')
    require(g['plan_sha256'] == life.sha(plan_path) and g['runner_sha256'] == life.sha(HERE)
            and g['lifecycle_sha256'] == LIFECYCLE_SHA, 'grant source binding')
    require(g['binary_sha256'] == plan['binary']['sha256'] and g['evidence_dir'] == str(E)
            and g['groups'] == plan['groups'] and g['budget'] == plan['budget'], 'grant bounds')
    prior = life.check_predecessor(g['predecessor_release_path'], g['predecessor_release_sha256'])
    return g, prior


def png_identity(path):
    """Byte/hash, CRC and dimensions only; never decodes or judges pixels."""
    raw = bounded(path, 2_100_000)
    require(raw[:8] == b'\x89PNG\r\n\x1a\n', 'PNG signature')
    offset, chunks, idat = 8, [], False
    while offset < len(raw):
        require(offset + 12 <= len(raw), 'truncated PNG chunk')
        n = struct.unpack('>I', raw[offset:offset+4])[0]
        end = offset + 12 + n
        require(end <= len(raw), 'PNG chunk bounds')
        kind = raw[offset+4:offset+8]
        payload = raw[offset+8:offset+8+n]
        require(zlib.crc32(kind + payload) & 0xffffffff == struct.unpack('>I', raw[end-4:end])[0], 'PNG CRC')
        if not chunks:
            require(kind == b'IHDR' and payload == struct.pack('>IIBBBBB', 960, 540, 8, 6, 0, 0, 0), 'RGBA8 960x540 PNG')
        else:
            require(kind != b'IHDR', 'duplicate PNG header')
        idat = idat or kind == b'IDAT'
        chunks.append(kind)
        offset = end
        if kind == b'IEND':
            require(n == 0 and offset == len(raw), 'PNG terminal chunk')
            break
    require(idat and chunks[-1] == b'IEND', 'PNG complete image')
    return {'path': str(path), 'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw), 'width': 960, 'height': 540}


def expected_checks(group):
    body = FIXTURE.read_text().split('fn ' + group + '() {', 1)[1].split('let checks = [', 1)[1].split('];', 1)[0]
    names = re.findall(r'\(\s*"([a-z0-9_]+)"\s*,', body) + ['budget_exactly_consumed']
    require(len(names) == len(set(names)), 'unambiguous frozen check names')
    return set(names)


def artifacts(directory, group, spec):
    root = directory / 'artifacts'
    require(root.is_dir() and not root.is_symlink(), 'artifact directory')
    names = spec['pngs']
    require(set(os.listdir(root)) == {'report.json', 'events.jsonl', *names}, 'exact artifact inventory')
    report = json_file(root / 'report.json')
    require(report['case'] == group and report['outcome'] == 'pass', 'report case/outcome')
    checks = report['checks']
    require(set(checks) == expected_checks(group) and all(value is True for value in checks.values()), 'complete typed contract checks')
    require(report['budget_limit'] == spec['budget'] == report['budget_used'], 'exact consumed budget')
    adapter = report['adapter']
    require(isinstance(adapter, dict) and adapter.get('backend') == 'Metal' and bool(adapter.get('name')), 'actual Metal adapter')
    require([row['name'] for row in report['pngs']] == names, 'ordered PNG inventory')
    pngs = []
    for row in report['pngs']:
        require(set(row) == {'name', 'sha256', 'rgba_sha256'} and life.HEX64.fullmatch(row['rgba_sha256']), 'PNG report identity')
        found = png_identity(root / row['name'])
        require(found['sha256'] == row['sha256'], 'PNG reported hash')
        pngs.append({**found, 'reported_rgba_sha256': row['rgba_sha256']})
    events_path = root / 'events.jsonl'
    lines = bounded(events_path, 4_000_000).decode().splitlines()
    require(len(lines) <= 1000, 'bounded event count')
    events = [json.loads(line) for line in lines]
    require(events[0] == {'event': 'open', 'case': group, 'budget': spec['budget']}
            and events[-1] == {'event': 'finish', 'pass': True}, 'complete event stream')
    require(all(e.get('event') in ['open', 'charge', 'shot', 'png', 'finish'] for e in events), 'known event kinds')
    require(sum(e.get('event') == 'open' for e in events) == 1 and sum(e.get('event') == 'finish' for e in events) == 1, 'unique open/finish')
    charges = {k: [] for k in ['Scene', 'Ui', 'Readback']}
    for event in events:
        if event.get('event') == 'charge':
            require(set(event) == {'event', 'name', 'kind', 'index'} and event['kind'] in charges, 'charge fields')
            charges[event['kind']].append(event)
    for kind, key in [('Scene', 'scene_submissions'), ('Ui', 'ui_submissions'), ('Readback', 'readback_submissions')]:
        require([e['index'] for e in charges[kind]] == list(range(1, spec['budget'][key] + 1)), 'charge counts/order')
    shots = [event for event in events if event['event'] == 'shot']
    shot_names = [event['name'] for event in shots]
    require(len(set(shot_names)) == len(shot_names) == spec['budget']['scene_submissions'], 'unique complete shots')
    require(shot_names == [e['name'] for e in charges['Scene']] == [e['name'] for e in charges['Readback']], 'shot/charge identity')
    require([e['name'] for e in shots if e['ui']] == [e['name'] for e in charges['Ui']], 'UI charge identity')
    require(all(life.HEX64.fullmatch(e['rgba_sha256']) for e in shots), 'shot readback hashes')
    require([e['png'] for e in events if e['event'] == 'png'] == report['pngs'], 'PNG events match report')
    return {'adapter': adapter, 'pngs': pngs, 'report_sha256': life.sha(root / 'report.json'),
            'events_sha256': life.sha(events_path), 'events': len(events), 'shots': len(shots),
            'contract_checks': checks, 'budget': spec['budget'],
            'scope': 'process/artifact/contracts only; observations do not imply historical-gate or appearance acceptance'}


def child(row, group, spec, env, directory):
    directory.mkdir()
    out, err = directory / 'stdout.txt', directory / 'stderr.txt'
    argv = [row['path'], '--ignored', '--exact', group, '--nocapture', '--test-threads=1']
    with open(out, 'x') as stdout, open(err, 'x') as stderr:
        record, caught = life.launch(argv, directory, env, stdout, stderr, TIMEOUT)
    result = {**record, 'group': group, 'argv': argv, 'cwd': str(directory),
              'environment': life.env_receipt(env), 'accepted': False}
    try:
        stdout = bounded(out, 4_000_000).decode(errors='replace')
        stderr = bounded(err, 4_000_000).decode(errors='replace')
        summaries = life.RESULT.findall(stdout)
        checks = {'successful_process': record.get('exit') == 0 and record.get('reaped')
                  and record.get('group_empty_after') and not record.get('timed_out')
                  and not record.get('interrupted_by') and 'exception' not in record and 'launch_error' not in record,
                  'one_test': 'running 1 test' in stdout.splitlines() and len(summaries) == 1
                  and summaries[0][:5] == ('ok', '1', '0', '0', '0'),
                  'no_panic': 'panicked at' not in stderr and '\nfailures:\n' not in stdout,
                  'binary_unchanged': life.sha(row['path']) == row['sha256'],
                  'no_survivors': not life.owned_processes(row['path'], record['pgid'])}
        result['checks'] = checks
        if all(checks.values()):
            result['artifacts'] = artifacts(directory, group, spec)
            result['adapter'] = result['artifacts']['adapter']
            result['accepted'] = True
    except BaseException as error:
        result['analysis_error'] = repr(error)
        caught = caught or error
    result.update(stdout_sha256=life.sha(out), stderr_sha256=life.sha(err))
    return result, caught


def execute(plan_path):
    require((E / 'grant.json').is_file(), 'no root grant; preparation only')
    plan, q = read_plan(plan_path)
    g, prior = grant(plan_path, plan)
    with open(E / 'owner.lock', 'a') as lock, open(GLOBAL_LOCK, 'a') as global_lock:
        for f in [lock, global_lock]:
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise life.Refused('refused: exclusive GPU lock already held') from None
        require(set(os.listdir(E)) <= {'grant.json', 'owner.lock'}, 'evidence not fresh; no retry')
        row = plan['binary']
        require(not life.owned_processes(row['path']), 'frozen binary already running')
        rows, caught, accepted = [], None, False
        with life.blocked():
            try:
                (E / 'tmp').mkdir()
                for group in plan['groups']:
                    life.check_binary(row)
                    spec = q['groups'][group]
                    tmp = E / 'tmp' / group
                    tmp.mkdir()
                    env = life.child_env({k: v for k, v in ENV.items() if k != 'INCANT_0073_OWNER_REVIEW_OUT'}, tmp)
                    env['INCANT_0073_OWNER_REVIEW_OUT'] = str(E / group / 'artifacts')
                    life.append_synced(E / 'debits.jsonl', {'at': life.now(), 'group': group,
                        'child_attempts': 1, **spec['budget'], 'grant_sha256': life.sha(E / 'grant.json'), 'refundable': False})
                    result, failure = child(row, group, spec, env, E / group)
                    rows.append(result)
                    if len(rows) > 1 and result.get('adapter') != rows[0].get('adapter'):
                        result['accepted'] = False
                        result['adapter_mismatch'] = True
                    life.append_synced(E / 'ledger.jsonl', result)
                    if failure:
                        raise failure
                    if not result['accepted']:
                        break
                accepted = len(rows) == len(plan['groups']) and all(r['accepted'] for r in rows)
            except BaseException as error:
                caught = error
            finally:
                survivors = life.owned_processes(row['path'])
                groups = [r['pgid'] for r in rows if r.get('pgid') and life.group_alive(r['pgid'])]
                clean = not survivors and not groups and all(r['reaped'] for r in rows)
                recorded_pngs = [p for r in rows for p in r.get('artifacts', {}).get('pngs', [])]
                actual_images = life.images_under(E)
                images_match = set(actual_images) == {p['path'] for p in recorded_pngs}
                release = {'released': clean, 'at': life.now(), 'accepted': accepted and images_match,
                    'plan_sha256': life.sha(plan_path), 'runner_sha256': life.sha(HERE), 'lifecycle_sha256': LIFECYCLE_SHA,
                    'grant_sha256': life.sha(E / 'grant.json'), 'predecessor': prior, 'groups': plan['groups'],
                    'budget': plan['budget'], 'children': rows, 'owned_processes': survivors,
                    'process_groups_alive': groups, 'images': recorded_pngs, 'actual_image_paths': actual_images,
                    'image_inventory_matches': images_match, 'exception': repr(caught) if caught else None,
                    'debits_sha256': life.sha(E / 'debits.jsonl') if (E / 'debits.jsonl').exists() else None,
                    'ledger_sha256': life.sha(E / 'ledger.jsonl') if (E / 'ledger.jsonl').exists() else None,
                    'scope': 'contract and artifact completion only; appearance remains Claude review', 'os': {'uname': list(os.uname())}}
                if clean:
                    release['released_at'] = release['at']
                life.write_new_json(E / ('release.json' if clean else 'release-blocked.json'), release)
        if caught:
            raise caught
        require(clean, 'owned process survived; release blocked')
        return release


def main():
    for sig in [signal.SIGTERM, signal.SIGHUP]:
        signal.signal(sig, life.raise_interrupt)
    require(len(sys.argv) == 3 and sys.argv[1] in ['check', 'run'], 'usage: runner.py check|run /absolute/plan.json')
    path = Path(sys.argv[2])
    if sys.argv[1] == 'check':
        plan, _ = read_plan(path, allow_unbuilt=True)
        g = grant(path, plan)[0] if (E / 'grant.json').exists() else 'absent; no GPU authority'
        print(json.dumps({'plan_sha256': life.sha(path), 'runner_sha256': life.sha(HERE), 'state': plan['state'], 'grant': g}))
        return 0
    result = execute(path)
    print(json.dumps(result, indent=2))
    return 0 if result['accepted'] else 1


if __name__ == '__main__':
    sys.exit(main())
