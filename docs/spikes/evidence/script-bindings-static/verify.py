"""Verify static binding source and retained CPU evidence without executing binaries."""
import argparse,base64,gzip,hashlib,json,re,subprocess
from pathlib import Path
if not __debug__:raise RuntimeError('assertions required')
p=argparse.ArgumentParser();p.add_argument('--verify-only',required=True,action='store_true');p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');p.add_argument('--historical',action='store_true',help='Verify the immutable tested commit; report rather than refuse later source changes.');a=p.parse_args()
D=Path(__file__).resolve().parent;R=D.parents[3];h=lambda b:hashlib.sha256(b).hexdigest();git=lambda *v:subprocess.check_output(['git',*v],cwd=R)
M=json.loads((D/'manifest.json').read_bytes())
def checked(row):
 b=(R/row['path']).read_bytes();assert len(b)==row['bytes'] and h(b)==row['sha256'],row;return b
checked(M['verifier'])
for row in M['reviews']:checked(row)
A=json.loads(gzip.decompress(checked(M['archive'])));raw={}
for row in A['records']:
 b=base64.b64decode(A['payloads'][row['sha256']],validate=True);assert h(b)==row['sha256'] and len(b)==row['bytes'];assert row['path'] not in raw;raw[row['path']]=b
 if a.originals:assert Path(row['original']).read_bytes()==b,row['path']
assert len(raw)==M['records'] and len(A['payloads'])==M['payloads'];read=lambda n:json.loads(raw[n])
F=read('source-001/freeze.json');I=read('committed-inputs.json');assert F['base']==M['base'] and I['source']==M['source']
assert len(F['files'])==6 and len(I['files'])==M['inputs'];assert git('diff','--name-only',M['base'],M['source']).decode().splitlines()==sorted(r['path'] for r in F['files'])
assert b'Built-by: astra' in git('show','-s','--format=%B',M['source']);subprocess.run(['git','merge-base','--is-ancestor',M['source'],'HEAD'],cwd=R,check=True)
proc=subprocess.Popen(['git','cat-file','--batch'],cwd=R,stdin=subprocess.PIPE,stdout=subprocess.PIPE)
def blob(path):
 proc.stdin.write((M['source']+':'+path+'\n').encode());proc.stdin.flush();header=proc.stdout.readline().split();assert len(header)==3 and header[1]==b'blob';b=proc.stdout.read(int(header[2]));assert proc.stdout.read(1)==b'\n';return b
changed=[]
try:
 for path,digest in I['files'].items():
  assert h(blob(path))==digest,path
  if not (R/path).is_file() or h((R/path).read_bytes())!=digest:changed.append(path)
finally:proc.stdin.close();proc.stdout.close();assert proc.wait()==0
if not a.historical:assert not changed,changed
for row in F['files']:assert I['files'][row['path']]==row['sha256']==h(raw['source-001/'+row['path']])
for path in ['Cargo.toml','Cargo.lock','crates/incant_script/Cargo.toml']:assert git('diff',M['base'],M['source'],'--',path)==b''
initial=read('initial-001/result.json');portable=read('portability-001/result.json');consumer=read('consumer-002/consumer-result.json');failed=read('initial-001/consumer-result.json');count=0
for directory,result in [('initial-001',initial),('portability-001',portable),('consumer-002',consumer)]:
 assert result['source']==M['source'] and result['passed']
 for c in result['commands']:
  assert c['exit_code']==0 and c['reaped'];count+=1
  name=('consumer-' if directory=='consumer-002' else '')+c['name']+'.log';assert h(raw[directory+'/'+name])==c['log_sha256']
assert count==M['successful_commands']==14
assert not failed['passed'] and len(failed['commands'])==1 and failed['commands'][0]['exit_code']==1 and failed['commands'][0]['reaped'];assert h(raw['initial-001/consumer-compile.log'])==failed['commands'][0]['log_sha256'] and b'E0463' in raw['initial-001/consumer-compile.log']
expected={'incant_script':138,'bindings_static':6,'project_bindings':12}
for name,n in expected.items():
 cases=re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw['initial-001/tests-'+name+'.log']);assert cases==[(str(n).encode(),b'0',b'0')]
assert sum(expected.values())==M['distinct_cpu_cases']==156
assert re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw['initial-001/emit-fixture.log'])==[(b'1',b'0',b'0')]
assert b'10000rows,all property kinds and f64 bits' in raw['consumer-002/consumer-execute.log']
assert consumer['inputs'][:5]==failed['inputs'][:5];assert raw['consumer-002/consumer.rs']==raw['initial-001/consumer.rs']
for row in consumer['inputs'][2:]:
 matches=[b for path,b in raw.items() if row['path'].endswith('/'+path)];assert len(matches)==1 and h(matches[0])==row['sha256']
artifacts=read('initial-001/artifacts.json')
for row in consumer['inputs'][:2]:assert any(row['path'] in artifact['filenames'] for artifact in artifacts)
assert len([x for x in consumer['commands'][0]['argv'] if x=='-L'])==2 and failed['commands'][0]['argv'].count('-L')==1
assert read('initial-001/generated/expected.json')['instances']==10000
B=read('binaries.json');assert len(B)==M['copied_binaries']==4
for row in B:
 if a.local_binaries:
  path=Path(row['path']);assert path.stat().st_mode&0o111 and path.stat().st_size==row['bytes'] and h(path.read_bytes())==row['sha256']
for row in initial['binaries']:
 artifact=[v for v in artifacts if v.get('executable') and v['target']['name']==row['name'] and v['profile']['test']];assert len(artifact)==1
 assert {'compiler','physics-deterministic'}<=set(artifact[0]['features'])
assert raw['portability-001/runtime-graph.log'].strip()==b'Authoring-free resolved dependencies: incant_script'
assert b'--target' in raw['run-portability.py'] and b'wasm32-unknown-unknown' in raw['run-portability.py']
print(json.dumps({'verified':True,'source':M['source'],'inputs':len(I['files']),'records':len(raw),'payloads':len(A['payloads']),'commands':count,'historical_failed_commands':1,'distinct_cpu_cases':156,'generated_consumer_rows':10000,'copied_binaries':4,'later_source_changes':changed,'shipping_speedup_claim':False}))
