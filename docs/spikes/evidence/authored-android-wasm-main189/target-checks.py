from pathlib import Path
import os,sys,json,subprocess,hashlib,re,tomllib
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');p=Path('/Users/andreyyurlov/.codex/worktrees/agent-budget-settings/incant');base=Path.home()/'.codex/incant-integration-evidence';o=base/'authored-android-wasm-main189-target-004';o.mkdir(exist_ok=False)
sys.path.insert(0,str(r/'tools'))
import export_ios,android_archive,android_wasm,physics_export,physics_features
export_ios.ROOT=r
h=lambda path:hashlib.sha256(Path(path).read_bytes()).hexdigest();git=lambda root,*a:subprocess.check_output(['git',*a],cwd=root,text=True).strip()
assert git(r,'rev-parse','HEAD')=='04c816cf7250cb66ccb109b4a66c42cff517d3d6';assert git(p,'rev-parse','HEAD')=='a11bd4f4fe3b74402572e8c3bc0ca41b814f62df';assert not git(r,'status','--porcelain') and not git(p,'status','--porcelain')
frozen=json.loads((base/'authored-android-wasm-main189-001/freeze.json').read_text())['files']
def unchanged():assert all(h(r/path)==sha for path,sha in frozen.items())
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']=str(Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin')+':/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH']
for k in list(e):
 if k.startswith(('INCANT_NATIVE_','INCANT_MOBILE_','INCANT_SCRIPT_','INCANT_WASM_','INCANT_AUTHORED_')):e.pop(k)
target=Path.home()/'.codex/incant-planar-integration-target';assert json.loads((target/'incant-checkout-owner.json').read_text())['checkout']==str(r);e['CARGO_TARGET_DIR']=str(target)
rustc=Path.home()/'.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin/rustc';ndk=android_archive.toolchain(Path('/opt/homebrew/share/android-commandlinetools/ndk/28.2.13676358'));wabt=base/'authored-ios-wasm-main185-001/trusted-tools/wabt';summary=[]
for mode in ('deterministic','fast'):
 unchanged();print('starting '+mode,flush=True);d=o/mode;d.mkdir();audit=export_ios.CommandAudit(d/'audit');execute=audit.execute
 prepared=base/f'authored-android-wasm-main189-001/{mode}-prepared';project=prepared/'authoring/game.json';env,selected=android_archive.environment(e,rustc,project,None,ndk)
 features=','.join(physics_export.cargo_features(mode,android_archive.CAPABILITIES+['static-module']));graphs={}
 for edges in ('normal','normal,build'):
  raw={}
  for label,root in [('producer',p),('current',r)]:
   export_ios.ROOT=root;raw[label]=execute([root/'tools/cargo','tree','--locked','--no-default-features','-p',android_archive.NAME,'--features',features,'--target',android_archive.TARGET,'-e',edges,'--prefix','none','--format','{p}|{f}'],env=env).stdout
   (d/(label+'-'+edges.replace(',','-')+'.txt')).write_text(raw[label])
  export_ios.ROOT=r;normalized={k:v.replace(str(p),'<checkout>').replace(str(r),'<checkout>') for k,v in raw.items()};assert normalized['producer']==normalized['current'],edges
  names={line.split()[0] for line in raw['current'].splitlines()};assert 'incant_render' not in names and 'incant_shader_cook' not in names
  if edges=='normal':
   android_archive.normal_dependencies(raw['current']);assert not physics_features.selected_graph_errors(raw['current'],mode,android_archive.TARGET)
  graphs[edges]={'normalized_sha256':hashlib.sha256(normalized['current'].encode()).hexdigest(),'packages':sorted(names)}
 source_rows=[];names=set(graphs['normal,build']['packages'])
 for name in sorted(names):
  directory=r/'crates'/name
  if not directory.is_dir():continue
  for rel in git(r,'ls-files',str(directory.relative_to(r))).splitlines():
   if not (r/rel).is_file():continue
   current=h(r/rel);original=hashlib.sha256(subprocess.check_output(['git','show','a11bd4f4:'+rel],cwd=p)).hexdigest()
   if current!=original:assert rel=='crates/incant_script/src/restart_request_tests.rs'
   source_rows.append({'path':rel,'current':current,'producer':original,'test_only_exception':current!=original})
 original_export=json.loads((Path.home()/f'.codex/incant-evidence/authored-android-wasm-shipping-001/{mode}-archive/export.json').read_text());assert android_archive.tool_hashes(ndk)==original_export['ndk']['tool_sha256'];compiler=execute([rustc,'-vV'],env=env).stdout;assert compiler==original_export['compiler']
 storage=d/'storage';storage.mkdir();module=android_wasm.prepare(project,None,d,storage,r/'tools/cargo',env,execute,wabt,ndk)
 translation=module['translation'];prior_translation=original_export['static_module']['translation'];assert translation['wabt']==prior_translation['wabt'];assert all(translation['files'][n]==prior_translation['files'][n] for n in ('linked.rs','m0.h','m0-facade.c'));guest_control=json.loads((base/'authored-android-wasm-main189-target-002/guest-current-path-control/result.json').read_text());assert guest_control['reconstructed_sha256']==guest_control['original_wasm_sha256'];assert guest_control['exact_sdk_provenance']
 current_compiler=module['compiler'];prior_compiler=original_export['static_module']['compiled']['compiler'];assert {k:v for k,v in current_compiler.items() if k!='driver_probe_sha256'}=={k:v for k,v in prior_compiler.items() if k!='driver_probe_sha256'};prior_audit=Path.home()/f'.codex/incant-evidence/authored-android-wasm-shipping-001/{mode}-archive/audit';prior_probe=[x for x in json.loads((prior_audit/'commands.json').read_text()) if '-###' in x['argv']];assert len(prior_probe)==1;prior_text=(prior_audit/prior_probe[0]['stderr']).read_text();current_probe=[x for x in audit.commands if '-###' in x['argv']];assert len(current_probe)==1;current_text=(audit.directory/current_probe[0]['stderr']).read_text();assert prior_text.replace(str(p),'<checkout>')==current_text.replace(str(r),'<checkout>')
 command=[r/'tools/cargo','check','--locked','--profile','shipping','-j2','-p','incant_game_player','--lib','--no-default-features','--features',features,'--target',android_archive.TARGET,'--target-dir',target,'--config','profile.shipping.split-debuginfo="off"','--message-format=json','-vv']
 print('target check '+mode,flush=True);result=execute(command,env=env,timeout=3600)
 messages=[json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')];physics=physics_features.validate_artifacts(messages,mode,target_dir=target,target=android_archive.TARGET,profile='shipping')
 ids={row['package_id'] for row in messages if row.get('reason')=='compiler-artifact' and row['target']['name']=='incant_game_player'};outputs=[Path(row['out_dir']) for row in messages if row.get('reason')=='build-script-executed' and row['package_id'] in ids and (Path(row['out_dir'])/'project-export.json').is_file()];assert len(outputs)==1
 receipt=json.loads((outputs[0]/'project-export.json').read_text());android_archive.producer_receipt(receipt,h(project),mode,module['program']);(d/'project-export.json').write_text(json.dumps(receipt,indent=2)+'\n')
 c_audit=android_wasm.audit_build(result.stdout,result.stderr,android_archive.TARGET,module,r)
 row={'mode':mode,'graphs':graphs,'source_rows':source_rows,'physics':physics,'module':module,'c_audit':c_audit,'project_receipt_sha256':h(d/'project-export.json'),'commands':len(audit.commands),'scope':'Actual Android target cargo check and build-time admission, not archive linking or Android process execution'};summary.append(row);(o/'results.json').write_text(json.dumps(summary,indent=2)+'\n');unchanged();print('passed '+mode,flush=True)
(o/'terminal.json').write_text(json.dumps({'exit':0,'source':git(r,'rev-parse','HEAD'),'producer':git(p,'rev-parse','HEAD'),'modes':2,'scope':'Exact selected normal/build graphs, audited wasm2c bundle/toolchain, fresh actual target checks; six prior Android callers remain producer-scoped.'},indent=2)+'\n');print('all terminal',flush=True)
