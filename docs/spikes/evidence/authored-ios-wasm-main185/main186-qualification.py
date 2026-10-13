from pathlib import Path
import hashlib,json,subprocess,sys,datetime
root=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
source=root/'docs/spikes/evidence/authored-ios-wasm-main185/collect.py'
freeze=json.loads((source.parent/'freeze.json').read_text())
main='1c5658cf5d7eee8390b272394f38b8d9b49e1628'
head='43ed989bae5ec6607a78db00b51fb6ed5c8213a6'
sha=lambda b:hashlib.sha256(b).hexdigest()
git=lambda *args:subprocess.check_output(['git',*args],cwd=root)
assert git('rev-parse','HEAD').decode().strip()==head
subprocess.run(['git','merge-base','--is-ancestor',main,head],cwd=root,check=True)
assert git('diff',freeze['head'],head,'--','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml')==b''
exceptions=[]
for name,digest in freeze['files'].items():
 actual=sha((root/name).read_bytes())
 if actual==digest:continue
 assert name=='.github/workflows/development-wasm.yml'
 assert sha(git('show',freeze['head']+':'+name))==digest
 assert sha(git('show',main+':'+name))==actual
 exceptions.append({'path':name,'tested_sha256':digest,'current_sha256':actual,'current_source':main})
assert len(exceptions)==1
text=source.read_text();old="assert all(h(r/k)==v for k,v in f['files'].items())";assert text.count(old)==1
text=text.replace(old,"assert all(h(r/k)==v for k,v in f['files'].items() if k != '.github/workflows/development-wasm.yml')")
sys.argv=[str(source),'--verify-only']
try:
 exec(compile(text,str(source),'exec'),{'__file__':str(source)})
except SystemExit as end:
 assert end.code==0
else:
 raise AssertionError('Collector failed to stop at read-only boundary')
report={'at':datetime.datetime.now(datetime.UTC).isoformat(),'tested':freeze['head'],'current':head,'main':main,'original_collector_sha256':sha(source.read_bytes()),'original_manifest_sha256':sha((root/'docs/spikes/evidence/authored-ios-wasm-main185.json').read_bytes()),'frozen_inputs':len(freeze['files']),'exact_except':exceptions,'runtime_and_build_inputs_equal':True,'collector_replay_exit':0,'scope':'Only accepted main186 CI workflow differs; original frozen workflow verified from exact Git blob. Original collector and execution receipts unchanged.'}
p=Path.home()/'.codex/incant-integration-evidence/ios-wasm-main186-qualification.json';p.write_text(json.dumps(report,indent=2)+'\n');print(p,sha(p.read_bytes()))
