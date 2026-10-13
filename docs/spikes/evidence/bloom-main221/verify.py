"""Read-only exact main221 qualification; original evidence verifier is unchanged."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess,sys
ap=argparse.ArgumentParser();ap.add_argument('--verify-only',required=True,action='store_true');ap.parse_args()
R=Path(__file__).resolve().parents[4];D=Path(__file__).parent;Q=json.loads((D/'qualification.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
assert Q['frozen_inputs']==3109 and len(Q['exceptions'])==5
allowed={r['path']:r for r in Q['exceptions']};assert len(allowed)==5
assert set(allowed)=={'crates/incant_physics_snapshot/src/lib.rs','third_party/rapier2d/INCANT_PATCH.md','third_party/rapier2d/src/dynamics/ccd/ccd_solver.rs','third_party/rapier3d/INCANT_PATCH.md','third_party/rapier3d/src/dynamics/ccd/ccd_solver.rs'}
assert git('merge-base','--is-ancestor',Q['actual_main'],'HEAD')==b''
for row in Q['main_delta']:
 p=row['path'];assert p=='STATUS.md' or p.startswith(('docs/','crates/incant_physics_snapshot/','third_party/rapier2d/','third_party/rapier3d/'))
 if row['old_sha256'] is not None:assert h(git('show',Q['original_base']+':'+p))==row['old_sha256']
 assert h(git('show',Q['actual_main']+':'+p))==row['new_sha256']
assert git('diff','--name-only',Q['original_base'],Q['actual_main']).decode().splitlines()==[r['path'] for r in Q['main_delta']]
assert not git('diff','--name-only',Q['original_base'],Q['actual_main'],'--','Cargo.lock','Cargo.toml','crates/incant_render','crates/incant_render_gpu_host','crates/incant_game_player','crates/incant_shader_cook','tools','.github').strip()
for r in allowed.values():
 assert h(git('show',r['old_commit']+':'+r['path']))==r['old_sha256']
 assert h(git('show',r['new_commit']+':'+r['path']))==r['new_sha256']
 assert h((R/r['path']).read_bytes())==r['new_sha256']
env=os.environ.copy();env['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:/opt/homebrew/bin:'+env['PATH']
assert len(Q['graphs'])==3
for row in Q['graphs']:
 before=(R/row['before']).read_bytes();after=(R/row['after']).read_bytes();assert before==after and h(before)==row['sha256'] and row['exit']==0
 current=subprocess.check_output(row['command'],cwd=R,env=env,stderr=subprocess.PIPE);assert current==after
 assert b'incant_physics_snapshot' not in current and b'incant-checkpoint' not in current
# Every other original assertion is replayed literally. The one exact source-freeze
# loop is redirected only for the five pinned, reviewed, graph-irrelevant rows.
def qualified_row(row):
 p=row['path'];actual=(R/p).read_bytes()
 if p not in allowed:return h(actual)==row['sha256']
 q=allowed[p];return row['sha256']==q['old_sha256'] and h(actual)==q['new_sha256'] and h(git('show',q['old_commit']+':'+p))==row['sha256']
original=R/'docs/spikes/evidence/bloom-main219/verify.py';source=original.read_text();assert h(original.read_bytes())==Q['original_verifier_sha256']
needle="for row in F['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']"
assert source.count(needle)==1
source=source.replace(needle,"for row in F['files']:assert qualified_row(row),row['path']")
exec(compile(source,str(original),'exec'),{'__file__':str(original),'__name__':'__main__','qualified_row':qualified_row})
print(json.dumps({'qualified_main':Q['actual_main'],'original_frozen_inputs':3109,'exact_pinned_exceptions':len(allowed),'selected_all_target_graphs_unchanged':3,'fresh_build_or_gpu_claim':False}))
