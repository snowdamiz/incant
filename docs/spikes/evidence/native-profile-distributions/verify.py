#!/usr/bin/env python3
"""Read-only integrity and exact outcome replay; never runs captured commands."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
R=D.parents[3]
p=argparse.ArgumentParser();p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
H=lambda b:hashlib.sha256(b).hexdigest()
m=json.loads((D/'manifest.json').read_text());archive=(D/'artifacts.json.gz').read_bytes()
assert H(archive)==m['archive_sha256']
v=json.loads(gzip.decompress(archive));payloads={h:base64.b64decode(b,validate=True) for h,b in v['payloads'].items()}
assert all(H(b)==h for h,b in payloads.items())
records={r['path']:r for r in v['records']}
assert len(records)==len(v['records'])==m['records']
assert len(payloads)==m['payloads']
def raw(name):
 r=records[name];b=payloads[r['sha256']];assert len(b)==r['bytes'];return b
for name in records: raw(name)
for source in m['sources']:
 b=raw('source-final/files/'+source['path']);assert H(b)==source['sha256']
 assert subprocess.check_output(['git','show',m['source']+':'+source['path']],cwd=R)==b
for generation in ('source-001','source-002'):
 for row in json.loads(raw(generation+'/freeze.json'))['files']:
  assert H(raw(generation+'/files/'+row['path']))==row['sha256']
 # Every production Rust byte was identical across executed generations.
 for row in json.loads(raw(generation+'/freeze.json'))['files']:
  if '/src/' in row['path'] and row['path'].endswith('.rs'):
   assert raw(generation+'/files/'+row['path'])==raw('source-final/files/'+row['path'])
for c in m['commands']:
 row=json.loads(raw(c['receipt']));assert row['exit']==c['exit']==0
 assert H(raw(c['receipt'].replace('.json','.log')))==row['log_sha256']
 if row['argv'][0]=='./tools/cargo' and row['argv'][1] in ('test','clippy','check'):
  assert all(flag in row['argv'] for flag in ('--offline','--locked','--target','--profile','ci','-j2'))
count=0
for name in ('profile-summary-001','profile-script-001','profile-player-001','profile-cli-002','profile-default-002'):
 rows=re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',raw(name+'.log'))
 assert rows and all(int(f)==int(i)==0 for _,f,i in rows)
 count+=sum(int(n) for n,_,_ in rows)
assert count==m['qualification']['named_cpu_tests']==17
cli=json.loads(raw('player-report-002.json'));profile=cli['profile'];wall=profile['frame_wall']
assert profile['version']==2 and wall['version']==1
assert wall['attempted_frames']==wall['successful_frames']==3
assert wall['failed_frames']==wall['invalid_frames']==wall['active_frames']==0 and not wall['saturated']
for name in ('player.frame','script.tick'):
 row=next(r for r in wall['phases'] if r['name']==name)
 assert row['samples']==3 and row['missing_frames']==row['invalid_frames']==row['overflow_samples']==0
 assert len(row['counts'])==10001 and sum(row['counts'])==3
 rank=(95*row['samples']+99)//100;total=0
 for i,n in enumerate(row['counts']):
  total+=n
  if total>=rank: break
 assert row['p95_upper_ns']==i*10000 and row['p95_lower_ns']==(0 if i==0 else (i-1)*10000+1)
assert H(raw('player-report-002.json'))==m['cli_report']['report_sha256']
line=next(line for line in raw('profile-cli-002.log').splitlines() if line.startswith(b'CLI_PROFILE_REPORT='))
assert line.split(b'=',1)[1]+b'\n'==raw('player-report-002.json')
assert 'incant_profile v' in raw('profile-graph-002.log').decode() and len(raw('profile-graph-002.log').decode().splitlines())==1
if a.local_binaries:
 for b in m['binaries']:
  path=Path(b['path']);assert H(path.read_bytes())==b['sha256'];assert path.stat().st_mode & 0o111
print(f"verified {len(records)} records, {len(payloads)} payloads, {len(m['sources'])} sources, {len(m['commands'])} commands, {len(m['binaries'])} binary identities")
