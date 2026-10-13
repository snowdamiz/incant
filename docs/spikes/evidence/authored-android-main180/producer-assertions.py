import pathlib,json,hashlib
out=pathlib.Path(__file__).parent;records=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def files(p):return {str(f.relative_to(p)):sha(f) for f in p.rglob('*') if f.is_file()}
for mode in ('deterministic','fast'):
 exp=out/(mode+'-export');run=out/(mode+'-callers');r=json.loads((run/'result.json').read_text());e=json.loads((exp/'export.json').read_text());assert r['passed'] and r['cleanup_errors']==[]
 assert e['private_build_storage_removed'] and e['physics_mode']==e['physics_variant']==mode
 assert files(exp/'runtime')==r['runtime_before'];assert files(run/'inputs')==r['inputs']
 assert not pathlib.Path(r['source_removal']['authoring']).exists();assert not (exp/'symbols/behavior.js.map').exists()
 assert r['emulator_cleanup']=={'graceful':True,'fallback':False,'adb_error':None,'exit_code':0};assert r['delete']['exit_code']==0
 count=0
 for c in [*r['commands'],r['create'],r['delete']]:
  for stream in ('stdout','stderr'):assert sha(pathlib.Path(c[stream]))==c[stream+'_sha256'];count+=1
 assert sha(pathlib.Path(r['caller']['path']))==r['caller']['sha256']
 assert sha(exp/e['archive']['path'])==e['archive']['sha256']
 assert sha(exp/'symbols/libincant_game_player.unstripped.a')==e['symbols']['unstripped_sha256']
 statuses=[c['expected_status'] for c in r['controls']];assert statuses==[0,0,0,3,1,2,2,2,2,1]
 for c in r['controls']:
  assert sha(pathlib.Path(c['request']))==c['request_sha256']
  if c['expected_status']==0:
   name=c['name'].removeprefix('c-');g=c['report']['game'];n=json.loads((run/'inputs'/(name+'-native.json')).read_text());oracle=json.loads((run/'inputs'/(name+'-expected.json')).read_text());assert g['logs']==n['logs']==oracle['logs'];assert len(g['logs']['entries'])==36
   for k in ('tick','frames','observations','pcm_sha256'):assert g[k]==n[k]
   pcm=(run/c['name']/'audio.pcm').read_bytes();assert pcm==(run/'inputs'/(name+'.pcm')).read_bytes() and len(pcm)==76800 and any(pcm)
 records.append({'mode':mode,'export_receipt_sha256':sha(exp/'export.json'),'caller_receipt_sha256':sha(run/'result.json'),'commands_with_avd':len(r['commands'])+2,'raw_command_streams_rehashed':count,'positive_processes':3,'exact_refusals':7,'logs_per_positive':36,'vector_checks_per_positive':72,'pcm_bytes_per_positive':76800,'runtime_files':len(r['runtime_before']),'archive_bytes':e['archive']['bytes'],'caller_bytes':r['caller']['bytes'],'api':r['api'],'os':r['os'],'owned_emulator_cleanup':True})
freeze=json.loads((out/'freeze.json').read_text());root=pathlib.Path(freeze['root']);assert all(sha(root/f['path'])==f['sha256'] for f in freeze['files'])
summary={'passed':True,'kind':'producer final re-audit; independent final review pending','frozen_inputs':len(freeze['files']),'records':records}
(out/'producer-reaudit.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
