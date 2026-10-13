"""Read-only verification of the isolated journal replay packet."""
import argparse, base64, gzip, hashlib, json, re, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
R=Path(__file__).resolve().parents[4]
M=json.loads((R/'docs/spikes/evidence/journal-replay-main224.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def file(row):
 b=(R/row['path']).read_bytes();assert h(b)==row['sha256'] and len(b)==row['bytes'];return b
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
a=json.loads(gzip.decompress(file(M['archive'])))
assert len(a['records'])==M['archive']['records'] and len(a['payloads'])==M['archive']['payloads']
payloads={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
for k,b in payloads.items():assert h(b)==k
raw={}
for row in a['records']:
 b=payloads[row['sha256']];assert len(b)==row['bytes'];assert Path(row['path']).read_bytes()==b;raw[row['path']]=b
freeze=json.loads(file(M['freeze']));assert len(freeze['files'])==3143
for row in freeze['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']
source=M['source_file'];assert h((R/source).read_bytes())==M['source_sha256']
assert (R/source).read_bytes()==git('show',M['source_head']+':'+source)
assert git('diff','--name-only',M['base'],M['source_head']).decode().splitlines()==[source]
patch=gzip.decompress(file(M['patch']));assert h(patch)==M['patch']['raw_sha256'] and len(patch)==M['patch']['raw_bytes']
assert git('diff',M['base'],M['source_head'],'--',source)==patch
# Exact-filter child execution remains mandatory; no unsafe diagnostic helper ships.
s=(R/source).read_text();assert 'unsafe' not in s and 'diagnostic_' not in s
assert 'stdout.contains("test result: ok. 1 passed; 0 failed;")' in s
assert s.count('matches!(actual, Some(CommandError::AgentBudgetUserOnly))')==2
assert s.count('assert_eq!(std::fs::read(&path).unwrap(), journal);')==2
assert len(M['checks'])==15
for c in M['checks']:
 assert c['exit']==0 and c['reaped']
 for key in ['stdout','stderr']:assert h(raw[c[key]['path']])==c[key]['sha256']
def counts(text):
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 return len(rows),tuple(sum(int(row[i]) for row in rows) for i in range(3))
commands={c['name']:c for c in M['checks']}
assert counts(raw[commands['command-tests']['stdout']['path']].decode())==(18,(54,0,0))
for i in range(10):
 c=commands['concurrent-replay-'+str(i)];assert '--test-threads=6' in c['argv']
 assert counts(raw[c['stdout']['path']].decode())==(1,(6,0,0))
assert len(M['binaries'])==1
for b in M['binaries']:
 data=Path(b['path']).read_bytes();assert h(data)==b['sha256'] and len(data)==b['bytes']
diag=M['controlled_diagnostic'];log=raw[diag['log']].decode()
assert 'before_release=Some(Io(Custom { kind: Other, error: "WouldBlock" }))' in log
assert 'after_release=Some(AgentBudgetUserOnly)' in log and counts(log)==(1,(1,0,0))
dsrc=raw[diag['source']].decode();assert 'downcast_ref::<std::fs::TryLockError>()' in dsrc
assert 'Some(std::fs::TryLockError::WouldBlock)' in dsrc
for stage in ['004','005']:
 name=next(k for k in raw if k.endswith('replay-budget-'+stage+'/focused.log'))
 assert 'test result: FAILED.' in raw[name].decode()
initial=[b.decode() for path,b in raw.items() if '/replay-budget-001/repeat-' in path and path.endswith('.log')]
assert len(initial)==63 and all(counts(s)==(1,(6,0,0)) for s in initial)
hosted=raw[M['hosted_failure']['raw_log']];assert h(hosted)==M['hosted_failure']['sha256']
assert b"replay_refuses_nonuser_budget_commands_that_do_not_change_the_projection'" in hosted
assert b'Err(CommandError::AgentBudgetUserOnly)' in hosted
print(json.dumps({'verified':True,'records':len(a['records']),'payloads':len(payloads),'freeze':3143,'checks':15,'cpu_cases':54,'concurrent_library_repeats':10,'controlled_lock_mechanism':True,'hosted_cause':'inferred','production_changes':False}))
