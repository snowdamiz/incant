"""Replay profiler/tooling source and CPU evidence without launching captured code."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
if not __debug__:raise RuntimeError('assertions required')
p=argparse.ArgumentParser();p.add_argument('--verify-only',required=True,action='store_true');p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
D=Path(__file__).resolve().parent;R=D.parents[3];h=lambda b:hashlib.sha256(b).hexdigest();git=lambda *args:subprocess.check_output(['git',*args],cwd=R)
M=json.loads((D/'manifest.json').read_bytes())
def checked(row):
 b=(R/row['path']).read_bytes();assert len(b)==row['bytes'] and h(b)==row['sha256'];return b
checked(M['verifier']);Q=json.loads(checked(M['qualification']))
for row in M['producer_files']+M['reviews']:checked(row)
A=json.loads(gzip.decompress(checked(M['archive'])));raw={}
for row in A['records']:
 b=base64.b64decode(A['payloads'][row['sha256']],validate=True);assert h(b)==row['sha256'] and len(b)==row['bytes'];assert row['path'] not in raw;raw[row['path']]=b
 if a.originals:assert Path(row['original']).read_bytes()==b
assert len(raw)==M['records'];read=lambda n:json.loads(raw[n]);S=read('source-map.json');F=read('inputs.json');result=read('result.json')
assert S['base']=='5017908020c318e644407c8a83342a30b68cde51' and S['source']=='f0f91be4fd1761edd9b0dfd042f5d818ca8ee1aa'
assert len(F)==S['inputs']==3880 and len(S['paths'])==15
assert result['source']==S['source'] and result['outcome']=='passed' and result['source_unchanged'] and result['inputs_sha256']==h(raw['inputs.json'])
assert result['target_released'] and result['child_reaped']
for ref in [S['base'],S['source'],Q['profiler_packet'],Q['runner_packet']]:subprocess.run(['git','merge-base','--is-ancestor',ref,'HEAD'],cwd=R,check=True)
for row in read('ancestry.json'):
 assert row['before_tree']==row['after_tree']==git('rev-parse',row['head']+'^{tree}').decode().strip()
 assert git('rev-parse',row['head']+'^1^{tree}').decode().strip()==row['before_tree']
proc=subprocess.Popen(['git','cat-file','--batch'],cwd=R,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
def blob(ref,path):
 proc.stdin.write(f'{ref}:{path}\n'.encode());proc.stdin.flush();header=proc.stdout.readline().split();assert len(header)==3 and header[1]==b'blob';b=proc.stdout.read(int(header[2]));assert proc.stdout.read(1)==b'\n';return b
try:
 for path,digest in F.items():assert h((R/path).read_bytes())==h(blob(S['source'],path))==digest,path
 for row in S['paths']:assert h(blob(row['producer'],row['path']))==row['sha256']==F[row['path']] and row['producer_exact']
 roots=['crates','third_party','tools','.cargo','sdk','platforms','Cargo.toml','Cargo.lock','rust-toolchain.toml']
 assert git('diff','--name-only',S['base'],S['source'],'--',*roots).decode().splitlines()==[r['path'] for r in S['paths']]
 assert git('diff','--name-only',S['source'],'HEAD','--',*roots)==b''
 for path in ['Cargo.toml','Cargo.lock','crates/incant_game_player/Cargo.toml','crates/incant_script/Cargo.toml','crates/incant_profile/Cargo.toml']:assert blob(S['base'],path)==blob(S['source'],path)
finally:proc.stdin.close();proc.stdout.close();assert proc.wait()==0
assert len(result['commands'])==Q['commands']
for c in result['commands']:assert c['exit']==0 and c['reaped'] and h(raw[Path(c['log']).name])==c['sha256']
assert sum(c.get('count',0) for c in result['cases'])==17
assert next(c for c in result['cases'] if c['name']=='dedicated-allocation-process')['exit']==0
actual=0
for c in result['commands']:
 if not c['name'].startswith('test-'):continue
 matches=re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw[Path(c['log']).name])
 assert all(f==i==b'0' for _,f,i in matches);actual+=sum(int(n) for n,_,_ in matches)
assert actual==17
for b in result['binaries']:
 a1=b['artifact'];messages=[]
 for line in raw['build-'+b['stage']+'.log'].splitlines():
  try:messages.append(json.loads(line))
  except ValueError:pass
 assert a1 in messages and a1['reason']=='compiler-artifact'
 features=set(a1['features']);stage=b['stage']
 if stage=='default':assert features=={'default'}
 elif stage=='summary':assert 'summary' in features and 'tracy' not in features
 elif stage=='script':assert {'profile','compiler','physics-deterministic'}<=features
 elif stage=='player':assert {'profile','physics-deterministic'}<=features and 'static-module' not in features
 else:raise AssertionError(stage)
 if a.local_binaries:
  path=Path(b['path']);assert h(path.read_bytes())==b['sha256'] and path.stat().st_size==b['bytes'] and path.stat().st_mode&0o111
cli=read('player-report.json');assert cli['profile']['version']==2
assert cli['profile']['frame_wall']['attempted_frames']==cli['profile']['frame_wall']['successful_frames']==3
lines=[line.split(b'=',1)[1]+b'\n' for c in result['commands'] if c['name'].startswith('test-player') for line in raw[Path(c['log']).name].splitlines() if line.startswith(b'CLI_PROFILE_REPORT=')];assert lines==[raw['player-report.json']]
import sys
sys.path.insert(0,str(R/'tools'));import mac_performance
assert mac_performance.metrics(cli,3)==json.loads(raw['parse-current-report.log'])
recorded_root=result['binaries'][0]['artifact']['manifest_path'].split('/crates/',1)[0]
assert raw['default-graph.log'].decode().splitlines()==['incant_profile v0.0.1 ('+recorded_root+'/crates/incant_profile)']
assert b'Ran 8 tests' in raw['runner-tests.log'] and raw['runner-tests.log'].endswith(b'OK\n')
args=['python3',str(R/'docs/spikes/evidence/native-profile-distributions/verify.py')]
if a.local_binaries:args.append('--local-binaries')
subprocess.run(args,cwd=R,check=True);subprocess.run(['python3',str(R/'docs/spikes/evidence/mac-cpu-runner/verify.py')],cwd=R,check=True)
print(json.dumps({'verified':True,'inputs':len(F),'producer_exact_paths':15,'commands':len(result['commands']),'named_cpu_cases':17,'runner_cases':8,'allocation_process':True,'records':len(raw),'binary_copies':len(result['binaries']),'shipping_performance':False}))
