from pathlib import Path
import json,hashlib,subprocess,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');d=r/'docs/spikes/evidence/core-live-hud-main190';read=lambda p:json.loads(Path(p).read_text());sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(d/'freeze.json');main='7b10ba000b2ca61a4cb36fdc710a89a370864101';allow={'tools/shipping.py','tools/probes/native-game.py','tools/probes/native-window.py'};added='tools/tests/test_player_probe_codegen.py';rows=[]
for p,v in f['files'].items():
 if h(r/p)!=v:
  assert p in allow,p;assert sha(subprocess.check_output(['git','show',f['head']+':'+p],cwd=r))==v
  latest=sha(subprocess.check_output(['git','show',main+':'+p],cwd=r));assert latest==h(r/p)
  rows.append({'path':p,'tested_sha256':v,'current_sha256':latest})
assert {x['path'] for x in rows}==allow
changed=git('diff','--name-only',f['head'],'HEAD','--','crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.github').splitlines();assert set(changed)==allow|{added},changed
assert h(r/added)==sha(subprocess.check_output(['git','show',main+':'+added],cwd=r))
# This collector reads the accepted immutable proof. Qualify exactly the three
# disjoint shipping helpers; do not overwrite historical receipts or sources.
p=d/'collect.py';code=p.read_text();old="assert all(h(r/k)==v for k,v in f['files'].items())";assert code.count(old)==1
code=code.replace(old,"assert all(h(r/k)==v or (k in qualified and qualified[k]==(v,h(r/k))) for k,v in f['files'].items())")
argv=sys.argv;sys.argv=[str(p),'--verify-only']
try:
 try:exec(compile(code,str(p),'exec'),{'__file__':str(p),'qualified':{x['path']:(x['tested_sha256'],x['current_sha256']) for x in rows}})
 except SystemExit as e:assert e.code==0
finally:sys.argv=argv
result={'passed':True,'tested_head':f['head'],'actual_main':main,'integration_head':git('rev-parse','HEAD'),'exemptions':rows,'added_test':{'path':added,'sha256':h(r/added)},'collector_original':{'path':str(p),'sha256':h(p)},'scope':'Only independently accepted shipping proof helpers and their new test differ; no crate/build/lock/SDK/schema/vendor input changed. Current native sample checks did not execute these helpers. No new runtime build or proof claimed.'}
print(json.dumps(result,indent=2))
if '--verify-only' not in argv:(d/'main191-qualification.json').write_text(json.dumps(result,indent=2)+'\n')
