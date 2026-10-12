from pathlib import Path
import datetime, hashlib, json, re, runpy, subprocess
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
b=Path('/Users/andreyyurlov/.codex/incant-integration-evidence')
o=b/'compound-main155-003';prior=b/'compound-main154-002';prep=b/'compound-main153-preparation'
h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();j=lambda p:json.loads(p.read_text())
f=j(o/'freeze.json');assert all(h(r/p)==v for p,v in f['files'].items())
assert all(h(r/p)==v for p,v in j(prior/'freeze.json')['files'].items())
checks=j(prior/'results.json')[:2]+j(o/'results.json');assert len(checks)==15 and all(x['exit']==0 for x in checks)
for check in checks:
 p=(prior if check['name'] in ['tests','clippy'] else o)/(check['name']+'.log');assert h(p)==check['sha256']
def counts(p):
 rows=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',p.read_text());assert rows
 return dict(zip(['passed','failed','ignored'],[sum(int(x[i]) for x in rows) for i in range(3)]),groups=len(rows))
tests=counts(prior/'tests.log');assert tests=={'passed':766,'failed':0,'ignored':82,'groups':115}
c=j(o/'conformance/conformance-result.json');assert c['passed'] and all(x['exit_code']==0 for x in c['commands'])
for x in c['commands']:assert h(o/'conformance'/x['log'])==x['sha256']
conformance=counts(o/'conformance/conformance-0.log')
p=o/'public-export';res=j(p/'result.json');assert res['passed'] and res['authoring_deleted'] and res['fresh_full_processes']==3 and res['continuation_pairs']==10
assert len(res['runs'])==23 and not (p/'authoring').exists()
assert res['vector_assertions_per_full_run']==224 and res['full_pose_log_ticks']==56
assert res['export']['build_storage_removed_before_startup']
for name,expected in res['package_files'].items():assert h(p/'package/player'/name)==expected
for name in res['removed_source_maps']:assert not (p/'package'/name).exists()
assert not list((p/'package').rglob('*.js.map'))
probe=runpy.run_path(str(r/'tools/probes/native-compound-animation-export.py'))
baseline=j(p/'baseline.json');pcm=(p/'baseline.pcm').read_bytes();save=(p/'baseline.save').read_bytes()
for row in res['runs']:
 name=row['name'];report=j(p/(name+'.json'))
 assert h(p/(name+'.json'))==row['report_sha256'] and h(p/(name+'.pcm'))==row['pcm_sha256'] and h(p/(name+'.save'))==row['save_sha256']
 probe['check_logs'](report['logs'],row['start'],row['start']+row['ticks'])
 assert report['observations']['passed'] and report['tick']==row['start']+row['ticks']
 assert len(report['observations']['checks'])==4*row['ticks'] and all(x['passed'] for x in report['observations']['checks'])
 if name.startswith('repeat-'):
  assert report==baseline and (p/(name+'.pcm')).read_bytes()==pcm and (p/(name+'.save')).read_bytes()==save
for split,label in probe['SPLITS']:
 first=j(p/(label+'.json'));tail=j(p/(label+'-continued.json'))
 assert (p/(label+'-continued.save')).read_bytes()==save
 assert (p/(label+'.pcm')).read_bytes()+(p/(label+'-continued.pcm')).read_bytes()==pcm
 assert first['logs']['entries']+tail['logs']['entries']==baseline['logs']['entries']
 assert tail['render_input']==baseline['render_input']
commands=j(p/'commands.json');assert h(p/'commands.json')==res['commands_sha256']
for row in commands:
 for stream in ['stdout','stderr']:
  if stream in row:assert h(p/row[stream]['file'])==row[stream]['sha256']
package_launches=[row for row in commands if '--package' in row['argv']]
startup=[row for row in package_launches if '--save-checkpoint' not in row['argv']]
assert len(startup)==1 and startup[0]['returncode']==0
players=[row for row in package_launches if '--save-checkpoint' in row['argv']]
assert len(players)==25 and sum(row['returncode']==0 for row in players)==23
negatives=[row for row in players if row['returncode']!=0];assert len(negatives)==2
for row in negatives:
 assert row['returncode']==1
 stdout=(p/row['stdout']['file']).read_text();stderr=(p/row['stderr']['file']).read_text().strip()
 if stderr=='Error: Checkpoint(Integrity)':assert not stdout
 else:
  assert stderr=='Error: "checkpoint withheld: native observation assertions failed"'
  report=json.loads(stdout);assert report['tick']==1 and not report['observations']['passed']
  assert len(report['observations']['checks'])==4 and all(not x['passed'] for x in report['observations']['checks'])
  probe['check_logs'](report['logs'],0,1)
 for flag in ['--output','--save-checkpoint']:assert not Path(row['argv'][row['argv'].index(flag)+1]).exists()
comparison=j(prep/'accepted-source-comparison.json');source_comparison={}
for name,old in comparison['producer_changes'].items():
 accepted=subprocess.check_output(['git','show',comparison['producer']+':'+name],cwd=r)
 source_comparison[name]={'accepted_sha256':hashlib.sha256(accepted).hexdigest(),'integrated_sha256':h(r/name),'identical':accepted==(r/name).read_bytes()}
main='fca5d5fe714748215fc285d24b998d33f6655fc1';subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
record={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'frozen_head':f['head'],'main':main,'inputs':len(f['files']),'zero_exit_top_level_commands':len(checks),'cpu_tests':tests,'actual_host_tests':conformance,'fresh_full_processes':3,'continuation_pairs':10,'typed_refusals':2,'source_comparison':source_comparison,'runtime_file_count':len(res['package_files']),'public_package_result_sha256':h(p/'result.json'),'limits':['CPU and ordinary shipping evidence only; GPU/window/device tests not run.','Legacy Wasm protocols retain explicit compound refusal; new compound ABI is a separate future intake.','Historical source diagnostics and cross-checkout stale wasm cache failures are retained; wasm recovery used scoped package cleanup on a private clone.']}
(o/'proof-verification.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k!='source_comparison'},indent=2))
