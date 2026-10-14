from pathlib import Path
import collections, datetime, hashlib, json, os, re, shutil, subprocess, sys, tempfile
sys.dont_write_bytecode=True
C=Path('/Users/andreyyurlov/.codex/worktrees/native-agent-bridge/incant/.worktrees/0073-upscaling-review')
B=C.parents[1]
P=C/'handoffs/0073-upscaling-review/reconstruction_r5'
Q=Path('/Users/andreyyurlov/.codex/worktrees/owner-quality-repair/incant')
E=Path(__file__).resolve().parent
ES=Path.home()/'.codex/incant-evidence'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
blob=lambda ref,p:subprocess.check_output(['git','show',f'{ref}:{p}'],cwd=C)
FIRST='329bfc6a5d368a03681e1ab5843590901f17815e'
FINAL='51deb3848801f46c78ea85ca65fc330c5f60b758'
BASE='825ff047284e84252d00f786a7b14cb6ecb44293'
assert subprocess.run(['git','merge-base','--is-ancestor',FINAL,'HEAD'],cwd=C).returncode==0
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=Q,text=True).strip()==BASE
assert not subprocess.check_output(['git','status','--porcelain'],cwd=Q,text=True).strip()
changed=subprocess.check_output(['git','diff','--name-only','f099a898',FINAL],cwd=C,text=True).splitlines()
assert all(p.startswith('handoffs/0073-upscaling-review/reconstruction_r5/') and '/baseline/' not in p and not p.endswith('/brief.md') for p in changed)
for ref in [FIRST,FINAL]:assert 'Built-by: claude' in subprocess.check_output(['git','show','-s','--format=%B',ref],cwd=C,text=True)
# Rehash all supplied baseline files against immutable Git, not the private live tree alone.
manifest=json.loads((P/'baseline.json').read_text());base_rows=manifest['files'];assert len(base_rows)==61
for row in base_rows:
 p=row['path'];actual=P/'baseline'/p
 assert sha(actual)==row['sha256']==hashlib.sha256(blob(BASE,p)).hexdigest(),p
# Re-run only standard-library controls and patch generation in isolated authorized scratch.
with tempfile.TemporaryDirectory(prefix='incant-0073-r5-review-',dir='/tmp') as tmp:
 T=Path(tmp);shutil.copytree(P,T/'packet');T=T/'packet'
 for name,args in [('controls',['controls.py','--out','controls.json']),('inventory',['inventory.py','--checkout',str(Q)])]:
  r=subprocess.run([sys.executable,'-I','-B',*args],cwd=T,capture_output=True,text=True)
  (E/f'{name}.stdout').write_text(r.stdout);(E/f'{name}.stderr').write_text(r.stderr)
  assert r.returncode==0,(name,r.stderr)
 for name in ['controls.json','inventory.json','repair.patch']:assert (P/name).read_bytes()==(T/name).read_bytes(),name
ctl=json.loads((P/'controls.json').read_text());assert len(ctl['checks'])==22 and all(x['ok'] for x in ctl['checks'])
inv=json.loads((P/'inventory.json').read_text());assert len(inv['files'])==7
for row in inv['files']:assert sha(P/'src'/row['path'])==row['candidate_sha256']
assert sha(P/'repair.patch')=='8e226ad36180561868b899b54188fabfa2a4bcf692bc0ec43df5e7b1e6bc8253'
# Only comments changed in source after first producer; cooker pins follow the comments.
def tokens(s):return re.sub(r'\s+','',re.sub(r'//[^\n]*','',s))
for row in inv['files']:
 p='handoffs/0073-upscaling-review/reconstruction_r5/src/'+row['path']
 if 'incant_shader_cook' not in p:assert tokens(blob(FIRST,p).decode())==tokens(blob(FINAL,p).decode()),p
# First-turn protected sources; the top brief is the only separately authorized Astra mutation.
b1=json.loads((ES/'0073-reconstruction-r5-supervision-001/before.json').read_text())
for row in b1['protected_tracked']+b1['protected_untracked']:
 if row['path']=='handoffs/0073-upscaling-review/brief.md':continue
 assert sha(C/row['path'])==row['sha256'],row['path']
assert sha(C/'handoffs/0073-upscaling-review/brief.md')==sha(ES/'0073-reconstruction-r5-supervision-002/correction-brief.md')
# Exact completed process groups and enforced settings per ACP turn.
turns=[]
logpath=Path.home()/'.claude/projects/-Users-andreyyurlov--codex-worktrees-native-agent-bridge-incant--worktrees-0073-upscaling-review/88296197-2b09-41fa-b481-9f8ae415e266.jsonl'
for n in [1,2]:
 S=ES/f'0073-reconstruction-r5-supervision-{n:03d}';before=json.loads((S/'before.json').read_text());term=json.loads((S/'acp-terminal.json').read_text())
 assert term['exit']==0 and term['reaped'] and term['group_empty_after'] and not term['timed_out'] and not term['interrupted_by'] and not term['watchdog']
 assert term['auto_memory_environment']=={'CLAUDE_CODE_DISABLE_AUTO_MEMORY':'1'}
 assert sha(S/'acp.log')==term['stdout_sha256'] and sha(S/'acp-stderr.log')==term['stderr_sha256']
 assert 'Claude session verified: Opus 5.5; thinking: max' in (S/'acp.log').read_text()
 for fn,k in [(os.kill,'pid'),(os.killpg,'pgid')]:
  try:fn(term[k],0)
  except ProcessLookupError:pass
  else:raise AssertionError(('live process/group',term[k]))
 start=before['wall_epoch_seconds'];end=start+term['wall_elapsed_seconds'];rows=[];ids={}
 for line in logpath.open():
  j=json.loads(line);at=j.get('timestamp');content=j.get('message',{}).get('content',[])
  if not at or not isinstance(content,list):continue
  stamp=datetime.datetime.fromisoformat(at.replace('Z','+00:00')).timestamp()
  if not start<=stamp<=end:continue
  for q in content:
   if q.get('type')=='tool_use':
    row={'at':at,'id':q['id'],'name':q['name'],'input':q['input']};rows.append(row);ids[q['id']]=row
   if q.get('type')=='tool_result' and q.get('tool_use_id') in ids:
    c=q.get('content');ids[q['tool_use_id']]['result']={'is_error':q.get('is_error',False),'content_types':[r.get('type') for r in c] if isinstance(c,list) else ['text']}
 assert rows
 for row in rows:
  assert row['name'] in ['Bash','Write','Edit','Read'],row['name']
  if row['name'] in ['Write','Edit']:assert row['input']['file_path'].startswith(str(P)+'/'),row
 (S/'tool-metadata.json').write_text(json.dumps(rows,indent=2)+'\n')
 turns.append({'turn':n,'terminal':term,'supervisor_sha256':sha(S/'supervise.py'),'before_sha256':sha(S/'before.json'),'tool_metadata_sha256':sha(S/'tool-metadata.json'),'tools':dict(collections.Counter(x['name'] for x in rows))})
assert turns[1]['terminal']['total_original_wall_seconds']<1200 and turns[1]['terminal']['total_original_monotonic_seconds']<1200
files=[p for p in P.rglob('*') if p.is_file() and '/baseline/' not in str(p) and '/review-closure-' not in str(p)]
result={'scope':'Independent source/scalar/ACP integrity only; no Rust compile, cook, GPU, pixel review or appearance acceptance', 'first_producer':FIRST,'final_producer':FINAL,'base':BASE,'baseline_files':61,'source_files':7,'scalar_checks':22,'control_results_exact':True,'comment_only_correction':True,'protected_original_files':len(b1['protected_tracked'])+len(b1['protected_untracked'])-1,'candidate_unchanged':True,'turns':turns,'files':[{'path':str(p.relative_to(P)),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(files)]}
(E/'audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['turns','files']}))
