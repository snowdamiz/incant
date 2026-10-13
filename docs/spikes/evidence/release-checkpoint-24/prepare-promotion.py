import concurrent.futures, datetime, hashlib, json, pathlib, subprocess
ROOT=pathlib.Path('/Users/andreyyurlov/.codex/worktrees/ci-platform-tools/incant')
OUT=pathlib.Path(__file__).parent
REPO='snowdamiz/incant'
manifest=json.loads((OUT/'manifest.json').read_text())
candidate=manifest['candidate']; selected=manifest['selected_main']; release=manifest['release']
terminal=json.loads((OUT/'terminal-state.json').read_text())
assert terminal['candidate']==candidate and terminal['counts']=={'SUCCESS':23,'MISSING':0}
assert terminal['workflows']=={'success':8,'missing':0}
def run(*cmd):
 return subprocess.check_output(cmd,cwd=ROOT,text=True).strip()
def save(n,v):
 (OUT/n).write_text(json.dumps(v,indent=2)+'\n')
def query(n,args):
 raw=subprocess.check_output(args,cwd=ROOT); (OUT/n).write_bytes(raw); return n,json.loads(raw)
run('git','fetch','origin','main','release','codex/release-candidate')
queries=[('pr-final.json',['gh','pr','view',str(manifest['pr']),'--repo',REPO,'--json','url,state,mergeable,mergeStateStatus,headRefOid,baseRefOid,headRefName,baseRefName,statusCheckRollup']),('runs-final.json',['gh','api',f'repos/{REPO}/actions/runs?head_sha={candidate}&event=push&per_page=100']),('protection-final.json',['gh','api',f'repos/{REPO}/branches/release/protection'])]
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as ex:
 data=dict(ex.map(lambda q:query(*q),queries))
pr=data['pr-final.json']; runs=data['runs-final.json']['workflow_runs']; protection=data['protection-final.json']
assert pr['headRefOid']==candidate and pr['baseRefOid']==release and pr['state']=='OPEN'
assert pr['mergeable']=='MERGEABLE' and pr['mergeStateStatus']=='CLEAN'
assert run('git','rev-parse','origin/release')==release
assert run('git','rev-parse','origin/codex/release-candidate')==candidate
required=sorted(manifest['required_contexts'])
policy=json.loads(run('git','show',f'{candidate}:.github/release-checks.json'))
release_policy=json.loads(run('git','show',f'{release}:.github/release-checks.json'))
assert policy==release_policy
contexts=policy if isinstance(policy,list) else policy['required_contexts']
assert sorted(contexts)==required and len(required)==23
assert sorted(protection['required_status_checks']['contexts'])==required
old=json.loads((OUT/'protection.json').read_text()); assert protection==old
checks={r['name']:r for r in pr['statusCheckRollup']}
assert len(checks)==len(pr['statusCheckRollup'])
for name in required:
 assert checks[name]['status']=='COMPLETED' and checks[name]['conclusion']=='SUCCESS',name
assert len(runs)==8
for r in runs:
 assert r['head_sha']==candidate and r['event']=='push' and r['head_branch']=='codex/release-candidate'
 assert r['status']=='completed' and r['conclusion']=='success' and r['run_attempt']==1
assert len({r['workflow_id'] for r in runs})==8
jobs=[]
for r in runs:
 n=f"jobs-{r['id']}.json"; _,d=query(n,['gh','api',f"repos/{REPO}/actions/runs/{r['id']}/jobs?per_page=100"])
 assert d['total_count']==len(d['jobs']); jobs.extend(d['jobs'])
assert len(jobs)==28
for j in jobs:
 assert j['head_sha']==candidate and j['status']=='completed'
 assert j['conclusion']=='success' or (j['name']=='deploy' and j['conclusion']=='skipped')
assert sum(j['conclusion']=='success' for j in jobs)==27
save('required-final.json',{'candidate':candidate,'required':[{**checks[n]} for n in required]})
for ref in [release,selected,manifest['previous_candidate']]:
 run('git','merge-base','--is-ancestor',ref,candidate)
tree=run('git','rev-parse',candidate+'^{tree}'); assert tree==manifest['tree']==run('git','rev-parse',selected+'^{tree}')
for name,args in [('release-to-selected.paths',['--name-only']),('release-to-selected.stat',['--stat'])]:
 (OUT/name).write_text(run('git','diff',*args,release,selected)+'\n')
record={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'checkpoint':manifest['checkpoint'],'candidate':candidate,'selected_main':selected,'tree':tree,'release_before':release,'main_now':run('git','rev-parse','origin/main'),'pr':pr['url'],'required_contexts':23,'all_required_success':True,'push_workflows':8,'all_push_workflows_success':True,'jobs':28,'successful_jobs':27,'nonrequired_skipped_jobs':['Website / deploy'],'candidate_exact_selected_tree':True,'candidate_contains_release_and_selected':True,'protection_unchanged':True,'prior_failed_checkpoints_preserved':True,'candidate_commit':run('git','show','-s','--format=fuller',candidate),'artifacts':{},'limitations':['Selected main226 only; later main227+ integrations remain outside this tested release tree.','Promotion is a tested integration baseline, not a Phase 1 or device/performance gate approval.']}
for p in sorted(OUT.iterdir()):
 if p.is_file() and p.name not in ['prepromotion.json']:
  record['artifacts'][p.name]=hashlib.sha256(p.read_bytes()).hexdigest()
save('prepromotion.json',record)
print(json.dumps({'prepromotion':str(OUT/'prepromotion.json'),'sha256':hashlib.sha256((OUT/'prepromotion.json').read_bytes()).hexdigest(),'required':23,'workflows':8,'jobs':28,'main':record['main_now']}))
