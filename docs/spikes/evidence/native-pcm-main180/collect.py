from pathlib import Path
import base64,gzip,hashlib,json,re,subprocess,sys,datetime,shutil,array,math
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/native-pcm-main180-001';p=o/'shipping';d=r/'docs/spikes/evidence/native-pcm-main180'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
f=read(o/'freeze.json');assert f['head']=='f47170c24a4b715c090071678edc6d420326fb68';assert all(h(r/k)==v for k,v in f['files'].items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==11 and all(x['exit']==0 for x in checks)
for c in checks:
 for s in ['stdout','stderr']:assert h(c[s]['path'])==c[s]['sha256']
counts={}
for name in ['player-tests','checkpoint-restart-tests']:
 c=next(x for x in checks if x['name']==name);m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',Path(c['stdout']['path']).read_text());assert m and all(int(x[1])==int(x[2])==0 for x in m);counts[name]={'passed':sum(int(x[0]) for x in m),'failed':0,'ignored':0,'groups':len(m)}
assert sum(x['passed'] for x in counts.values())==75
comp=read(o.parent/'native-pcm-main180-preparation/producer-comparison.json')
for row in comp['rows']:
 assert h(r/row['path'])==row['current_sha256'];assert sha(subprocess.check_output(['git','show',comp['producer']+':'+row['path']],cwd=r))==row['producer_sha256']
assert [x['path'] for x in comp['rows'] if not x['exact']]==['crates/incant_game_player/Cargo.toml']
result=read(p/'result.json');export=read(p/'package/export.json');assert result['passed'] and result['commands']==8 and result['frames']==1724800 and result['won_tick']==2066
assert result['helper_sha256']==h(r/'tools/probes/native-streaming-pcm.py');assert h(p/'package/player/incant_game_player')==result['player_sha256']==export['player_sha256'];assert export['build_storage_removed_before_startup'] and not export['startup_source_parsing'];assert not (p/'authoring').exists() and not list((p/'package').rglob('*.map'))
assert all(h(p/'package'/k)==v for k,v in result['package_files'].items());assert not Path(export['physics']['target_output']).exists()
sys.path.insert(0,str(r/'tools'));from export_native import verify_codegen
assert verify_codegen((p/'package/build.log').read_text(),export['target'],'packed')==export['codegen']
fixture=Path.home()/'.codex/incant-evidence/native-streaming-pcm-fixture-001';old=Path.home()/'.codex/incant-evidence/0060/packages/f5-en-playing';assert all(h(fixture/k)==v for k,v in result['fixture'].items());assert all(h(old/k)==v for k,v in result['old_package'].items());assert read(p/'deleted-authoring.json')==read(fixture/'receipt.json')['project_files']
commands=[read(x) for x in sorted(p.glob('command-*.json'))];assert len(commands)==8;by={}
for c in commands:
 args=c['argv'];out=Path(args[args.index('--output')+1]);name=Path(args[args.index('--input-recording')+1]).stem.removesuffix('-input');by[name]=c
 if c['stdout']:
  report=json.loads(c['stdout']);assert report==read(p/(name+'.json'));assert report['frames']*8==out.stat().st_size and report['pcm_sha256']==h(out)
  if name=='failed-observation':assert c['exit_code']!=0 and c['stderr']=='Error: "native observation assertions failed (see completed report)"\n' and not report['observations']['passed']
  else:assert c['exit_code']==0 and c['stderr']=='' and report['observations']['passed']
 else:assert c['exit_code']!=0
assert by['old-bound']['stderr']=='Error: "output frame bound"\n' and not (p/'old-bound.pcm').exists()
assert by['no-clobber']['stderr']=='Error: Os { code: 17, kind: AlreadyExists, message: "File exists" }\n' and (p/'existing.pcm').read_bytes()==b'existing accepted output'
assert 'kind: NotFound' in by['output-io']['stderr'] and not (p/'absent/audio.pcm').exists() and not list(p.glob('.incant-pcm-*'))
full=read(p/'full.json');assert full==read(p/'repeat.json');assert h(p/'full.pcm')==h(p/'repeat.pcm')==h(p/'failed-observation.pcm')==result['pcm_sha256'];assert full['tick']==2156 and full['frames']==1724800 and full['render_input']['ui']['glyphs']>0 and full['logs']['dropped']==0
assert [x['tick'] for x in full['logs']['entries'] if x['message'].startswith('core.pickup')]==[127,508,1266,1842];assert [x['tick'] for x in full['logs']['entries'] if x['message'].startswith('core.won')]==[2066]
for key in ['tick','frames','logs','observations','pcm_sha256']:assert read(p/'old-prefix.json')[key]==read(p/'prefix.json')[key]
assert h(p/'old-prefix.pcm')==h(p/'prefix.pcm')
samples=array.array('f');samples.frombytes((p/'full.pcm').read_bytes());assert sys.byteorder=='little' and len(samples)==3449600 and all(math.isfinite(v) for v in samples) and any(v!=0 for v in samples)
print(json.dumps({'counts':counts,'commands':len(checks),'player_processes':len(commands),'frames':full['frames'],'player_sha256':result['player_sha256'],'sources':len(f['files'])}))
if '--verify-only' in sys.argv:raise SystemExit(0)
d.mkdir(exist_ok=True,parents=True);records=[];external=[];payloads={}
for q in sorted(o.rglob('*')):
 if not q.is_file():continue
 raw=q.read_bytes();s=sha(raw);row={'original':str(q),'sha256':s,'bytes':len(raw)}
 if q.suffix=='.pcm' or q.name=='incant_game_player' or 'symbols' in q.relative_to(o).parts:external.append(row);continue
 records.append(row)
 if s not in payloads:
  try:payloads[s]={'encoding':'utf8','data':raw.decode()}
  except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'artifacts':records,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
shutil.copyfile(o/'freeze.json',d/'freeze.json');shutil.copyfile(Path(__file__),d/'collect.py');shutil.copyfile(o.parent/'native-pcm-main180-run.py',d/'run.py')
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':f['head'],'base_main':'3b2ffd06a8cbe85748723a3cf8acae35b1a89b8a','review_head':git('rev-parse','HEAD'),'source_freeze':{'path':str((d/'freeze.json').relative_to(r)),'sha256':h(d/'freeze.json'),'count':len(f['files'])},'producer_comparison':comp,'checks':checks,'counts':counts,'package_proof':result,'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(records),'payloads':len(payloads)},'external_artifacts':external,'review_state':'Independent current-main final review pending','limits':['CPU shipping only; no GPU/window/device/performance claim','PCM and checkpoint publication are separate transactions','Reused byte-verified trusted raw Wasm fixture for compile boundary only']}
q=r/'docs/spikes/evidence/native-pcm-main180.json';q.write_text(json.dumps(manifest,indent=2)+'\n');print(q,h(q))
