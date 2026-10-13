"""Qualify only reviewed test/tool ancestry; replay preserved observable evidence read-only."""
from pathlib import Path
import gzip,hashlib,json,subprocess,sys
R=Path(__file__).resolve().parents[4];D=Path(__file__).resolve().parent
M=json.loads((D.parent/'core0069-main212.json').read_text());E=Path(M['proof_root'])
C=json.loads((D/'current-main-config.json').read_text());main=C['actual_main'];tested=M['source_head']
def sha(b):return hashlib.sha256(b).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
def h(p):return sha(p.read_bytes())
for ref in [main,tested,M['accepted_producer']]:subprocess.run(['git','merge-base','--is-ancestor',ref,'HEAD'],cwd=R,check=True)
paths=['crates','third_party','Cargo.toml','Cargo.lock','tools','.cargo','.github','sdk','schemas','samples','editor','rust-toolchain.toml']
changed=git('diff','--name-only',tested,'HEAD','--',*paths).decode().splitlines();assert set(changed)==set(C['allowed']),changed
rawmap=gzip.decompress((R/M['artifact_map']['path']).read_bytes());assert sha(rawmap)==M['artifact_map']['raw_sha256'];rows=json.loads(rawmap)
def retained(path):
 row=next(x for x in rows if x['original']==str(path));assert h(path)==row['sha256'];return row
freeze_path=E/'source-freeze.json';retained(freeze_path);freeze=json.loads(freeze_path.read_text());exemptions={}
for row in freeze['files']:
 p=row['path'];actual=h(R/p)
 if actual!=row['sha256']:
  assert p in C['allowed'],p
  assert sha(git('show',tested+':'+p))==row['sha256']
  assert actual==sha(git('show',main+':'+p))
  exemptions[p]=(row['sha256'],actual)
assert set(exemptions)==set(C['allowed'])
for p in changed:assert (R/p).read_bytes()==git('show',main+':'+p)
for review in C['reviews']:
 assert h(R/review['path'])==review['sha256']
audit=E/'audit.py';retained(audit);code=audit.read_text()
original="for row in freeze['files']:assert sha(R/row['path'])==row['sha256'],row['path']"
assert code.count(original)==1
code=code.replace(original,"for row in freeze['files']:\n actual=sha(R/row['path']);assert actual==row['sha256'] or qualified.get(row['path'])==(row['sha256'],actual),row['path']")
rootline="R=Path('/Users/andreyyurlov/.codex/worktrees/grid-navigation/incant');E=Path(__file__).resolve().parent"
assert code.count(rootline)==1;code=code.replace(rootline,'R=qualified_root;E=Path(__file__).resolve().parent')
exec(compile(code,str(audit),'exec'),{'__file__':str(audit),'qualified':exemptions,'qualified_root':R})
subprocess.run([sys.executable,str(D/'verify.py'),'--root',str(R)],check=True)
result={'passed':True,'tested_source':tested,'actual_main':main,'integration_head':git('rev-parse','HEAD').decode().strip(),'frozen_inputs':len(freeze['files']),'exemptions':[{'path':p,'tested_sha256':v[0],'current_sha256':v[1]} for p,v in sorted(exemptions.items())],'original_audit_sha256':h(audit),'source_review':C['reviews'],'scope':C['scope']}
print(json.dumps(result,indent=2))
if '--verify-only' not in sys.argv:(D/'current-main-qualification.json').write_text(json.dumps(result,indent=2)+'\n')
