from pathlib import Path
import base64,gzip,hashlib,json,re,shutil,subprocess,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/gpu-timestamps-main200-001';d=r/'docs/spikes/evidence/gpu-timestamps-main200'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
main='7f0e61f245ad1c4a49e6f2dec908f6274f8f02dc';producer=git('rev-parse','40f138f7').decode().strip();f=read(o/'freeze.json');t=read(o/'terminal.json');assert t['exit']==0 and t['all_frozen_inputs_unchanged'] and t['head']==f['head']
for p,s in f['files'].items():assert h(r/p)==s,p
for head in [main,producer]:subprocess.run(['git','merge-base','--is-ancestor',head,'HEAD'],cwd=r,check=True)
checks=read(o/'results.json');assert len(checks)==10 and all(x['exit']==0 for x in checks)
counts={}
for row in checks:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
 if row['name'] in ['default-cpu','cooked-cpu']:
  text=Path(row['stdout']['path']).read_text()+Path(row['stderr']['path']).read_text();m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)
  counts[row['name']]={'passed':sum(int(x[0]) for x in m),'failed':sum(int(x[1]) for x in m),'ignored':sum(int(x[2]) for x in m),'nonempty_groups':sum(int(x[0])>0 for x in m)}
assert counts['default-cpu']=={'passed':91,'failed':0,'ignored':92,'nonempty_groups':6};assert counts['cooked-cpu']=={'passed':118,'failed':0,'ignored':92,'nonempty_groups':6}
p=r/'docs/spikes/evidence/gpu-timestamps-pass-resolve.json';assert h(p)=='17cccdc112948d7b1ff56f0ef7626a3b9645853ed7359a04e27f96871e3c52a7';prior=read(p)
records=prior.get('records',prior.get('artifacts'));assert isinstance(records,list)
for row in records:
 z=(r/row['retained']).read_bytes();b=gzip.decompress(z);assert sha(z)==row['retained_sha256'] and sha(b)==row['sha256'] and len(b)==row['bytes'];assert Path(row['original']).read_bytes()==b
base=git('merge-base',main,producer).decode().strip();paths=git('diff','--name-only',base,producer,'--','crates').decode().splitlines();sources=[]
for p in paths:sources.append({'path':p,'producer_sha256':sha(git('show',producer+':'+p)),'current_sha256':h(r/p)})
# Accepted dynamic resolution and its current mapping remain exact main bytes.
for p in git('ls-files','crates/incant_render/src/resolution','crates/incant_render_gpu_host').decode().splitlines():assert (r/p).read_bytes()==git('show',main+':'+p),p
assert (r/'Cargo.lock').read_bytes()==git('show',main+':Cargo.lock')
assert (r/'crates/incant_render/src/graph.rs').read_bytes()==git('show',producer+':crates/incant_render/src/graph.rs')
for p in git('ls-files','crates/incant_render/src/timing').decode().splitlines():assert (r/p).read_bytes()==git('show',producer+':'+p),p
summary={'checks':len(checks),'counts':counts,'frozen_inputs':len(f['files']),'producer_artifacts':len(records),'producer_crate_paths':len(sources),'main':main,'producer':producer,'gpu_execution':False,'private_candidate_only':True};print(json.dumps(summary),flush=True)
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(parents=True,exist_ok=True);artifacts=[];payloads={}
for path in sorted(o.iterdir()):
 if path.is_file():
  b=path.read_bytes();s=sha(b);artifacts.append({'original':str(path),'sha256':s,'bytes':len(b)});payloads.setdefault(s,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':artifacts,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024;(d/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(o/'freeze.json','freeze.json'),(o.parent/'gpu-timestamps-main200-run.py','run.py')]:shutil.copyfile(src,d/name)
m={'tested_head':f['head'],'main':main,'producer':producer,'summary':summary,'checks':checks,'sources':sources,'freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json')},'archive':{'path':str((d/'artifacts.json.gz').relative_to(r)),'sha256':sha(z),'bytes':len(z),'records':len(artifacts),'payloads':len(payloads)},'integration_seams_sha256':h(o/'integration-seams.patch'),'limits':['Private candidate for revised Claude GPU/appearance review; no publication or current GPU execution.','Default and cooked CPU counts overlap. All92 ignored native GPU cases were not executed.','Five wasm native-only timing dead-code warnings retained; browser timing admission still fails explicitly.','Original corrected producer Metal measurements are prior-source evidence; no device or performance gate.','Exactly original seven timestamp groups; motion, SSAO and bloom are absent.']}
p=r/'docs/spikes/evidence/gpu-timestamps-main200.json';p.write_text(json.dumps(m,indent=2)+'\n');print(p,h(p))
