from pathlib import Path
import hashlib,json,re,runpy,subprocess
root=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
out=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/animation-trees-main147-001')
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
j=lambda p:json.loads(p.read_text())
results=j(out/'results.json');assert len(results)==18 and all(x['exit']==0 for x in results)
f=j(out/'freeze.json');assert all(h(root/p)==v for p,v in f['files'].items())
for x in results:assert h(out/(x['name']+'.log'))==x['sha256']
counts={}
for name,path in [('actual-host',out/'conformance/conformance-0.log'),('affected',out/'affected-tests.log')]:
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',path.read_text());assert rows
 counts[name]={'groups':len(rows),'passed':sum(int(a) for a,b,c in rows),'failed':sum(int(b) for a,b,c in rows),'ignored':sum(int(c) for a,b,c in rows)}
 assert counts[name]['failed']==0
conf=j(out/'conformance/conformance-result.json');assert conf['passed'] and all(x['exit_code']==0 for x in conf['commands'])
for x in conf['commands']:assert h(out/'conformance'/x['log'])==x['sha256']
records={}
def command_streams(p):
 cs=j(p/'commands.json')
 for c in cs:
  if 'stdout' not in c or 'stderr' not in c:
   assert p.name=='quickjs-export' and c['argv'][:2]==[str(root/'tools/cargo'),'build'] and c['returncode']==0
   assert 'stdout' not in c and 'stderr' not in c and (p/'package/build.log').is_file()
  else:
   for k in ['stdout','stderr']:assert h(p/c[k]['file'])==c[k]['sha256']
 return cs
p=out/'quickjs-export';q=j(p/'result.json');assert q['passed'] and q['authoring_deleted'] and q['export']['build_storage_removed_before_startup']
assert not (p/'authoring').exists() and not list((p/'package').rglob('*.js.map'))
for path,digest in q['package_files'].items():assert h(p/'package/player'/path)==digest
assert h(p/'package/player/incant_game_player')==q['export']['player_sha256']
for path in q['removed_source_maps']:assert not (p/'package'/path).exists()
probe=runpy.run_path(str(root/'tools/probes/native-animation-tree-export.py'))
cs=command_streams(p);pcs=[c for c in cs if c['argv'][0]==str(p/'package/player/incant_game_player') and '--package' in c['argv']]
assert len(pcs)==17 and sum(c['returncode']==0 for c in pcs)==15
baseline=j(p/'baseline.json')
for r in q['runs']:
 name=r['name'];report=j(p/(name+'.json'));assert h(p/(name+'.json'))==r['report_sha256']
 assert h(p/(name+'.pcm'))==r['pcm_sha256'] and h(p/(name+'.save'))==r['save_sha256']
 c=next(c for c in pcs if c['argv'][c['argv'].index('--output')+1]==str(p/(name+'.pcm')))
 assert c['returncode']==0 and json.loads((p/c['stdout']['file']).read_text())==report
 probe['check_logs'](report['logs'],r['start'],r['start']+r['ticks'])
 assert report['observations']['passed'] and not report['render_input']['gpu_submitted']
 assert probe['BASE']['checkpoint_version'](p/(name+'.save'))==6
for name in ['repeat-0','repeat-1']:
 assert j(p/(name+'.json'))==baseline
 assert (p/(name+'.pcm')).read_bytes()==(p/'baseline.pcm').read_bytes()
 assert (p/(name+'.save')).read_bytes()==(p/'baseline.save').read_bytes()
for name in ['eight-to-four','paused','rest-completed-paused','reverse','second-state','zero-speed']:
 assert (p/(name+'.pcm')).read_bytes()+(p/(name+'-continued.pcm')).read_bytes()==(p/'baseline.pcm').read_bytes()
 assert (p/(name+'-continued.save')).read_bytes()==(p/'baseline.save').read_bytes()
 assert j(p/(name+'.json'))['logs']['entries']+j(p/(name+'-continued.json'))['logs']['entries']==baseline['logs']['entries']
for name,diagnostic in [('corrupt','Error: Checkpoint(Integrity)'),('wrong-observation','Error: "checkpoint withheld: native observation assertions failed"')]:
 c=next(c for c in pcs if c['argv'][c['argv'].index('--output')+1]==str(p/(name+'.pcm')))
 assert c['returncode']==1 and (p/c['stderr']['file']).read_text().strip()==diagnostic
 assert not (p/(name+'.pcm')).exists() and not (p/(name+'.save')).exists()
records['quickjs']={'actual_package_files':q['package_files'],'player_sha256':q['export']['player_sha256'],'commands':len(cs),'successes':15,'full_runs':3,'continuation_pairs':6,'exact_refusals':2,'independent_full_log_check':True,'repeat_and_split_pcm_exact':True,'final_saves_equal':True}
p=out/'wasm-export';w=j(p/'result.json');assert w['passed'] and w['project_cache_build_maps_deleted_before_runs']
for path,digest in w['package_hashes'].items():assert h(p/path)==digest
assert h(p/'package/player/incant_game_player')==w['player_sha256']
for path in w['removed_private_files']:assert not (p/path).exists()
assert not list((p/'package').rglob('*.map'))
cs=command_streams(p);probe=runpy.run_path(str(root/'tools/probes/native-wasm-animation-pose-export.py'));rows=j(p/'oracle.json');dev=j(p/'development.json')
def require(value,message):assert value,message
probe['check_logs'](dev['logs'],rows,require)
assert h(p/'oracle.scene')==h(p/'package/player/data/world.scene')
for i in range(3):
 name=f'run-{i}';report=j(p/(name+'.json'))
 probe['check_logs'](report['logs'],rows,require)
 assert report['observations']['passed'] and report['observations']==dev['observations'] and report['logs']==dev['logs']
 assert report['pcm_sha256']==dev['pcm_sha256'] and (p/(name+'.pcm')).read_bytes()==(p/'development.pcm').read_bytes()
 c=next(c for c in cs if '--output' in c['argv'] and c['argv'][c['argv'].index('--output')+1]==str(p/(name+'.pcm')))
 assert c['exit_code']==0 and json.loads((p/c['stdout']['file']).read_text())==report
for name in w['refusals']:
 r=j(p/(name+'.json'))
 expected=probe['DEV_ERRORS'][name[4:]] if name.startswith('dev-') else (probe['PACKAGE_ERRORS'][name[8:]] if name.startswith('package-') else 'Error: Checkpoint(Unsupported("static module state ABI"))')
 assert r['exit_code']==1 and r['output_absent'] and not r['stdout'] and r['stderr'].strip()==r['expected_stderr']==expected
 c=next(c for c in cs if c['argv']==r['command'])
 assert c['exit_code']==1 and (p/c['stdout']['file']).read_text()==r['stdout'] and (p/c['stderr']['file']).read_text()==r['stderr']
 assert not (p/(name+'.pcm')).exists()
assert not (p/'checkpoint-refused.bin').exists()
records['wasm']={'actual_package_files':w['package_hashes'],'player_sha256':w['player_sha256'],'commands':len(cs),'fresh_shipping_runs':3,'full_scalars_per_run':3420,'selected_vectors_per_run':228,'exact_refusals':len(w['refusals']),'independent_native_oracle_and_full_log_check':True,'actual_pcm_equals_development':True,'removed_private_files':len(w['removed_private_files'])}
for name in ['quickjs-export','wasm-export']:
 p=out/name
 records['quickjs' if name.startswith('quickjs') else 'wasm']['actual_output_files']={str(x.relative_to(p)):{'sha256':h(x),'bytes':x.stat().st_size} for x in p.iterdir() if x.is_file() and x.suffix in ['.pcm','.save']}
record={'frozen_head':f['head'],'frozen_input_count':len(f['files']),'all_frozen_inputs_unchanged':True,'zero_exit_commands':len(results),'tests':counts,'conformance_passed':True,'public_proofs':records,'limitations':['CPU-only local macOS integrated proof, no GPU/native-window/device/performance claim.','Original producer evidence remains separate; these are new current-main/current-lock builds.','Module checkpoint state and nonzero transitions involving tree states remain unsupported.']}
(out/'proof-verification.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'tests':counts,'record_sha256':h(out/'proof-verification.json')}))
