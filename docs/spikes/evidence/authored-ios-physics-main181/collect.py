from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil
sys.dont_write_bytecode=True
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/authored-ios-physics-main181-001'
prepared=o.parent/'authored-ios-physics-preparation-main181'
d=r/'docs/spikes/evidence/authored-ios-physics-main181'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='6b1724690f80b5fdd979dfc166b9dda0194958cc'
assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==9 and all(x['exit']==0 for x in checks)
for c in checks:
 for stream in ['stdout','stderr']:assert h(c[stream]['path'])==c[stream]['sha256']
sys.path.insert(0,str(r/'tools'));from physics_features import validate_artifacts,compiler_messages
for mode in ['deterministic','fast']:
 row=next(c for c in checks if c['name']==mode+'-device-check');v=row['compiler_features']
 assert v==validate_artifacts(compiler_messages(o/(mode+'-device-check.stdout')),mode,target_dir=Path(v['target_output']).parents[1],target='aarch64-apple-ios',profile='ci')
original=Path.home()/'.codex/incant-evidence/authored-ios-physics-shipping-001/audit.py'
prefix=original.read_text().split('fallback = []')[0]
prefix=prefix.replace('OUT = Path(__file__).parent','OUT = Path('+repr(str(o))+')')
prefix=prefix.replace("prepared = OUT.parent / 'authored-ios-physics-preflight-001' / mode",'prepared = Path('+repr(str(prepared))+') / (mode + "-prepared")')
prefix=prefix.replace("for platform in ('simulator', 'device'):","for platform in ('simulator',):")
ns={'__file__':str(original)};exec(compile(prefix,str(original),'exec'),ns);execution=ns['records'];assert len(execution)==2
producer=read(r/'docs/spikes/evidence/authored-ios-physics/manifest.json')
assert h(r/producer['archive']['path'])==producer['archive']['sha256']
pa=read_gzip=json.loads(gzip.decompress((r/producer['archive']['path']).read_bytes()))
for row in pa['records']:
 data=base64.b64decode(pa['payloads'][row['sha256']]);assert sha(data)==row['sha256'] and len(data)==row['bytes'];assert h(row['path'])==row['sha256']
for row in producer['external_binaries']:assert h(row['path'])==row['sha256']
source_rows=[]
for p in git('diff-tree','--no-commit-id','--name-only','-r','818ff1b2').splitlines():
 b=subprocess.check_output(['git','show','818ff1b2:'+p],cwd=r);assert (r/p).read_bytes()==b;source_rows.append({'path':p,'sha256':sha(b),'producer_exact':True})
subprocess.run(['git','diff','--exit-code','489168fc',f['head'],'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml'],cwd=r,check=True)
prepares=read(prepared/'prepares.json');assert len(prepares)==2 and all(x['exit']==0 for x in prepares)
for row in prepares:assert h(row['headless']['path'])==row['headless']['sha256']
summary={'checks':len(checks),'current_simulator_execution':execution,'frozen_inputs':len(f['files']),'producer_source_files':len(source_rows),'producer_retained_records':len(pa['records']),'producer_device_links':'original749; current device checks compile only'}
print(json.dumps(summary))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);artifacts=[];external=[];payloads={}
for folder in [o,prepared]:
 for q in sorted(folder.rglob('*')):
  if not q.is_file():continue
  raw=q.read_bytes();s=sha(raw);row={'original':str(q),'sha256':s,'bytes':len(raw)}
  if q.name in ['caller-c','caller-swift'] or q.suffix=='.a':external.append(row);continue
  artifacts.append(row)
  if s not in payloads:
   try:payloads[s]={'encoding':'utf8','data':raw.decode()}
   except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'artifacts':artifacts,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(o.parent/'authored-ios-physics-main181-run.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'base_main':'569f491311fbd25b2d47354386739cc08b049f94','review_head':git('rev-parse','HEAD'),'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'producer_rows':source_rows,'checks':checks,'execution':execution,'reused_native_prepares':prepares,'reused_runtime_checks':{'android_tested_head':'489168fc49a23a4f4779907cd85793971dfe3cca','all_crates_manifests_lock_sdk_schemas_vendor_toolchain_equal':True,'scope':'Both-mode21player/mobile library tests,Clippy,7cook admission and actual matching headless solver audits; all Rust inputs unchanged by this Python-only iOS integration.'},'producer_evidence':{'path':'docs/spikes/evidence/authored-ios-physics/manifest.json','sha256':h(r/'docs/spikes/evidence/authored-ios-physics/manifest.json'),'review_path':'docs/spikes/evidence/authored-ios-physics/final-review.json','review_sha256':h(r/'docs/spikes/evidence/authored-ios-physics/final-review.json'),'records':len(pa['records']),'external_binaries':len(producer['external_binaries']),'scope':'Original749 four shipping archives/twelve simulator processes/device unsigned links. No original device artifact relabeled as current.'},'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(artifacts),'payloads':len(payloads)},'external_artifacts':external,'assertion_source':{'original':str(original),'sha256':h(original)},'review_state':'Final current-main independent audit pending','limits':['Current local arm64 iOS27.0 simulator CPU only; both fixed physics modes','Current device compilation only; unsigned shipping device links are original749 producer evidence','No physical-device/minimum-OS/performance/rendering/signing gate','Static Wasm iOS follow-on remains separate; Android static modules still refused']}
q=r/'docs/spikes/evidence/authored-ios-physics-main181.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q))
