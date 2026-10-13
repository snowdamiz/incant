from pathlib import Path
import json,hashlib,subprocess,os,re
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path.home()/'.codex/incant-integration-evidence/native-cpu-profiler-integration-001';p=Path.home()/'.codex/incant-evidence/native-cpu-profiler-checks-001';h=lambda b:hashlib.sha256(b).hexdigest()
e=dict(os.environ);e['PATH']='/Users/andreyyurlov/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin:'+e['PATH']
rows=[]
for old in json.loads((p/'selected-graphs.json').read_text()):
 mode=old['mode'] or 'default';cmd=old['command'];q=subprocess.run(cmd,cwd=r,env=e,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE);(o/f'graph-{mode}.stdout').write_text(q.stdout);(o/f'graph-{mode}.stderr').write_text(q.stderr);assert q.returncode==0
 name={'default':'normal-default.txt','summary':'normal-summary.txt','tracy':'normal-tracy.txt'}[mode];before=(p/name).read_text()
 norm=lambda x:sorted(re.sub(r'\(/Users/andreyyurlov/\.codex/worktrees/[^/]+/incant', '(CHECKOUT',line).strip() for line in x.splitlines())
 assert norm(q.stdout)==norm(before),(mode,'selected graph changed')
 rows.append({'mode':mode,'command':cmd,'exit':q.returncode,'current_stdout':{'path':str(o/f'graph-{mode}.stdout'),'sha256':h(q.stdout.encode())},'producer_stdout':{'path':str(p/name),'sha256':h(before.encode())},'exact_package_versions_and_features_after_only_checkout_path_normalization':True})
# Only incant_render gains an existing rustix dependency in the current lock.
before=subprocess.check_output(['git','show','bfe8ede2:Cargo.lock'],cwd=r);current=(r/'Cargo.lock').read_bytes();import tomllib
a=tomllib.loads(before.decode())['package'];b=tomllib.loads(current.decode())['package'];assert len(a)==len(b)
differences=[]
for x,y in zip(a,b):
 if x!=y:
  assert x['name']==y['name']=='incant_render';z=dict(x);z['dependencies']=sorted(x['dependencies']+['rustix']);assert z==y;differences.append({'package':'incant_render','only_added_dependency':'rustix'})
assert len(differences)==1
out={'passed':True,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip(),'graphs':rows,'lock_change':differences,'qualification':'Selected desktop target dependency versions/features unchanged; other changed build/tool sources still require explicit source relevance review. This does not claim new current-main shipping/capture execution.'};(o/'graph-relevance.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out),flush=True)
