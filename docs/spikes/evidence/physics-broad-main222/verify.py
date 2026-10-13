"""Read-only four-renderer-path qualification of exact broad-owner evidence."""
from pathlib import Path
import argparse,hashlib,json,subprocess,os
p=argparse.ArgumentParser();p.add_argument('--verify-only',required=True,action='store_true');p.parse_args()
R=Path(__file__).resolve().parents[4];D=Path(__file__).parent;Q=json.loads((D/'qualification.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
assert Q['frozen_inputs']==3125
A={r['path']:r for r in Q['exceptions']};assert set(A)=={'crates/incant_render/src/'+p for p in ['graph.rs','lib.rs','output.rs','temporal_joined_tests.rs']}
assert git('merge-base','--is-ancestor',Q['actual_main'],'HEAD')==b''
paths=git('diff','--name-only',Q['original_base'],Q['actual_main']).decode().splitlines();assert paths==Q['main_paths'] and len(paths)==42
assert all(p=='STATUS.md' or p.startswith(('docs/','crates/incant_render/')) for p in paths)
assert not git('diff','--name-only',Q['original_base'],Q['actual_main'],'--','Cargo.toml','Cargo.lock','crates/incant_physics_snapshot','third_party','tools','.github').strip()
for r in A.values():
 assert h(git('show',Q['original_base']+':'+r['path']))==r['old_sha256']
 assert h(git('show',Q['actual_main']+':'+r['path']))==r['new_sha256']==h((R/r['path']).read_bytes())
g=Q['normal_graph'];b=(R/g['path']).read_bytes();assert h(b)==g['sha256'] and len(b.splitlines())==1 and b.startswith(b'incant_physics_snapshot v0.0.1')
env=os.environ.copy();env['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+env['PATH'];assert subprocess.check_output(g['command'],cwd=R,env=env,stderr=subprocess.PIPE)==b
def qualified_row(row):
 p=row['path'];b=(R/p).read_bytes()
 return h(b)==row['sha256'] if p not in A else row['sha256']==A[p]['old_sha256'] and h(b)==A[p]['new_sha256']
original=R/'docs/spikes/evidence/physics-broad-main221/verify.py';source=original.read_text();assert h(original.read_bytes())==Q['original_verifier_sha256']
needle="for row in d['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']";assert source.count(needle)==1
# Original physics identity and every execution assertion remain unchanged.
source=source.replace(needle,"for row in d['files']:assert qualified_row(row),row['path']")
exec(compile(source,str(original),'exec'),{'__file__':str(original),'__name__':'__main__','qualified_row':qualified_row})
print(json.dumps({'qualified_main':Q['actual_main'],'original_frozen_inputs':3125,'exact_renderer_exceptions':4,'physics_vendor_lock_unchanged':True,'fresh_build_or_gpu_claim':False}))
