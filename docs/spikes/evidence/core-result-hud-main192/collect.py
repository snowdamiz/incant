from pathlib import Path
import base64,gzip,hashlib,json,subprocess,sys,shutil,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/core-result-hud-main192-001';d=r/'docs/spikes/evidence/core-result-hud-main192'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==5 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ('stdout','stderr'):assert h(c[s]['path'])==c[s]['sha256']
main='7fc413d23cbb65e244ca515ca5d684aec21cfb90';producer='16656913b3ea6ee6aa422875af4dcf1f0c6aca56'
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
assert subprocess.run(['git','merge-base','--is-ancestor','f517265a568724a5aa09ff1abc939c9368d35afb','HEAD'],cwd=r).returncode==1
p=read(r/'docs/spikes/evidence/core-result-hud.json');source=[]
for row in p['sources']:
 if not row['path'].startswith('samples/'):continue
 assert h(r/row['path'])==row['sha256']==sha(subprocess.check_output(['git','show',producer+':'+row['path']],cwd=r))
 source.append({'path':row['path'],'sha256':row['sha256']})
review=Path.home()/'.codex/incant-reviews/core-result-hud-final-20261013.json';assert h(review)=='d7506cad86b502449ada9f258a78f7e774bca789636224d74c644a2f33ff1b55'
# Producer process evidence remains historical. Rehash without mutating its receipts.
for row in p['artifacts']:
 raw=Path(row['original']).read_bytes();ret=row['retained'];encoded=(r/ret['path']).read_bytes()
 assert sha(raw)==row['sha256'] and len(raw)==row['bytes'];assert sha(encoded)==ret['sha256'];assert gzip.decompress(encoded)==raw
for row in p['external_artifacts']:assert h(row['original'])==row['sha256']
n=o/'native';old=Path.home()/'.codex/incant-evidence/core-result-hud-native-002';assert read(n/'result.json')['passed']
positive=negative=0;comparisons=[]
for variant in ('existing','prompt'):
 current=n/variant;prior=old/variant;prepared=read(current/'prepared.json');assert prepared['passed'] and prepared['public_undo_redo_reopen']
 assert len(prepared['binding_refusals'])==3
 for path in sorted(current.glob('command-*.json')):
  row=read(path);before=read(prior/path.name);assert row['environment'] is None;assert row['exit_code']==before['exit_code']
  if row['exit_code']==0:
   report=json.loads(row['stdout']);previous=json.loads(before['stdout']);assert report['observations']['passed'] and report['logs']['dropped']==0
   for key in ('tick','frames','logs','observations','pcm_sha256'):assert report[key]==previous[key],(variant,path.name,key)
   positive+=1;comparisons.append({'variant':variant,'command':path.name,'fields':['tick','frames','logs','observations','pcm_sha256'],'exact':True})
  else:
   assert not row['stdout'] and row['stderr']=='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n';negative+=1
 for path in current.glob('*.pcm'):assert path.read_bytes()==(prior/path.name).read_bytes()
 assert prepared['cue_checks']==read(prior/'prepared.json')['cue_checks']
assert (positive,negative)==(28,2)
ref=read(n/'refusals/result.json');assert ref['passed'] and len(ref['cases'])==7 and ref['exact_diagnostics'] and ref['no_pcm_on_failure'] and ref['public_edit_undo_reopen']
for path in (n/'refusals').glob('command-*.json'):
 row=read(path);assert row['exit_code']!=0 and not row['stdout'] and 'Core Sample:' in row['stderr'];assert row['environment'] is None
assert not list((n/'refusals').glob('*.pcm'))
rel=read(o/'headless-relevance.json');assert h(rel['binary']['path'])==rel['binary']['sha256']
subprocess.run(['git','diff','--exit-code',rel['tested_head'],f['head'],'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml'],cwd=r,check=True)
assert set(git('diff','--name-only',rel['tested_head'],f['head'],'--','tools').splitlines())==set(read(o/'tool-diff.json')['paths'])
summary={'passed':True,'checks':len(checks),'frozen_inputs':len(f['files']),'producer_sources':len(source),'native_positives':positive,'native_restart_refusals':negative,'binding_refusals':7,'prior_native_reports_and_pcm_exact':True,'producer_records_rehashed':len(p['artifacts']),'producer_external_rehashed':len(p['external_artifacts']),'producer_shipping_runs':20,'current_shipping_runs':0,'new_build':False}
print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={};external=[]
for path in sorted(o.rglob('*')):
 if not path.is_file():continue
 b=path.read_bytes();s=sha(b);row={'original':str(path),'sha256':s,'bytes':len(b)}
 # Generated/copied authoring media and caches stay at immutable local paths.
 # All command streams, reports, PCM, source receipts and inventories are retained.
 if any(part in ('authoring','project') for part in path.relative_to(o).parts):external.append(row);continue
 records.append(row);payloads.setdefault(s,base64.b64encode(b).decode())
raw=json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode();encoded=gzip.compress(raw,mtime=0);parts=[]
for i,offset in enumerate(range(0,len(encoded),16*1024*1024)):
 path=d/f'artifacts.json.gz.part-{i:03}';path.write_bytes(encoded[offset:offset+16*1024*1024]);parts.append({'path':str(path.relative_to(r)),'sha256':h(path),'bytes':path.stat().st_size})
assert b''.join((r/x['path']).read_bytes() for x in parts)==encoded
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(o.parent/'core-result-hud-main192-run.py','run.py'),(review,'producer-review.json')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD'),'main_ancestor':main,'producer':producer,'summary':summary,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'checks':checks,'headless_relevance':rel,'sources':source,'exact_report_comparisons':comparisons,'producer_review':{'path':str((d/'producer-review.json').relative_to(r)),'sha256':h(d/'producer-review.json')},'archive':{'parts':parts,'gzip_sha256':sha(encoded),'gzip_bytes':len(encoded),'records':len(records),'payloads':len(payloads)},'external_files':external,'limits':['Current-main CPU public authoring/native Play only; original 20 source-free shipping processes remain producer-scoped','No current archive rebuild: exact engine/build-source equality supports immutable headless reuse; four disjoint shipping-proof helper/test changes qualified','No new production UI nodes, layout or localized style; optional correctness fixture reuses approved controls','No GPU/window/phone/performance or phase claim; physics session reset is not physics checkpoint save/restore','Generated authoring media/caches retained externally and fully inventoried; all current process outputs and PCM retained losslessly'],'review_state':'Independent current-main review pending'}
path=r/'docs/spikes/evidence/core-result-hud-main192.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(str(path),h(path),flush=True)
