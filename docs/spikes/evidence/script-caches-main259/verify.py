"""Check the reviewed script data/cache union and its exact current CPU evidence."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
if not __debug__:raise RuntimeError('assertions required')
p=argparse.ArgumentParser();p.add_argument('--verify-only',required=True,action='store_true');p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
D=Path(__file__).resolve().parent;R=D.parents[3];h=lambda b:hashlib.sha256(b).hexdigest();git=lambda *v:subprocess.check_output(['git',*v],cwd=R)
M=json.loads((D/'manifest.json').read_bytes())
def checked(row):
 b=(R/row['path']).read_bytes();assert h(b)==row['sha256'] and len(b)==row['bytes'],row;return b
checked(M['verifier'])
for row in M['reviews']+M['producer_files']:checked(row)
A=json.loads(gzip.decompress(checked(M['archive'])));raw={}
for row in A['records']:
 b=base64.b64decode(A['payloads'][row['sha256']],validate=True);assert h(b)==row['sha256'] and len(b)==row['bytes'];assert row['path'] not in raw;raw[row['path']]=b
 if a.originals:assert Path(row['original']).read_bytes()==b,row['path']
assert len(raw)==M['records'] and len(A['payloads'])==M['payloads'];read=lambda n:json.loads(raw[n]);F=read('freeze.json');result=read('result.json')
assert F['source']==M['source']==result['source'] and F['base']==M['base'];assert result['freeze_sha256']==h(raw['freeze.json']) and result['passed'] and result['completed'] and result['source_unchanged'] and result['target_released']
assert len(F['inputs'])==3885 and len(F['owned_paths'])==12
for ref in [F['base'],F['source']]:subprocess.run(['git','merge-base','--is-ancestor',ref,'HEAD'],cwd=R,check=True)
proc=subprocess.Popen(['git','cat-file','--batch'],cwd=R,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
def blob(ref,path):
 proc.stdin.write((ref+':'+path+'\n').encode());proc.stdin.flush();header=proc.stdout.readline().split();assert len(header)==3 and header[1]==b'blob';b=proc.stdout.read(int(header[2]));assert proc.stdout.read(1)==b'\n';return b
try:
 for path,digest in F['inputs'].items():assert h(blob(F['source'],path))==h((R/path).read_bytes())==digest,path
 for path in ['Cargo.toml','Cargo.lock','crates/incant_script/Cargo.toml']:assert blob(F['base'],path)==blob(F['source'],path)
finally:proc.stdin.close();proc.stdout.close();assert proc.wait()==0
assert git('diff','--name-only',F['base'],F['source'],'--','crates').decode().splitlines()==F['owned_paths']
assert git('diff',F['source'],'HEAD','--','crates','third_party','tools','.cargo','sdk','platforms','Cargo.toml','Cargo.lock','rust-toolchain.toml')==b''
for row in M['production_patches']:assert h(raw[row['record']])==row['sha256']
assert len(result['commands'])==16
for c in result['commands']:assert c['exit_code']==0 and c['reaped'] and h(raw[c['name']+'.log'])==c['log_sha256']
for mode in ('deterministic','fast-simd8'):
 for name,n in [('incant_script',144),('project_bindings',17),('bindings_static',6)]:
  assert re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw['test-'+mode+'-'+name+'.log'])==[(str(n).encode(),b'0',b'0')]
 build=next(c for c in result['commands'] if c['name']=='build-'+mode);assert build['argv'][build['argv'].index('--features')+1]=='compiler,physics-'+mode and '-j2' in build['argv'] and '--locked' in build['argv']
 lint=next(c for c in result['commands'] if c['name']=='clippy-'+mode);assert lint['argv'][-4:]==['--all-targets','--','-D','warnings']
assert len(result['binaries'])==6
for b in result['binaries']:
 artifacts=[]
 for line in raw['build-'+b['mode']+'.log'].splitlines():
  try:v=json.loads(line)
  except ValueError:continue
  if v.get('reason')=='compiler-artifact':artifacts.append(v)
 assert b['artifact'] in artifacts and b['artifact']['profile']['test'];assert set(b['artifact']['features'])=={'compiler','physics-'+b['mode']}
 if a.local_binaries:
  path=Path(b['path']);assert path.stat().st_mode&0o111 and path.stat().st_size==b['bytes'] and h(path.read_bytes())==b['sha256']
assert raw['graph.log'].strip()==b'Authoring-free resolved dependencies: incant_script'
for c in read('producer-checks.json'):assert c['exit_code']==0 and c['reaped'] and h(raw['producer-'+c['name']+'.log'])==c['sha256']
static=['python3','docs/spikes/evidence/script-bindings-static/verify.py','--verify-only','--historical']
if a.originals:static.append('--originals')
if a.local_binaries:static.append('--local-binaries')
subprocess.run(static,cwd=R,check=True)
for producer in ('script-instance-cache','script-chunk-id-cache','mac-core-baseline'):
 argv=['python3','docs/spikes/evidence/'+producer+'/verify.py']
 if a.originals and a.local_binaries:argv.append('--external')
 if producer=='mac-core-baseline':
  baseline=json.loads((R/'docs/spikes/evidence/mac-core-baseline/manifest.json').read_bytes())
  if subprocess.run(['git','cat-file','-e',baseline['executed_source']],cwd=R,capture_output=True).returncode==0:argv.append('--git-source')
 subprocess.run(argv,cwd=R,check=True)
print(json.dumps({'verified':True,'source':M['source'],'records':len(raw),'payloads':len(A['payloads']),'inputs':len(F['inputs']),'production_paths':12,'current_commands':16,'producer_replays':4,'cases_per_mode':167,'current_binary_copies':6,'shipping_speedup_claim':False}))
