from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o=Path.home()/'.codex/incant-integration-evidence/authored-android-pcm-main181-001'
old=o.parent/'authored-android-main179-003';d=r/'docs/spikes/evidence/authored-android-main181'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='489168fc49a23a4f4779907cd85793971dfe3cca'
assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==17 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ['stdout','stderr']:assert h(c[s]['path'])==c[s]['sha256']
counts={}
for c in checks:
 if c['name'].endswith('-tests') or c['name']=='cook-admission':
  m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',Path(c['stdout']['path']).read_text())
  assert m and all(int(x[1])==int(x[2])==0 for x in m)
  counts[c['name']]={'passed':sum(int(x[0]) for x in m),'failed':0,'ignored':0,'groups':len(m)}
for mode in ['deterministic','fast']:
 assert next(c for c in checks if c['name']==mode+'-headless')['compiler_features']
sys.path.insert(0,str(r/'tools'))
from physics_features import validate_artifacts,compiler_messages
fast_target=read(o/'fast-android-target-features.json')
assert fast_target==validate_artifacts(compiler_messages(o/'fast-android-target.stdout'),'fast',target_dir=Path(fast_target['target_output']).parents[1],target='aarch64-linux-android',profile='ci')
original=Path.home()/'.codex/incant-evidence/authored-android-shipping-002/audit.py'
prefix=original.read_text().split('freeze=json.loads')[0]
def audit(directory,modes):
 text=prefix.replace('out=pathlib.Path(__file__).parent','out=pathlib.Path('+repr(str(directory))+')')
 text=text.replace("for mode in ('deterministic','fast'):",'for mode in '+repr(modes)+':')
 ns={'__file__':str(original)};exec(compile(text,str(original),'exec'),ns)
 for mode in modes:
  result=read(directory/(mode+'-callers/result.json'))
  for control in result['controls'][3:]:
   request=read(control['request']);destination=Path(request['output_directory'])
   if control['expected_status']==2:assert not destination.exists()
   if control['expected_status']==1:
    expected={'c-late-failure':'script tick failed: Error: authored Android late failure','c-checksum-failure':'native package startup checksum mismatch'}[control['name']]
    assert control['report']['error']==expected and control['report']['game'] is None and control['report']['passed'] is False
    assert read(destination/'result.json')==control['report'] and not (destination/'audio.pcm').exists()
 return ns['records']
current=audit(o,('deterministic',));historical=audit(old,('deterministic','fast'))
assert len(current)==1 and len(historical)==2
prior=read(r/'docs/spikes/evidence/authored-android-main180.json')
assert h(r/prior['archive']['path'])==prior['archive']['sha256']
prior_archive=json.loads(gzip.decompress((r/prior['archive']['path']).read_bytes()))
for row in prior_archive['artifacts']:
 raw=prior_archive['payloads_by_sha256'][row['sha256']]
 data=raw['data'].encode() if raw['encoding']=='utf8' else base64.b64decode(raw['data'])
 assert sha(data)==row['sha256'] and len(data)==row['bytes'] and h(row['original'])==row['sha256']
for row in prior['external_artifacts']:assert h(row['original'])==row['sha256']
oldf=read(old/'freeze.json');fallback=[]
for p,s in oldf['files'].items():
 if h(r/p)!=s:
  assert sha(subprocess.check_output(['git','show',oldf['head']+':'+p],cwd=r))==s;fallback.append(p)
assert set(fallback)=={'.gitattributes','Cargo.lock','crates/incant_game_player/Cargo.toml','crates/incant_game_player/src/checkpoint.rs','crates/incant_game_player/src/lib.rs'}
for c in [prior['reused_script_check'],*prior['checks']]:
 assert c['exit']==0
 for stream in ['stdout','stderr']:assert h(c[stream]['path'])==c[stream]['sha256']
producer=read(r/'docs/spikes/evidence/authored-android-project/manifest.json');producer_rows=[]
for p,s in producer['source_inputs'].items():
 assert h(r/p)==s;producer_rows.append({'path':p,'sha256':s,'producer_exact':True})
pcm_paths=['Cargo.lock','crates/incant_game_player/src/lib.rs','crates/incant_game_player/src/checkpoint.rs','crates/incant_game_player/src/pcm_output.rs','crates/incant_game_player/src/pcm_output/tests.rs','crates/incant_game_player/src/pcm_output/tests/runner.rs']
for p in pcm_paths:assert (r/p).read_bytes()==subprocess.check_output(['git','show','b59e1de8:'+p],cwd=r)
subprocess.run(['git','merge-base','--is-ancestor','569f491311fbd25b2d47354386739cc08b049f94',f['head']],cwd=r,check=True)
summary={'checks':len(checks),'counts':counts,'current_execution':current,'historical_pre_pcm_execution':historical,'frozen_inputs':len(f['files']),'historical_git_fallback':fallback}
print(json.dumps(summary))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);artifacts=[];external=[];payloads={}
for q in sorted(o.rglob('*')):
 if not q.is_file():continue
 raw=q.read_bytes();s=sha(raw);row={'original':str(q),'sha256':s,'bytes':len(raw)}
 if q.name.startswith('incant_headless-') or q.name=='caller-c' or q.suffix=='.a':external.append(row);continue
 artifacts.append(row)
 if s not in payloads:
  try:payloads[s]={'encoding':'utf8','data':raw.decode()}
  except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'artifacts':artifacts,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(o.parent/'authored-android-pcm-main181-run.py','run.py'),(o/'source-relevance.json','source-relevance.json')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'base_main':'569f491311fbd25b2d47354386739cc08b049f94','review_head':git('rev-parse','HEAD'),'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'producer_rows':producer_rows,'pcm_exact_paths':pcm_paths,'checks':checks,'counts':counts,'execution':current,'historical_pre_pcm':{'manifest_path':'docs/spikes/evidence/authored-android-main180.json','manifest_sha256':h(r/'docs/spikes/evidence/authored-android-main180.json'),'tested_head':oldf['head'],'execution':historical,'checks':len(prior['checks']),'reused_script_tests':prior['counts']['script-tests'],'historical_git_fallback':fallback},'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(artifacts),'payloads':len(payloads)},'external_artifacts':external,'assertion_source':{'original':str(original),'sha256':h(original)},'source_relevance':'Mobile ABI calls crate::run, so PCM affects linked Android behavior. Final combined deterministic archive/callers and both-mode mobile tests plus fast Android target compile pass. Fast actual shipping execution remains the explicitly pre-PCM paired proof. Android implementation/probe and complete PCM Rust source are exact accepted inputs.','review_state':'Final current-main independent audit pending','limits':producer['limits']}
q=r/'docs/spikes/evidence/authored-android-main181.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q))
