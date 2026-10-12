from pathlib import Path
import json,hashlib,re,math,datetime
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');o=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/wasm-ik-main152-001');p=o/'public-export';h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
f=json.loads((o/'freeze.json').read_text());assert all(h(r/n)==v for n,v in f['files'].items())
s=json.loads((o/'accepted-source-comparison.json').read_text());assert len(s)==29 and all(h(r/n)==v for n,v in s.items())
c=json.loads((o/'results.json').read_text());assert len(c)==13 and all(x['exit']==0 for x in c)
for x in c:assert h(o/(x['name']+'.log'))==x['sha256']
co=json.loads((o/'conformance/conformance-result.json').read_text());assert co['passed'] and len(co['commands'])==3 and all(x['exit_code']==0 for x in co['commands'])
for x in co['commands']:assert h(o/'conformance'/x['log'])==x['sha256']
counts=[tuple(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', (o/'conformance/conformance-0.log').read_text())];assert [sum(x[i] for x in counts) for i in range(3)]==[418,0,0];assert len(counts)==74
j=json.loads((p/'result.json').read_text());assert j['passed'] and j['fresh_ordinary_package_runs']==3 and len(j['refusals'])==15
for n,sha in j['package_hashes'].items():assert h(p/n)==sha,n
for n in j['removed_private_files']:assert not (p/n).exists(),n
for n in ['authoring','guest-build','private-builds','trusted-tools']:assert not (p/n).exists(),n
assert not list((p/'package/symbols').glob('*.map'))
for n,sha in j['proof_source_hashes'].items():assert h(r/n)==sha
commands=json.loads((p/'commands.json').read_text());assert h(p/'commands.json')==j['commands_sha256']
for x in commands:
 for stream in ['stdout','stderr']:
  if stream in x:assert h(p/x[stream]['file'])==x[stream]['sha256']
dev=json.loads((p/'development.json').read_text());oracle=json.loads((p/'oracle.json').read_text());assert len(oracle)==20
for i in range(3):
 a=json.loads((p/f'run-{i}.json').read_text());assert a['observations']['passed'] and len(a['observations']['checks'])==252
 for key in ['observations','logs','tick','frames','pcm_sha256']:assert a[key]==dev[key],key
 assert (p/f'run-{i}.pcm').read_bytes()==(p/'development.pcm').read_bytes()
 entries=a['logs']['entries'];assert len(entries)==100 and a['logs']['dropped']==0;scalar_count=0
 for offset,e in enumerate(entries):
  v=json.loads(e['message']);row=oracle[offset//5];index=offset%5;assert e['tick']==v['tick']==row['tick'] and e['level']=='info'
  if index>=3:
   assert v['entity']==index-2 and v['constraints']==row['ik'][(index-3)*6:(index-2)*6];scalar_count+=18
  else:
   assert v['entity']==index+1
   for key,size in [('samples',16),('metadata',3)]:
    assert len(v[key])==size
    expected=row[key][index*size:(index+1)*size]
    for aa,bb in zip(v[key],expected):
     assert len(aa)==len(bb)==3
     for aa,bb in zip(aa,bb):assert type(aa) in (float,int) and math.isfinite(aa) and abs(aa-bb)<=1e-12;scalar_count+=1
 assert scalar_count==4140
for n in j['refusals']:
 v=json.loads((p/(n+'.json')).read_text());assert v['exit_code']==1 and not v['stdout'] and v['stderr'].strip()==v['expected_stderr'] and v['output_absent'],n
 argv=v['command'];assert not Path(argv[argv.index('--pcm-output' if n.startswith('dev-') else '--output')+1]).exists()
assert not (p/'checkpoint-refused.bin').exists()
player=p/'package/player/incant_game_player';assert h(player)==j['player_sha256']
result={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'passed':True,'tested_head':f['head'],'frozen_inputs':len(f['files']),'accepted_sources':len(s),'commands':len(c),'conformance':{'passed':418,'failed':0,'ignored':0,'groups':74,'development_processes':3,'prepared_files_verified_before_cleanup':co['verified_prepared_files']},'package':{'files':len(j['package_hashes']),'player_sha256':h(player),'player_bytes':player.stat().st_size,'fresh_runs':3,'ticks_each':20,'scalar_comparisons_each':4140,'public_vector_checks_each':252,'exact_refusals':15,'pcm_log_development_equality':True,'removed_files':len(j['removed_private_files'])},'limits':j['limits']};(o/'verified.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
