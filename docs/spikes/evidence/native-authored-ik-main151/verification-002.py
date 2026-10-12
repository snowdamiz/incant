from pathlib import Path
import json,hashlib,re,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/authored-ik-main151-001');p=o/'public-export';h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
f=json.loads((o/'freeze.json').read_text());assert all(h(r/n)==v for n,v in f['files'].items())
c=json.loads((o/'results.json').read_text());assert len(c)==17 and all(x['exit']==0 for x in c)
for x in c:assert h(o/(x['name']+'.log'))==x['sha256']
a=json.loads((p/'result.json').read_text());assert a['passed'] and a['authoring_deleted'];assert not (p/'authoring').exists();assert not list((p/'package/symbols').glob('*.js.map'))
for n,v in a['package_files'].items():
 q=p/'package'/n;assert q.is_file() and h(q)==v['sha256'] and q.stat().st_size==v['bytes'],n
player=p/'package/player/incant_game_player';assert h(player)==a['player_sha256'] and player.stat().st_size==a['player_bytes'];assert a['export']['build_storage_removed_before_startup'];assert len(a['runs'])==4 and len(a['refusals'])==7
reports={}
for v in a['runs']:
 n=v['name'];reports[n]=json.loads((p/(n+'.json')).read_text());assert reports[n]==v['report'];assert reports[n]['observations']['passed'] and not reports[n]['render_input']['gpu_submitted'];assert h(p/(n+'.save'))==v['save_sha256'];assert h(p/(n+'.pcm'))==v['pcm_sha256'];receipt=json.loads((p/(n+'-receipt.json')).read_text());assert receipt['returncode']==0 and json.loads(receipt['stdout'])==reports[n]
for n in ['resumed','repeat']:
 assert (p/(n+'.save')).read_bytes()==(p/'baseline.save').read_bytes()
 assert (p/'middle.pcm').read_bytes()+(p/(n+'.pcm')).read_bytes()==(p/'baseline.pcm').read_bytes()
 assert reports['middle']['logs']['entries']+reports[n]['logs']['entries']==reports['baseline']['logs']['entries']
 assert reports[n]['render_input']==reports['baseline']['render_input']
assert reports['baseline']['logs']==a['authoring']['native_play_logs']
expected={'corrupt':'Error: Checkpoint(Integrity)','wrong-program':'Error: Checkpoint(ProgramMismatch)','wrong-constraint':'Error: Checkpoint(World(Scene(Invalid("IK state binding"))))','wrong-chain':'Error: Checkpoint(World(Scene(Invalid("IK state binding"))))','oversized':'Error: "runtime input size limit"','assertion':'Error: "checkpoint withheld: native observation assertions failed"'}
for v in a['refusals']:
 argv=v['argv'];pcm=Path(argv[argv.index('--output')+1]);n=pcm.stem;assert v['returncode']==1;assert (not json.loads(v['stdout'])['observations']['passed']) if n=='assertion' else (not v['stdout']);assert v['stderr'].strip()==v['matched_diagnostic'];assert v['matched_diagnostic']==expected[n] if n in expected else re.fullmatch(r'Error: Os \{ code: \d+, kind: AlreadyExists, message: "[^"\n]*" \}',v['matched_diagnostic']);assert n=='no-clobber' or not pcm.exists()
assert (p/'existing.save').read_bytes()==b'preserve existing';assert not list(p.glob('.incant-checkpoint-*'))
counts=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (o/'affected-tests.log').read_text())];totals=[sum(x[i] for x in counts) for i in range(3)];assert totals==[823,0,82],totals
result={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'passed':True,'frozen_inputs':len(f['files']),'commands':len(c),'cpu':{'passed':823,'failed':0,'ignored_gpu':82,'groups':len(counts)},'source_head':f['head'],'actual_package_files':len(a['package_files']),'fresh_processes':4,'exact_refusals':7,'save_and_split_pcm_equal':True,'player_sha256':a['player_sha256'],'player_bytes':a['player_bytes'],'result_sha256':h(p/'result.json'),'headless_sha256':h(o/'incant_headless'),'source_maps_absent':True,'authoring_absent':True,'limits':a['limits']};(o/'verified.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
