"""Replay bounded execution metadata and PNG identities without decoding pixels."""
from pathlib import Path
import fcntl
import hashlib
import importlib.util
import json
import os
import sys

sys.dont_write_bytecode = True
S = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-evidence/0073-r4-quality-gpu-001')
P = Path('/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/r4_quality')
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
runner = P / 'execution/runner.py'
assert sha(runner) == '2a991b3abde5fb2b5e5ddcc28eac6187f88e0f5fa4b1c7dbe235eae112b8af6c'
spec = importlib.util.spec_from_file_location('r4_quality_execution', runner)
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)
plan_path = P / 'execution/plan-ready.json'
plan, quality = r.read_plan(plan_path)
grant, prior = r.grant(plan_path, plan)
release = r.json_file(E / 'release.json')
ledger = [json.loads(x) for x in (E / 'ledger.jsonl').read_text().splitlines()]
debits = [json.loads(x) for x in (E / 'debits.jsonl').read_text().splitlines()]
assert release['released'] and release['accepted'] and release['exception'] is None
assert release['children'] == ledger and len(ledger) == len(debits) == 5
assert [x['group'] for x in ledger] == [x['group'] for x in debits] == r.ORDER
assert release['budget'] == plan['budget'] and release['predecessor'] == prior
assert release['plan_sha256'] == sha(plan_path) and release['runner_sha256'] == sha(runner)
assert release['grant_sha256'] == sha(E / 'grant.json')
assert release['ledger_sha256'] == sha(E / 'ledger.jsonl') and release['debits_sha256'] == sha(E / 'debits.jsonl')
images = []
for row, debit in zip(ledger, debits):
    group = row['group']
    assert row['exit'] == 0 and row['accepted'] and row['reaped'] and row['group_empty_after']
    assert not row['timed_out'] and not row['interrupted_by'] and all(row['checks'].values())
    assert row['argv'] == [plan['binary']['path'], '--ignored', '--exact', group, '--nocapture', '--test-threads=1']
    stdout = (E / group / 'stdout.txt').read_text()
    stderr = (E / group / 'stderr.txt').read_text()
    assert sha(E / group / 'stdout.txt') == row['stdout_sha256']
    assert sha(E / group / 'stderr.txt') == row['stderr_sha256']
    summaries = r.life.RESULT.findall(stdout)
    assert 'running 1 test' in stdout.splitlines() and len(summaries) == 1
    assert summaries[0][:5] == ('ok', '1', '0', '0', '0')
    assert 'panicked at' not in stderr and '\nfailures:\n' not in stdout
    for call, key in [(os.kill, 'pid'), (os.killpg, 'pgid')]:
        try:
            call(row[key], 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError(('live child/group', key, row[key]))
    expected = quality['groups'][group]['budget']
    for key, value in expected.items():
        assert debit[key] == value
    assert debit['child_attempts'] == 1 and debit['refundable'] is False and debit['grant_sha256'] == sha(E / 'grant.json')
    replay = r.artifacts(E / group, group, quality['groups'][group])
    assert replay == row['artifacts']
    assert row['adapter'] == ledger[0]['adapter'] == replay['adapter']
    images.extend(replay['pngs'])
assert images == release['images'] and len(images) == 15
assert set(release['actual_image_paths']) == {x['path'] for x in images}
assert sum(x['queue_submissions'] for x in debits) == 778
assert sum(x['readback_maps'] for x in debits) == 380
assert not release['owned_processes'] and not release['process_groups_alive']
assert not r.life.owned_processes(plan['binary']['path'])
with open(E.parent / 'native-gpu.lock', 'a') as held:
    fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
    fcntl.flock(held, fcntl.LOCK_UN)
files = [p for p in E.rglob('*') if p.is_file() and p.name != 'owner.lock']
result = {
    'scope': 'Independent nonvisual execution integrity only; appearance is Claude judgment',
    'release_sha256': sha(E / 'release.json'), 'plan_sha256': sha(plan_path), 'runner_sha256': sha(runner),
    'grant_sha256': sha(E / 'grant.json'), 'binary_sha256': plan['binary']['sha256'],
    'groups': r.ORDER, 'queue_submissions': 778, 'maps': 380, 'pngs': images,
    'adapter': ledger[0]['adapter'], 'all_children_reaped': True, 'global_lock_available': True,
    'files': [{'path': str(p), 'bytes': p.stat().st_size, 'sha256': sha(p)} for p in sorted(files)],
}
(S / 'execution-audit.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'accepted_execution_integrity': True, 'files': len(files), 'pngs': len(images), 'queue_submissions': 778, 'maps': 380}))
