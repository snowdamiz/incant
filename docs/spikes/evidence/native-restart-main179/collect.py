from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,subprocess,sys,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');base=Path.home()/'.codex/incant-integration-evidence';o=base/'native-restart-main178-001';dest=r/'docs/spikes/evidence/native-restart-main179'
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
sha=lambda b:hashlib.sha256(b).hexdigest()
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=r,text=True).strip()
freeze=read(o/'freeze.json');assert freeze['head']=='7f6e41c4a1a4d799dd115944f26cee51676fd36c';assert all(h(r/p)==v for p,v in freeze['files'].items() if p!='.gitattributes')
original_attrs=subprocess.check_output(['git','show',freeze['head']+':.gitattributes'],cwd=r);attrs_suffix=b'\n# Preserve raw restart evidence logs, including intentional terminal blank lines.\ndocs/spikes/evidence/native-session-restart/**/*.log -text whitespace=-blank-at-eof\n';assert (r/'.gitattributes').read_bytes()==original_attrs+attrs_suffix
metadata_seam={'path':'.gitattributes','tested_sha256':sha(original_attrs),'current_sha256':h(r/'.gitattributes'),'reason':'Preserve producer raw log bytes and exempt only their terminal blank lines; no compiler input or runtime change.'}
assert read(o/'terminal.json')['exit']==0
checks=read(o/'results.json');assert len(checks)==24
assert [(c['name'],c['exit']) for c in checks if c['exit']!=0]==[('static-check',101)]
assert next(c for c in checks if c['name']=='static-check-retry')['exit']==0
assert 'INCANT_WASM_BUNDLE' in (o/'static-check.log').read_text()
for c in checks:assert h(c['log'])==c['sha256']
counts={}
for name in ['window-tests','runtime-tests','mobile-tests']:
 a=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', (o/(name+'.log')).read_text());assert a and not any(int(x[1]) or int(x[2]) for x in a);counts[name]={'passed':sum(int(x[0]) for x in a),'failed':0,'ignored':0,'groups':len(a)}
assert sum(v['passed'] for v in counts.values())==355
binary=read(o/'binary.json');assert h(binary['path'])==binary['sha256']
comparison=read(o/'producer-source-comparison.json');producer=comparison['producer'];assert len(comparison['rows'])==34
for s in comparison['rows']:
 assert h(r/s['path'])==s['current_sha256'];assert sha(subprocess.check_output(['git','show',producer+':'+s['path']],cwd=r))==s['producer_sha256']
assert [s['path'] for s in comparison['rows'] if not s['exact']]==['crates/incant_game_player/Cargo.toml']
producer_manifest=read(r/'docs/spikes/evidence/native-session-restart.json');review=r/producer_manifest['final_review']['receipt'];assert h(review)==producer_manifest['final_review']['sha256']
sys.path.insert(0,str(r/'tools'));from export_native import verify_codegen
proofs={};external=[]
for sample in ['core','planar']:
 dev=o/(sample+'-development');p=o/(sample+'-shipping');prepared=read(dev/'prepared.json');result=read(p/'result.json');export=read(p/'package/export.json')
 assert prepared['binary_sha256']==binary['sha256'] and prepared['public_undo_redo_reopen']
 assert prepared['source_sha256']==h(r/f'samples/{sample}/gameplay.ts')==result['source_sha256']
 assert prepared['helper_sha256']==h(r/'tools/probes/native-session-restart.py')==result['helper_sha256']
 assert result['passed'] and result['commands']==12 and len(result['runs'])==12
 assert result['exact_fresh_session_pcm'] and result['same_simulation_tick_distinct_host_checks']
 for path,s in prepared['original_hashes'].items():assert h(Path(prepared['original_authoring'])/path)==s
 for path,s in prepared['authoring_files'].items():assert h(dev/'authoring'/path)==s
 for path,s in result['package_files'].items():assert h(p/'package'/path)==s
 assert h(p/'package/player/incant_game_player')==result['player_sha256']==export['player_sha256']
 assert export['build_storage_removed_before_startup'] and export['startup_source_parsing'] is False
 assert not Path(export['physics']['target_output']).exists()
 assert export['target']=='aarch64-apple-darwin' and export['physics_mode']=='deterministic'
 assert verify_codegen((p/'package/build.log').read_text(),export['target'],'packed')==export['codegen']
 assert not (p/'authoring').exists() and not list((p/'package').rglob('*.map'))
 deleted=read(p/'deleted-authoring.json');assert deleted[f'{sample}.json']==export['project_sha256'] and deleted['gameplay.ts']==prepared['authored_source_sha256']
 commands=sorted(p.glob('command-*.json'));assert len(commands)==12
 records={}
 for f in commands:
  c=read(f);assert c['exit_code']==0 and c['argv'][0]==str(p/'package/player/incant_game_player');a=c['argv'];output=Path(a[a.index('--output')+1]);name=output.stem;report=read(p/(name+'.json'));assert json.loads(c['stdout'])==report
  assert h(output)==report['pcm_sha256'];assert report['observations']['passed'] and all(x['passed'] for x in report['observations']['checks']);assert report['render_input']['ui']['glyphs']>0 and report['logs']['dropped']==0
  plan=read(p/(name+'-observe.json'));assert len(plan['checks'])==len(report['observations']['checks']);assert {(x['name'],x['tick']) for x in plan['checks']}=={(x['name'],x['tick']) for x in report['observations']['checks']};records[name]=report
 for name in prepared['runs']:
  report=records[name];expected=read(dev/(name+'.json'));assert 'session' not in report
  for field in ['tick','frames','logs','observations','pcm_sha256']:assert report[field]==expected[field],(sample,name,field)
 for name,first,second in [('win-restart-win','win','win'),('gamepad-restart-win','gamepad','gamepad'),('lose-restart-win','lose','win'),('held-input-restart-idle','win','idle')]:
  report=records[name];expected=records[second];split=read(dev/(first+'-input.json'))['ticks'];ticks=read(dev/(second+'-input.json'))['ticks']
  assert report['tick']==expected['tick'] and report['session']=={'generation':2,'host_tick':split+ticks,'simulation_tick':ticks,'restarts':[{'host_step':split,'completed_generation':1,'simulation_tick':split,'output_frames':str(records[first]['frames'])}]}
  for field in ['logs','render_input','flipbooks','sources']:assert report[field]==expected[field],(sample,name,field)
  assert report['observations']['entities']==expected['observations']['entities']
  assert (p/(name+'.pcm')).read_bytes()==(p/(first+'.pcm')).read_bytes()+(p/(second+'.pcm')).read_bytes()
  assert report['frames']==records[first]['frames']+expected['frames']
  checked={x['name']:x for x in report['observations']['checks']};assert all(checked[f'fresh-pickup-host-{t}']['tick']==t and checked[f'fresh-pickup-host-{t}']['passed'] for t in [1,split+1])
  if name=='held-input-restart-idle':assert all(checked['held-isolated-'+a]['passed'] and checked['held-isolated-'+a]['tick']==split+1 for a in ['move','restart'])
 devcommands=[read(x) for x in sorted(dev.glob('command-*.json'))];assert len(devcommands)==9
 refusal=devcommands[-1];assert refusal['exit_code']==1 and not refusal['stdout'];assert refusal['stderr']=='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n'
 authoring=read(dev/'authoring-calls.json');methods=[c['method'] for c in authoring];assert all(n in methods for n in ['history.undo','history.redo','project.save']);assert methods[-2:]==['project.read','script.read']
 proofs[sample]={'processes':12,'baseline_cases':prepared['runs'],'restart_cases':[n for n in records if 'restart' in n],'exact_session_state_and_pcm':True,'headless_refusal':refusal['stderr'],'package_files':len(result['package_files']),'player_sha256':result['player_sha256'],'result_sha256':h(p/'result.json'),'export_sha256':h(p/'package/export.json'),'authoring_undo_redo_reopen':True}
print(json.dumps({'checks':len(checks),'counts':counts,'proofs':proofs,'source_inputs':len(freeze['files']),'source_rows':len(comparison['rows'])}))
# Everything above is assertion-only and may be independently replayed without mutation.
if '--verify-only' in sys.argv:raise SystemExit(0)
dest.mkdir(exist_ok=True,parents=True);artifacts=[];payloads={}
for p in sorted(o.rglob('*')):
 if not p.is_file():continue
 rel=p.relative_to(o);raw=p.read_bytes();s=sha(raw)
 if p==Path(binary['path']) or ('symbols' in rel.parts) or p.name=='incant_game_player':
  external.append({'path':str(p),'sha256':s,'bytes':len(raw)});continue
 if 'static-fixture' in rel.parts and p.stat().st_size>200000: # compiled trusted fixture; retained outside repo, compile-only
  external.append({'path':str(p),'sha256':s,'bytes':len(raw)});continue
 artifacts.append({'original':str(p),'sha256':s,'bytes':len(raw)})
 if s not in payloads:
  try:payloads[s]={'encoding':'utf8','data':raw.decode('utf8')}
  except UnicodeDecodeError:payloads[s]={'encoding':'base64','data':base64.b64encode(raw).decode()}
archive=dest/'artifacts.json.gz';archive.write_bytes(gzip.compress(json.dumps({'artifacts':artifacts,'payloads_by_sha256':payloads},ensure_ascii=False,separators=(',',':')).encode(),mtime=0))
for name in ['freeze.json','producer-source-comparison.json','integration-manifest.patch']:shutil.copyfile(o/name,dest/name)
shutil.copyfile(Path(__file__),dest/'collect.py');shutil.copyfile(base/'native-restart-main178-run.py',dest/'run.py');shutil.copyfile(base/'native-restart-main178-resume.py',dest/'resume.py')
manifest={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'tested_head':freeze['head'],'review_head':git('rev-parse','HEAD'),'actual_main':'02203670a7f18c68ff9e3bcf1913b0d574e82f6d','source_freeze':{'path':str((dest/'freeze.json').relative_to(r)),'sha256':h(dest/'freeze.json'),'count':len(freeze['files'])},'source_comparison':comparison,'metadata_seam':metadata_seam,'checks':checks,'counts':counts,'proofs':proofs,'archive':{'path':str(archive.relative_to(r)),'sha256':h(archive),'artifacts':len(artifacts),'unique_payloads':len(payloads),'bytes':archive.stat().st_size},'external_artifacts':external,'producer_final_review':producer_manifest['final_review'],'initial_failure':{'name':'static-check','exit':101,'reason':'Runner omitted mandatory INCANT_WASM_BUNDLE; no source defect. Fresh current bulk guest/pinned WABT prepared, compile-only retry passed on identical source. Original failure retained.'},'pause':'Final valid Planar export was active at user pause; allowed to drain. Docs-only pause commit466c retained. User resumed before this final audit.','review_state':'Independent final integration audit pending; actual main179 ancestry still to join after review.','limits':read(o/'core-shipping/result.json')['limits']}
path=r/'docs/spikes/evidence/native-restart-main179.json';path.write_text(json.dumps(manifest,indent=2)+'\n');print(str(path),h(path))
