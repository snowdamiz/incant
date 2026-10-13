from pathlib import Path
import base64, gzip, hashlib, json, re, shutil, subprocess, sys
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
o=Path.home()/'.codex/incant-integration-evidence/gpu-timestamps-main206-001'
d=r/'docs/spikes/evidence/gpu-timestamps-main207'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
main='9aa8165f3c3a5ec4754b1bda180cf6169bd7c21c'
producer='9da53be04c008a992b638c87129178df4e6000a9'
tested='7e6ac08b58fbc4659daf6c35bd6fd9ea7e5d89f0'
joined='5686cd2d4a9b3abf907796ba959e7e265c41de7c'
scopes=['crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github']
seams=['crates/incant_render/src/graph.rs','crates/incant_render/src/lib.rs','crates/incant_render/src/timing/mod.rs']
patch=o.parent/'gpu-timestamps-main206-seams/integration-seams.patch'
assert h(patch)=='0186fb0f8444b37825171cbbefd08b0001a4029c36d293e735fd41ec2e5b121f'
assert git('diff',producer,tested,'--',*seams)==patch.read_bytes()
f=read(o/'freeze.json');t=read(o/'terminal.json')
assert f['head']==tested==t['head'] and t['exit']==0 and t['commands']==10 and t['all_frozen_inputs_unchanged'] and not t['gpu_execution']
assert set(f['files'])==set(git('ls-tree','-r','--name-only',tested,'--',*scopes).decode().splitlines())
for p,s in f['files'].items():assert h(r/p)==s,p
assert not git('diff','--name-only',tested,'HEAD','--',*scopes)
assert git('rev-parse',tested+'^{tree}')==git('rev-parse',joined+'^{tree}')
for head in [main,producer,tested,joined]:subprocess.run(['git','merge-base','--is-ancestor',head,'HEAD'],cwd=r,check=True)
assert (r/'Cargo.lock').read_bytes()==git('show',main+':Cargo.lock')
checks=read(o/'results.json');assert len(checks)==10 and all(x['exit']==0 for x in checks)
counts={}
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
 if row['name'] in ['default-cpu','cooked-cpu']:
  text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text()
  m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
  counts[row['name']]={'passed':sum(int(x[0]) for x in m),'failed':sum(int(x[1]) for x in m),'ignored':sum(int(x[2]) for x in m),'nonempty_groups':sum(int(x[0])>0 for x in m),'summary_rows':len(m)}
assert counts['default-cpu']=={'passed':103,'failed':0,'ignored':94,'nonempty_groups':6,'summary_rows':10}
assert counts['cooked-cpu']=={'passed':130,'failed':0,'ignored':94,'nonempty_groups':6,'summary_rows':10}
base=git('merge-base',main,producer).decode().strip()
paths=git('diff','--name-only',base,producer,'--',*scopes).decode().splitlines()
assert len(paths)==18 and set(paths)==set(git('diff','--name-only',main,tested,'--',*scopes).decode().splitlines())
sources=[]
for p in paths:
 b=git('show',producer+':'+p);c=(r/p).read_bytes()
 assert p in seams or c==b,p
 sources.append({'path':p,'producer_sha256':sha(b),'current_sha256':sha(c),'reviewed_seam':p in seams})
# Every implementation/build change from actual main is one of these18 reviewed files.
# Current motion/DRS/controller/consumer files and every dependency remain main207 bytes.
for p in git('ls-files','crates/incant_render/src/resolution','crates/incant_render_gpu_host','crates/incant_render/src/motion').decode().splitlines():assert (r/p).read_bytes()==git('show',main+':'+p),p
reviews=[('gpu-frame-envelope-source-20261013.json','816d243f8cb64d84e338890bfc4692b8d1e2f261d6af5fdf1d3cb4252e0f31ac'),('gpu-timestamps-envelope-final-20261013.json','df1ef75d22b54edfd191d6fb62f51d98a873e91ebbdbf687b47263964f0965bc')]
review_paths=[]
for name,s in reviews:
 p=Path.home()/'.codex/incant-reviews'/name;assert h(p)==s;review_paths.append(p)
prior_specs=[('gpu-frame-envelope','26ecacd4ca1dd0881033470c8ae14d8336d686bd3c0408b41f990cde5cdab1d5'),('gpu-timestamps-r2','ed606eedfc155bedf9b81a5603447d48b94a4d1848dd441711885fd00080e59e'),('gpu-timestamps-pass-resolve','17cccdc112948d7b1ff56f0ef7626a3b9645853ed7359a04e27f96871e3c52a7')]
priors=[]
for name,s in prior_specs:
 p=r/'docs/spikes/evidence'/f'{name}.json';assert h(p)==s;j=read(p);records=j['records']
 for row in records:
  z=(r/row['retained']).read_bytes();b=gzip.decompress(z)
  assert sha(z)==row['retained_sha256'] and sha(b)==row['sha256'] and len(b)==row['bytes']
  orig=row.get('original',row.get('path'))
  if orig.startswith('source/'):actual=git('show',producer+':'+orig[7:])
  elif orig=='peer-review.json':actual=review_paths[0].read_bytes()
  else:actual=Path(orig).read_bytes()
  assert actual==b,orig
 if name=='gpu-frame-envelope':
  for path,s in j['sources'].items():assert sha(git('show',producer+':'+path))==s
  assert len(j['checks'])==3 and all(x['exit']==0 for x in j['checks'])
 if name=='gpu-timestamps-r2':assert sum(j['counts'].values())==92 and len(j['original_frame_aggregation_finding'])==3
 priors.append({'path':str(p.relative_to(r)),'sha256':h(p),'records':len(records)})
summary={'checks':10,'counts':counts,'frozen_inputs':len(f['files']),'producer_records':sum(x['records'] for x in priors),'producer_crate_paths':len(sources),'exact_producer_crate_paths':len(sources)-len(seams),'reviewed_seams':seams,'main':main,'producer':producer,'gpu_execution':False,'tested_and_joined_trees_equal':True}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True)
ancestry={'main':main,'producer':producer,'tested':tested,'joined':joined,'tree':git('rev-parse',joined+'^{tree}').decode().strip(),'compiled_input_diff':git('diff','--name-only',tested,joined,'--',*scopes).decode(),'main_merge_receipt':read(o.parent/'pr207/merge.json')}
(o/'ancestry.json').write_text(json.dumps(ancestry,indent=2)+'\n')
artifacts=[];payloads={}
for path in sorted(list(o.iterdir())+[patch]+review_paths+[o.parent/'gpu-timestamps-main206-run.py']):
 if path.is_file():
  b=path.read_bytes();s=sha(b);artifacts.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':artifacts,'payloads':payloads},separators=(',',':')).encode(),mtime=0)
assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'gpu-timestamps-main206-run.py','run.py'),(patch,'integration-seams.patch'),(review_paths[0],'producer-source-review.json'),(review_paths[1],'producer-final-review.json')]:shutil.copyfile(src,d/name)
m={'tested_head':tested,'joined_head':joined,'main':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'producer_evidence':priors,'freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'collector':{'path':str((d/'collect.py').relative_to(r)),'sha256':h(d/'collect.py')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(artifacts),'payloads':len(payloads)},'integration_seams_sha256':h(patch),'limits':['Current integration ran no GPU or native window. Default103 and cooked130 CPU cases overlap;94 GPU cases were ignored in each suite.','The92 actual default-feature M5 Pro Metal cases and paired identical PNG bytes belong to f33495b2; final envelope correction9da53be0 was CPU-tested only. No current composed GPU or image review is inferred.','Seven timestamp pass groups only. Frame is min start through max end of instrumented pairs including intervening gaps; it does not measure complete-frame GPU cost. Retained motion has no query pair.','Five wasm native-only timing dead-code warnings retained. Browser timing admission fails explicitly.','No native-window, other-backend GPU, phone, performance or complete-profiler gate approval.'],'review':'Independent final integration review pending.'}
p=r/'docs/spikes/evidence/gpu-timestamps-main207.json';p.write_text(json.dumps(m,indent=2)+'\n');print(p,h(p))
