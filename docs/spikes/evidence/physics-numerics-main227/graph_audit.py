import subprocess,json,hashlib,tomllib,os,sys
from pathlib import Path
E=Path(__file__).parent
R=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
def run(name,args):
 p=subprocess.run(args,cwd=R,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 (E/(name+'.stdout')).write_bytes(p.stdout);(E/(name+'.stderr')).write_bytes(p.stderr)
 print(json.dumps({'name':name,'argv':args,'exit_code':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()}))
 if p.returncode: raise SystemExit(p.returncode)
 return p.stdout
for name in ['physics','physics-parity','physics-snapshot-boundary','native-physics']:
 run('probe-'+name,['./tools/cargo','metadata','--locked','--offline','--manifest-path','tools/probes/'+name+'/Cargo.toml','--format-version','1'])
b=run('runtime-default-tree',['./tools/cargo','tree','--locked','--offline','-p','incant_runtime','--edges','normal','--prefix','none','--format','{p}|{f}'])
assert b'incant_physics_snapshot' not in b and b'incant-checked-restore' not in b
base=tomllib.loads(subprocess.check_output(['git','show','d9d9c4e850ac18516d28c71f9a4e58fabc9db6d8:Cargo.lock'],cwd=R).decode())
cur=tomllib.loads((R/'Cargo.lock').read_text())
identity=lambda d: sorted((x['name'],x['version'],x.get('source'),x.get('checksum')) for x in d['package'])
assert identity(base)==identity(cur)
print(json.dumps({'locked_package_identities_unchanged':len(cur['package']),'runtime_default_has_no_admission':True}))

base_pkgs={(p['name'],p['version']):p for p in base['package']}
changes=[]
for p in cur['package']:
 old=base_pkgs[(p['name'],p['version'])]
 if old != p:
  assert p['name'] in ['rapier2d','rapier3d']
  assert sorted(p['dependencies'])==sorted(old['dependencies']+['incant_physics_snapshot'])
  q=dict(p); q['dependencies']=old['dependencies']; assert q==old
  changes.append(p['name'])
assert sorted(changes)==['rapier2d','rapier3d']
print(json.dumps({'only_added_local_edges':changes,'package_identities':len(cur['package'])}))
