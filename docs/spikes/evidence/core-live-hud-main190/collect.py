from pathlib import Path
import base64,gzip,hashlib,json,subprocess,sys,shutil,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/core-live-hud-main190-001';d=r/'docs/spikes/evidence/core-live-hud-main190'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==7 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ('stdout','stderr'):assert h(c[s]['path'])==c[s]['sha256']
safe='8b82ad4d12445edf9abbc71e264147544f87cce6';private='f517265a568724a5aa09ff1abc939c9368d35afb'
assert subprocess.run(['git','merge-base','--is-ancestor',private,'HEAD'],cwd=r).returncode==1
subprocess.run(['git','merge-base','--is-ancestor','0b18a0d7cd53c871aa0a6af395edd3108b56fa5f','HEAD'],cwd=r,check=True)
paths=git('diff-tree','--no-commit-id','--name-only','-r',safe).splitlines();source=[]
for p in paths:
 blob=subprocess.check_output(['git','show',safe+':'+p],cwd=r);assert sha(blob)==h(r/p),p
 source.append({'path':p,'sha256':sha(blob),'bytes':len(blob)})
review=Path.home()/'.codex/incant-reviews/claude0065-final-20261013.json';assert h(review)=='8c29be8dcd748265b8c366391b9c683b5bff4184ec9b1114048ad00731f98260'
p=read(review)
for row in p['packet_records']:assert h(r/row['path'])==row['sha256']
assert h(r/'handoffs/0065-core-live-hud/result.md')==p['result_sha256']
# Read only the public sanitized packet. No private ledger/payload enters this collector.
allowed={'CARGO_TARGET_DIR','INCANT_NATIVE_PROJECT'};envrows=[]
def envcheck(obj):
 if isinstance(obj,dict):
  for key,value in obj.items():
   if key in ('env','environment') and isinstance(value,dict):
    assert set(value)<=allowed,set(value);envrows.append(sorted(value))
   envcheck(value)
 elif isinstance(obj,list):
  for v in obj:envcheck(v)
for path in (r/'handoffs/0065-core-live-hud/evidence').glob('*.json'):envcheck(read(path))
assert envrows
n=o/'native';oldn=Path.home()/'.codex/incant-integration-evidence/core-main182-001/native';prepared=read(n/'prepared.json');assert prepared['passed'] and prepared['public_undo_redo_reopen']
positive=negative=0
for pth in sorted(n.glob('command-*.json')):
 row=read(pth);assert row['environment'] is None
 if row['exit_code']==0:
  q=json.loads(row['stdout']);assert q['observations']['passed'] and q['logs']['dropped']==0;positive+=1
 else:
  assert not row['stdout'] and row['stderr']=='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n';negative+=1
assert (positive,negative)==(14,1),(positive,negative)
for q in n.rglob('*.pcm'):
 prior=oldn/q.relative_to(n);assert prior.is_file() and q.read_bytes()==prior.read_bytes(),str(q)
for name in prepared['runs']:
 a=read(n/(name+'.json'));b=read(oldn/(name+'.json'))
 for field in ('tick','frames','logs','observations','pcm_sha256'):assert a[field]==b[field],(name,field)
assert prepared['cue_checks']==read(oldn/'prepared.json')['cue_checks']
assert len(prepared['binding_refusals'])==3
fonts=read(o/'font-coverage.json');assert set(fonts['locales'])=={'en','ar','zh-Hans','en-XA'}
assert all(not x['missing'] for x in fonts['locales'].values())
assert '8d78a4c412fca7fd980e12c4ccf958bc13d21d8dd016752808b4b83116e62af0' in fonts['fonts'].values()
rel=read(o/'headless-relevance.json');assert h(rel['binary']['path'])==rel['binary']['sha256']
subprocess.run(['git','diff','--exit-code',rel['tested_head'],f['head'],'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml','tools'],cwd=r,check=True)
summary={'passed':True,'current_checks':len(checks),'current_native_positive':positive,'current_native_refusal':negative,'public_binding_refusals':3,'locales':4,'frozen_inputs':len(f['files']),'safe_producer_files':len(source),'sanitized_environment_rows':len(envrows),'private_ancestry_excluded':True,'baseline_gameplay_and_pcm_exact':True}
print(json.dumps(summary))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for pth in sorted(o.rglob('*')):
 if pth.is_file():
  b=pth.read_bytes();s=sha(b);records.append({'original':str(pth),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
raw=json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode();encoded=gzip.compress(raw,mtime=0);parts=[]
for i,offset in enumerate(range(0,len(encoded),16*1024*1024)):
 pth=d/f'artifacts.json.gz.part-{i:03}';pth.write_bytes(encoded[offset:offset+16*1024*1024]);parts.append({'path':str(pth.relative_to(r)),'sha256':h(pth),'bytes':pth.stat().st_size})
assert b''.join((r/x['path']).read_bytes() for x in parts)==encoded
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(o.parent/'core-live-hud-main190-run.py','run.py'),(o.parent/'core-live-hud-fonts.py','font-check.py'),(review,'producer-review.json')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD'),'summary':summary,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'checks':checks,'headless_relevance':rel,'safe_producer':safe,'accepted_source_files':source,'producer_review':{'path':str((d/'producer-review.json').relative_to(r)),'sha256':h(d/'producer-review.json')},'archive':{'parts':parts,'gzip_sha256':sha(encoded),'gzip_bytes':len(encoded),'records':len(records),'payloads':len(payloads)},'limits':['Current CPU authoring/native readback/font coverage only; no current GPU, native window or source-free shipping of changed HUD','Claude producer reports result-title and narrow contrast defects; full visual acceptance remains open','Original0060 offscreen helper identity gap remains unresolved; prompt/stale objective follow-up separate','Only clean sanitized8b82 producer ancestry retained; private original f517 and raw environment ledger excluded','Immutable current-source headless reused by exact implementation/build input equality; no redundant build claim'],'review_state':'Final integration independent review pending'}
pth=r/'docs/spikes/evidence/core-live-hud-main190.json';pth.write_text(json.dumps(manifest,indent=2)+'\n');print(str(pth),h(pth))
