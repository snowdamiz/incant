"""Independent execution replay; PNG identities/metadata only, no pixel decoding."""
from pathlib import Path
import fcntl, hashlib, importlib.util, json, os, sys
sys.dont_write_bytecode = True
S = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-evidence/0073-r5-numeric-quality-gpu-001')
P = Path('/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review/handoffs/0073-upscaling-review/r5_quality')
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
runner = P / 'execution/runner.py'
assert sha(runner) == 'f2ba4c9695cc276b0ce7fcf2faff967ad7f1b52ca5967735ad7ce4078849d72f'
spec = importlib.util.spec_from_file_location('r5_execution', runner)
r = importlib.util.module_from_spec(spec); spec.loader.exec_module(r)
plan_path = P / 'execution/plan-ready.json'
plan, quality = r.read_plan(plan_path); grant, prior = r.grant(plan_path, plan)
release = r.json_file(E / 'release.json')
ledger = [json.loads(x) for x in (E / 'ledger.jsonl').read_text().splitlines()]
debits = [json.loads(x) for x in (E / 'debits.jsonl').read_text().splitlines()]
assert release['released'] and release['accepted'] and release['exception'] is None
assert release['children'] == ledger and len(ledger) == len(debits) == 9
jobs = list(r.jobs(plan, quality))
assert [x['group'] for x in ledger] == [x['group'] for x in debits] == [x['name'] for x in jobs]
assert release['budget'] == plan['budget'] and release['predecessor'] == prior
for key, path in [('plan_sha256',plan_path),('runner_sha256',runner),('grant_sha256',E/'grant.json'),('ledger_sha256',E/'ledger.jsonl'),('debits_sha256',E/'debits.jsonl')]:
    assert release[key] == sha(path)
images=[]
for row, debit, job in zip(ledger, debits, jobs):
    group = row['group']; binary=job['row']; test=job.get('test',group)
    assert row['exit']==0 and row['accepted'] and row['reaped'] and row['group_empty_after']
    assert not row['timed_out'] and not row['interrupted_by'] and all(row['checks'].values())
    assert row['kind']==job['kind'] and row['argv']==[binary['path'],'--ignored','--exact',test,'--nocapture','--test-threads=1']
    stdout=(E/group/'stdout.txt').read_text();stderr=(E/group/'stderr.txt').read_text()
    assert sha(E/group/'stdout.txt')==row['stdout_sha256'] and sha(E/group/'stderr.txt')==row['stderr_sha256']
    summaries=r.life.RESULT.findall(stdout)
    assert 'running 1 test' in stdout.splitlines() and len(summaries)==1 and summaries[0][:5]==('ok','1','0','0','0')
    assert 'panicked at' not in stderr and '\nfailures:\n' not in stdout
    for call,key in [(os.kill,'pid'),(os.killpg,'pgid')]:
        try:call(row[key],0)
        except ProcessLookupError:pass
        else:raise AssertionError(('live child/group',key,row[key]))
    for key,value in job['budget'].items():assert debit[key]==value
    assert debit['child_attempts']==1 and debit['refundable'] is False and debit['grant_sha256']==sha(E/'grant.json')
    if job['kind']=='numeric':
        checks,adapter=r.judge_numeric(stdout,stderr,row,test)
        assert all(checks.values()) and adapter==row['adapter'] and not r.life.images_under(E/group)
    else:
        replay=r.artifacts(E/group,group,quality['groups'][group])
        assert replay==row['artifacts'] and row['adapter']==replay['adapter'];images.extend(replay['pngs'])
    assert r.adapter_key(row['adapter'])==r.adapter_key(ledger[0]['adapter'])
assert images==release['images'] and len(images)==15
assert set(release['actual_image_paths'])=={x['path'] for x in images}==set(r.life.images_under(E))
assert sum(x['queue_submissions'] for x in debits)==840
assert sum(x['readback_maps'] for x in debits)==442
assert not release['owned_processes'] and not release['process_groups_alive']
for binary in plan['numeric_binaries']+[plan['binary']]:assert not r.life.owned_processes(binary['path'])
with open(E.parent/'native-gpu.lock','a') as held:
    fcntl.flock(held,fcntl.LOCK_EX|fcntl.LOCK_NB);fcntl.flock(held,fcntl.LOCK_UN)
files=[p for p in E.rglob('*') if p.is_file() and p.name!='owner.lock']
result=dict(scope='Execution integrity only; pixel judgment remains Claude-owned',release_sha256=sha(E/'release.json'),plan_sha256=sha(plan_path),runner_sha256=sha(runner),grant_sha256=sha(E/'grant.json'),binary_sha256=[b['sha256'] for b in plan['numeric_binaries']+[plan['binary']]],groups=[j['name'] for j in jobs],queue_submissions=840,maps=442,pngs=images,adapter=ledger[4]['adapter'],all_children_reaped=True,global_lock_available=True,files=[dict(path=str(p),bytes=p.stat().st_size,sha256=sha(p)) for p in sorted(files)])
(S/'execution-audit.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(dict(accepted_execution_integrity=True,files=len(files),pngs=len(images),children=9,queue_submissions=840,maps=442)))
