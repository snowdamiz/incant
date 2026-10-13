from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o=Path.home()/'.codex/incant-integration-evidence/native-cpu-profiler-integration-001'
d=r/'docs/spikes/evidence/native-cpu-profiler-main194'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
producer='bfe8ede26d09c3e28da519b01ed3f3b4c41d1bf6'
main='071cdf1b7c235702948e27f15d36cda43755577f'
freeze=read(o/'freeze.json');terminal=read(o/'terminal.json');assert terminal['exit']==0
attrs=read(o/'raw-log-attributes.json')
for p,s in freeze['files'].items():
 if p=='.gitattributes':continue
 assert h(r/p)==s,p
assert h(o/'raw-log-attributes.patch')=='9fe35a76c4d612795d32d62918fb7c94f88fc898bff9ca8056c0e47ade775bb5'
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
assert not git('diff','--name-only',freeze['head'],'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml').strip()
checks=read(o/'results.json');assert len(checks)==10 and all(x['exit']==0 for x in checks)
for row in checks:
 for key in ('stdout','stderr'):assert h(row[key]['path'])==row[key]['sha256']
counts={}
for name in ['affected','default-player']:
 row=next(x for x in checks if x['name']==name)
 text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text()
 matches=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
 counts[name]={'passed':sum(int(x[0]) for x in matches),'failed':sum(int(x[1]) for x in matches),'ignored':sum(int(x[2]) for x in matches),'groups':len(matches)}
 assert counts[name]['failed']==counts[name]['ignored']==0
assert counts['affected']['passed']==495 and counts['default-player']['passed']==19
relevance=read(o/'source-relevance.json');graphs=read(o/'graph-relevance.json')
assert relevance['passed'] and graphs['passed'] and len(relevance['selector_controls'])==5
for row in relevance['runtime_source_rows']+relevance['changed_inputs']:assert h(r/row['path'])==row['current_sha256']
for row in relevance['unchanged_export_inputs']:assert h(r/row['path'])==row['sha256']
for row in graphs['graphs']:
 for key in ['current_stdout','producer_stdout']:assert h(row[key]['path'])==row[key]['sha256']
 assert row['exact_package_versions_and_features_after_only_checkout_path_normalization']
p=read(r/'docs/spikes/evidence/native-cpu-profiling.json');sources=[]
for row in p['source_identities']+p['helper_identities']:
 old=git('show',producer+':'+row['path']);assert sha(old)==row['sha256']
 current=h(r/row['path'])
 if row['path']!='Cargo.lock':assert current==row['sha256']
 sources.append({'path':row['path'],'producer_sha256':row['sha256'],'current_sha256':current,'qualified_lock_difference':row['path']=='Cargo.lock'})
for row in p['artifacts']:
 raw=Path(row['original']).read_bytes();stored=(r/row['retained']).read_bytes()
 assert sha(raw)==row['raw_sha256'] and len(raw)==row['raw_bytes'];assert sha(stored)==row['sha256'] and len(stored)==row['bytes']
 assert (gzip.decompress(stored) if row['encoding']=='gzip' else stored)==raw
for row in p['package_files']+p['external_binaries']:assert h(row['path'])==row['sha256'] and Path(row['path']).stat().st_size==row['bytes']
review=p['independent_review'];assert h(r/review['path'])==review['sha256']=='63843e9675028b45e06ed0ffa99d54603fbd369c84708dc6d0e024ffc4619efa'
proof=Path(p['proof_root']);result=read(proof/'result.json');assert result==p['result'] and result['passed'] and result['all_processes_reaped']
assert len(result['refusals'])==5 and h(proof/'capture.tracy')==result['capture_sha256']
for mode,files in result['packages'].items():
 root=Path(result['package_root'])/('package-'+mode)
 assert not list(root.rglob('*.map'))
 for path,s in files.items():assert h(root/path)==s
assert not (Path(result['package_root'])/'authoring').exists()
baseline=None;pcmhash=None;processes=[]
for name in ['summary-baseline','summary-summary','tracy-baseline','tracy-summary','tracy-captured']:
 row=read(proof/'commands'/f'{name}.json');assert row['exit']==0 and not row['stderr'] and row['network_queries']>0
 for stream in ['stdout','stderr']:assert h(proof/'commands'/f'{name}.{stream}')==row[stream+'_sha256']==sha(row[stream].encode())
 report=json.loads(row['stdout']);profile=report.pop('profile',None);assert report['tick']==3600 and report['observations']['passed']
 pcm=proof/f'{name}.pcm';assert h(pcm)==report['pcm_sha256'] and pcm.stat().st_size==report['frames']*8
 if baseline is None:baseline=report;pcmhash=h(pcm)
 assert report==baseline and h(pcm)==pcmhash
 if name.endswith('baseline'):assert profile is None and not row['network']
 else:
  assert profile['completed_frames']==3600 and profile['failed_frames']==0 and profile['quickjs_heap']['samples']==3600
  assert len(profile['scripts'])==2 and all(x['totals']['calls']==3600 for x in profile['scripts'])
  assert profile['rust_allocations']['installed'] and not profile['rust_allocations']['saturated']
  assert profile['tracy_process_transport']==(name=='tracy-captured')
  assert ('PRIVATE-authored-marker' not in json.dumps(profile))
 if name=='tracy-captured':assert row['capture_started_after_owned_listener'] and any('127.0.0.1:8086 (LISTEN)' in x for x in row['network'])
 processes.append({'name':name,'receipt_sha256':h(proof/'commands'/f'{name}.json'),'stdout_sha256':row['stdout_sha256'],'pcm_sha256':h(pcm),'producer_scoped':True})
for name,diagnostic in [('duplicate','Error: Options\n'),('summary-transport','Error: TracyUnavailable\n'),('unsupported','Error: UnsupportedRoute\n')]:
 row=read(proof/'commands'/f'refusal-{name}.json');assert row['exit']!=0 and not row['stdout'] and row['stderr']==diagnostic
guest=[]
for i in [0,1]:
 row=read(proof/'commands'/f'refusal-guest-{i}.json');assert row['exit']!=0 and not row['stdout'] and not (proof/f'failed-{i}.pcm').exists();assert 'cpu-profiler deliberate trap' in row['stderr'];guest.append(row['stderr'])
assert guest[0]==guest[1]
summary={'passed':True,'current_checks':len(checks),'current_counts':counts,'frozen_inputs':len(freeze['files']),'producer_inputs':len(sources),'normal_target_source_inputs':len(relevance['runtime_source_rows']),'producer_artifacts_rehashed':len(p['artifacts']),'producer_package_files_rehashed':len(p['package_files']),'producer_external_binaries_rehashed':len(p['external_binaries']),'producer_shipping_runs':5,'producer_refusals':5,'current_shipping_runs':0,'new_shipping_build':False,'current_target_graph_modes':3,'selector_controls':5}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);records.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
encoded=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);parts=[]
for i,offset in enumerate(range(0,len(encoded),16*1024*1024)):
 path=d/f'artifacts.json.gz.part-{i:03}';path.write_bytes(encoded[offset:offset+16*1024*1024]);parts.append({'path':str(path.relative_to(r)),'sha256':h(path),'bytes':path.stat().st_size})
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o/'source-relevance.json','source-relevance.json'),(o/'graph-relevance.json','graph-relevance.json'),(o/'source-relevance.patch','source-relevance.patch'),(o.parent/'profile-source-relevance.py','source-relevance.py'),(o.parent/'profile-graph-relevance.py','graph-relevance.py'),(o.parent/'native-cpu-profiler-integration-run.py','run.py'),(o.parent/'native-cpu-profiler-integration-resume.py','resume.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':freeze['head'],'review_head':git('rev-parse','HEAD').decode().strip(),'main_ancestor':main,'producer':producer,'summary':summary,'sources':sources,'checks':checks,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'parts':parts,'gzip_sha256':sha(encoded),'gzip_bytes':len(encoded),'records':len(records),'payloads':len(payloads)},'relevance':{'source':{'path':str((d/'source-relevance.json').relative_to(r)),'sha256':h(d/'source-relevance.json')},'graphs':{'path':str((d/'graph-relevance.json').relative_to(r)),'sha256':h(d/'graph-relevance.json')}},'producer_process_comparisons':processes,'producer_review':review,'limits':['Current integration runs CPU/Clippy/wasm/tools/dependency checks; producer shipping builds, five processes and partial Tracy capture retain their exact original scope.','No new shipping archive/process/capture or full profiler/performance/phone/GPU/window claim.','One cfg(test,compiler) quota-only test differs in the normal target package sources; normal target production and exact dependency features are unchanged.','Build-host mobile static-Wasm changes are guarded outside the desktop TypeScript path; five old/new bounded selector controls match without invoking any build.','Original guard invocation usage failure, raw-log EOF metadata check failure and linked temporary selector fixture failure remain retained separately; no production fix was needed.'],'review_state':'Independent current-main acceptance pending'}
path=r/'docs/spikes/evidence/native-cpu-profiler-main194.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(str(path),h(path),flush=True)
