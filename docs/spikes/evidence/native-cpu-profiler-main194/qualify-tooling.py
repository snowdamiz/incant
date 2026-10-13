from pathlib import Path
import hashlib,json,subprocess,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');d=r/'docs/spikes/evidence/native-cpu-profiler-main194'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
old='4c4a519a3d67f74a209cd73df7186b3b19f786d1'
f=json.loads((d/'freeze.json').read_text())
allowed={'.gitattributes','tools/probes/native-physics/Cargo.lock','tools/tests/test_credential_impact.py'}
assert set(git('diff','--name-only',old,'HEAD','--','.gitattributes','tools','crates','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml').decode().splitlines())==allowed
rows=[]
for p in sorted(allowed):
 before=git('show',old+':'+p);current=(r/p).read_bytes()
 rows.append({'path':p,'accepted_sha256':sha(before),'current_sha256':sha(current),'frozen_sha256':f['files'][p]})
attrs=git('show',old+':.gitattributes')
assert (r/'.gitattributes').read_bytes()==attrs+b'# Preserve the leading space on the retained source-relevance diff context lines.\ndocs/spikes/evidence/native-cpu-profiler-main194/source-relevance.patch -text whitespace=-blank-at-eol\n'
qualified={x['path']:(x['frozen_sha256'],x['current_sha256']) for x in rows}
for p,s in f['files'].items():assert h(r/p)==s or (p in qualified and qualified[p]==(s,h(r/p))),p
path=d/'collect.py';code=path.read_text()
before="assert h(r/p)==s,p";assert code.count(before)==1
code=code.replace(before,"assert h(r/p)==s or (p in qualified and qualified[p]==(s,h(r/p))),p")
before="assert git('diff',freeze['head'],'HEAD','--','.gitattributes')==(o/'raw-log-attributes.patch').read_bytes()";assert code.count(before)==1
code=code.replace(before,"assert git('diff',freeze['head'],accepted,'--','.gitattributes')==(o/'raw-log-attributes.patch').read_bytes()")
before="assert not git('diff','--name-only',freeze['head'],'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml').strip()";assert code.count(before)==1
code=code.replace(before,"assert set(git('diff','--name-only',freeze['head'],'HEAD','--','crates','tools','Cargo.toml','Cargo.lock','sdk','schemas','third_party','.cargo','rust-toolchain.toml').decode().splitlines())==set(qualified)-{'.gitattributes'}")
argv=sys.argv;sys.argv=[str(path),'--verify-only']
try:
 try:exec(compile(code,str(path),'exec'),{'__file__':str(path),'qualified':qualified,'accepted':old})
 except SystemExit as error:assert error.code==0
finally:sys.argv=argv
result={'passed':True,'accepted_head':old,'current_head':git('rev-parse','HEAD').decode().strip(),'qualified_inputs':rows,'original_collector_sha256':h(path),'scope':'Only isolated probe lock, actual-manifest tooling test and exact evidence whitespace metadata changed. Runtime/exporter/root-lock/profiler/selected target inputs unchanged. All original tests and producer process evidence retain their original identities; no new Rust or shipping run claimed.'}
print(json.dumps(result,indent=2))
if '--verify-only' not in argv:(d/'tooling-qualification.json').write_text(json.dumps(result,indent=2)+'\n')
