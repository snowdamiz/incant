from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/authored-android-main179-003';d=r/'docs/spikes/evidence/authored-android-main180'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='79557b893a13e13ac5eacf7356b3a1591a12bde8';assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==19 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ['stdout','stderr']:assert h(c[s]['path'])==c[s]['sha256']
reused=read(o/'reused-script-tests.json')['check'];assert reused['exit']==0
for s in ['stdout','stderr']:assert h(reused[s]['path'])==reused[s]['sha256']
counts={}
for c in [reused,*checks]:
 if 'tests' in c['name']:
  m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',Path(c['stdout']['path']).read_text());assert m and all(int(x[1])==int(x[2])==0 for x in m);counts[c['name']]={'passed':sum(int(x[0]) for x in m),'failed':0,'ignored':0,'groups':len(m)}
for mode in ['deterministic','fast']:
 c=next(c for c in checks if c['name']==mode+'-headless');assert c['compiler_features']['mode']==mode if 'mode' in c['compiler_features'] else c['compiler_features']
# Reuse only the already independently reviewed assertion prefix, binding its bytes.
original=Path.home()/'.codex/incant-evidence/authored-android-shipping-002/audit.py';prefix=original.read_text().split('freeze=json.loads')[0];prefix=prefix.replace('out=pathlib.Path(__file__).parent','out=pathlib.Path('+repr(str(o))+')');ns={'__file__':str(original)};exec(compile(prefix,str(original),'exec'),ns);records=ns['records'];assert len(records)==2
producer=read(r/'docs/spikes/evidence/authored-android-project/manifest.json');rows=[]
for path,s in producer['source_inputs'].items():
 assert h(r/path)==s;rows.append({'path':path,'producer_sha256':s,'current_sha256':h(r/path)})
print(json.dumps({'counts':counts,'checks':len(checks),'source_inputs':len(f['files']),'execution':records}))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(exist_ok=True,parents=True);artifacts=[];external=[];payloads={}
for q in sorted(o.rglob('*')):
 if not q.is_file():continue
 raw=q.read_bytes();s=sha(raw);row={'original':str(q),'sha256':s,'bytes':len(raw)}
 if q.name.startswith('incant_headless-') or q.name=='caller-c' or q.suffix=='.a':external.append(row);continue
 artifacts.append(row)
 if s not in payloads:
  try:payloads[s]={'encoding':'utf8','data':raw.decode()}
  except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'artifacts':artifacts,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(o.parent/'authored-android-main179-resume.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'base_main':'02203670a7f18c68ff9e3bcf1913b0d574e82f6d','review_head':git('rev-parse','HEAD'),'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'producer_rows':rows,'checks':checks,'reused_script_check':reused,'counts':counts,'execution':records,'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(artifacts),'payloads':len(payloads)},'external_artifacts':external,'assertion_source':{'original':str(original),'sha256':h(original)},'initial_failure':'Old cooker test expected exact ARM Android refusal. Reviewed test-only correction79557 retains unknown architecture refusals and unchanged production. Original002 failure preserved separately.','review_state':'Independent final integration review and actualmain ancestry pending','limits':producer['limits']}
q=r/'docs/spikes/evidence/authored-android-main180.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q))
