from pathlib import Path
import base64,gzip,hashlib,json,subprocess,sys,shutil,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/core-main182-001';d=r/'docs/spikes/evidence/core-main182'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='32482b67abcdeaf463ac1b6f4418e6ff99c8aaef';assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==6 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ('stdout','stderr'):assert h(c[s]['path'])==c[s]['sha256']
producer=read(r/'docs/spikes/evidence/core-authored-runtime-bindings.json');producer_tip='a9e26999e0763de7cbd625930595be8a36f3f460'
assert not git('diff',producer_tip,'HEAD','--','samples/core','handoffs/0060-core-sample-presentation')
for row in producer['source_freeze']['sources']:assert h(r/row['path'])==row['sha256']
for row in producer['artifacts']:
 raw=gzip.decompress((r/row['retained']).read_bytes());assert h(r/row['retained'])==row['retained_sha256'];assert sha(raw)==row['raw_sha256'] and len(raw)==row['raw_bytes'];assert h(row['original'])==row['raw_sha256']
for row in producer['binaries']:assert h(row['path'])==row['sha256']
changed=[]
for row in read(producer['engine_inputs']['path'])['rows']:
 current=h(r/row['path'])
 if current!=row['sha256']:
  blob=subprocess.check_output(['git','show',producer_tip+':'+row['path']],cwd=r);assert sha(blob)==row['sha256'];changed.append({**row,'current_sha256':current,'historical_git':producer_tip})
assert len(changed)==11,changed
# Replay the complete accepted producer output oracle, with exact Git fallback
# only for the eleven independently enumerated later Android source changes.
original=Path(producer['proof_root'])/'audit.py';code=original.read_text()
code=code.replace("REPO=Path('/Users/andreyyurlov/.codex/worktrees/color-mip-roles/incant')",'REPO=Path('+repr(str(r))+')')
old="for row in read(E/'build-inputs.json')['rows']:\n assert sha(REPO/row['path'])==row['sha256'],row['path']"
assert old in code;code=code.replace(old,"for row in read(E/'build-inputs.json')['rows']:\n assert sha(REPO/row['path'])==row['sha256'] or hashlib.sha256(subprocess.check_output(['git','show',"+repr(producer_tip)+"+':'+row['path']],cwd=REPO)).hexdigest()==row['sha256'],row['path']")
old="(E/'audit-result.json').write_text(json.dumps(output,indent=2)+'\\n');print(json.dumps(output,indent=2))";assert old in code;code=code.replace(old,"assert output==read(E/'audit-result.json');print(json.dumps(output,indent=2))")
ns={'subprocess':subprocess,'__file__':str(original)};exec(compile(code,str(original),'exec'),ns);historical=ns['output']
# Current matched headless proof: all baseline native logs, observations and PCM
# equal the preserved producer, with fresh public edits and owner processes.
n=o/'native';oldn=Path(producer['proof_root'])/'native-003';prepared=read(n/'prepared.json');assert prepared['passed'] and prepared['public_undo_redo_reopen']
positive=negative=0
for p in sorted(n.glob('command-*.json')):
 row=read(p)
 if row['exit_code']==0:
  q=json.loads(row['stdout']);assert q['observations']['passed'] and q['logs']['dropped']==0;positive+=1
 else:
  assert not row['stdout'] and row['stderr']=='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n';negative+=1
assert (positive,negative)==(14,1),(positive,negative)
for q in n.rglob('*.pcm'):
 prior=oldn/q.relative_to(n);assert prior.is_file() and q.read_bytes()==prior.read_bytes()
for name in prepared['runs']:
 a=read(n/(name+'.json'));b=read(oldn/(name+'.json'))
 for field in ('tick','frames','logs','observations','pcm_sha256'):assert a[field]==b[field],(name,field)
assert prepared['cue_checks']==read(oldn/'prepared.json')['cue_checks']
source_relevance=read(o/'headless-relevance.json');assert h(source_relevance['binary']['path'])==source_relevance['binary']['sha256']
subprocess.run(['git','diff','--exit-code','489168fc',f['head'],'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml','tools'],cwd=r,check=True)
summary={'passed':True,'current_checks':len(checks),'current_native_positive':positive,'current_native_refusal':negative,'producer_artifacts':len(producer['artifacts']),'producer_shipping_positive':historical['shipping_processes'],'producer_exact_reset_pairs':historical['restart_exact_pcm_pairs'],'frozen_inputs':len(f['files']),'historical_changed_inputs':len(changed)};print(json.dumps(summary))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for p in sorted(o.rglob('*')):
 if p.is_file():
  b=p.read_bytes();s=sha(b);records.append({'original':str(p),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0))
for src,name in [(o/'freeze.json','freeze.json'),(Path(__file__),'collect.py'),(original,'producer-assertions.py'),(o.parent/'core-main182-run.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'review_head':git('rev-parse','HEAD'),'summary':summary,'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'checks':checks,'headless_relevance':source_relevance,'producer_manifest':{'path':'docs/spikes/evidence/core-authored-runtime-bindings.json','sha256':h(r/'docs/spikes/evidence/core-authored-runtime-bindings.json'),'tip':producer_tip},'historical_source_differences':changed,'historical_proof_assertions':{'path':str(original),'sha256':h(original),'result':historical},'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(records),'payloads':len(payloads)},'limits':['Current CPU authoring and native readback only; prior ten shipping processes/four resets retain original main181 identity','No new appearance/native-window/device claim; historical0060 helper identity gap remains unresolved','No engine/controller/art changes; original full Claude0060 and locked-exit correction retained','One exact-file whitespace attribute preserves original font license bytes'],'review_state':'Final integration independent review pending'}
p=r/'docs/spikes/evidence/core-main182.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(p,h(p))
