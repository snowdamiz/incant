from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil
sys.dont_write_bytecode=True
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/authored-ios-wasm-main185-001'
d=r/'docs/spikes/evidence/authored-ios-wasm-main185'
read=lambda p:json.loads(Path(p).read_text())
def h(p):
 v=hashlib.sha256()
 with Path(p).open('rb') as f:
  for x in iter(lambda:f.read(1024*1024),b''):v.update(x)
 return v.hexdigest()
sha=lambda b:hashlib.sha256(b).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='4b277d577c6627e98e407b2c25daf274a99fd406'
assert all(h(r/k)==v for k,v in f['files'].items())
assert read(o/'shipping-terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==22
assert [(x['name'],x['exit']) for x in checks if x['exit']] == [('tools-wasm2c',5)]
assert 'Ran 0 tests' in (o/'tools-wasm2c.stderr').read_text()
for c in checks:
 for s in ['stdout','stderr']:assert h(c[s]['path'])==c[s]['sha256']
tests=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', (o/'cook-tests.stdout').read_text())
assert sum(int(x[0]) for x in tests)==62 and all(int(x[1])==int(x[2])==0 for x in tests)
original=Path.home()/'.codex/incant-evidence/authored-ios-wasm-001/audit.py'
prefix=original.read_text().split('fallback = []')[0]
prefix=prefix.replace("OUT = Path.home()/'.codex/incant-evidence/authored-ios-wasm-shipping-001'", 'OUT = Path('+repr(str(o))+')')
prefix=prefix.replace("FREEZE = json.loads((OUT / 'continuation-freeze-002.json').read_text())",'')
prefix=prefix.replace("ORIGINAL = json.loads((OUT / 'freeze.json').read_text())",'')
prefix=prefix.replace("ROOT = Path('/Users/andreyyurlov/.codex/worktrees/local-service-stack/incant')",'ROOT = Path('+repr(str(r))+')')
prefix=prefix.replace("OUT.parent / 'authored-ios-wasm-preflight-001' / mode",'OUT / (mode + "-prepared")')
prefix=prefix.replace("for platform in ('simulator', 'device'):","for platform in ('simulator',):")
prefix=prefix.replace("OUT / (label + '-archive')","OUT / (label + '-export')")
prefix=prefix.replace("OUT / (label + '-callers' + ('-002' if label == 'fast-simulator' else ''))","OUT / (label + '-callers')")
prefix=prefix.replace("OUT / (opposite + '-simulator-archive')","OUT / (opposite + '-simulator-export')")
ns={'__file__':str(original)};exec(compile(prefix,str(original),'exec'),ns);execution=ns['records'];assert len(execution)==2
assert sum(x['positive_processes'] for x in execution)==12 and sum(x['exact_refusals'] for x in execution)==52
producer_path=r/'docs/spikes/evidence/authored-ios-wasm/manifest.json';producer=read(producer_path)
assert h(r/producer['archive']['path'])==producer['archive']['sha256']
pa=json.loads(gzip.decompress((r/producer['archive']['path']).read_bytes()))
for row in pa['records']:
 raw=base64.b64decode(pa['payloads'][row['sha256']]);assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert h(row['path'])==row['sha256']
for row in producer['external_binaries']:assert h(row['path'])==row['sha256']
source_rows=read(o/'producer-source-comparison.json')['rows'];assert len(source_rows)==13
for row in source_rows:
 assert h(r/row['path'])==row['sha256']
 assert sha(subprocess.check_output(['git','show','a7654dd9:'+row['path']],cwd=r))==row['producer_sha256']
assert [x['path'] for x in source_rows if not x['equal']]==['crates/incant_cook/src/project_build.rs','crates/incant_cook/tests/mobile_project.rs']
summary={'passed':True,'current_simulator_execution':execution,'frozen_inputs':len(f['files']),'producer_sources':len(source_rows),'successful_commands':21,'historical_no_tests_pattern_failure':1,'cook_cases':62,'producer_records':len(pa['records']),'producer_external_binaries':len(producer['external_binaries'])}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);artifacts=[];external=[];payloads={}
for q in sorted(o.rglob('*')):
 if not q.is_file() or '__pycache__' in q.parts:continue
 raw=q.read_bytes();s=sha(raw);row={'path':str(q),'sha256':s,'bytes':len(raw)}
 if q.name in ['caller-c','caller-swift','incant_headless','audio_package_oracle'] or q.suffix=='.a' or 'trusted-tools' in q.parts:
  external.append(row);continue
 artifacts.append(row);payloads.setdefault(s,base64.b64encode(raw).decode())
compressed=gzip.compress(json.dumps({'records':artifacts,'payloads':payloads},separators=(',',':')).encode(),mtime=0)
parts=[]
for i,pos in enumerate(range(0,len(compressed),16*1024*1024)):
 q=d/f'artifacts.json.gz.part-{i:03}';b=compressed[pos:pos+16*1024*1024];q.write_bytes(b);parts.append({'path':str(q.relative_to(r)),'sha256':sha(b),'bytes':len(b)})
archive={'parts':parts,'sha256':sha(compressed),'bytes':len(compressed),'records':len(artifacts),'payloads':len(payloads)}
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(o.parent/'authored-ios-wasm-main185-checks.py','checks.py'),(o.parent/'authored-ios-wasm-main185-checks-resume.py','checks-resume.py'),(o.parent/'authored-ios-wasm-main185-shipping.py','shipping.py'),(o.parent/'authored-ios-wasm-main184-001/integration-seams.patch','integration-seams.patch')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.UTC).isoformat(),'tested_head':f['head'],'base_main':'d5702546b34fcc6452a9fe2b22a0e2b23802991a','review_head':git('rev-parse','HEAD'),'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'sources':source_rows,'checks':checks,'execution':execution,'summary':summary,'archive':archive,'external_artifacts':external,'producer_evidence':{'path':str(producer_path.relative_to(r)),'sha256':h(producer_path),'review_path':'docs/spikes/evidence/authored-ios-wasm/final-review.json','review_sha256':h(r/'docs/spikes/evidence/authored-ios-wasm/final-review.json'),'records':len(pa['records']),'external_binaries':len(producer['external_binaries'])},'review_state':'Final current-main independent review pending','limits':['Current macOS arm64 host and local iOS27 simulator CPU only; both fixed physics modes.','Current archives are fresh normal exports. The original a765 device unsigned links and first-archive compiler-recovery qualification remain original producer evidence, not current device execution.','Initial tools-wasm2c discovery pattern matched no tests and exited5; retained. Corrected actual test names passed without source changes or repeated completed checks.','Android TypeScript admission preserved; static Wasm admission explicitly iOS-only.','No physical-device, minimum-OS, performance, rendering, signing or phase approval.']}
q=r/'docs/spikes/evidence/authored-ios-wasm-main185.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q),len(compressed),flush=True)
