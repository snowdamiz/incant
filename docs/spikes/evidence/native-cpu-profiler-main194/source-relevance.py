from pathlib import Path
import ast,hashlib,json,re,subprocess,sys,tempfile,types
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o=Path.home()/'.codex/incant-integration-evidence/native-cpu-profiler-integration-001'
producer='bfe8ede26d09c3e28da519b01ed3f3b4c41d1bf6'
sha=lambda b:hashlib.sha256(b).hexdigest()
git=lambda *a:subprocess.check_output(['git',*a],cwd=r)
blob=lambda ref,p:git('show',ref+':'+p)
head=git('rev-parse','HEAD').decode().strip()
graphs=json.loads((o/'graph-relevance.json').read_text())
assert graphs['passed']
packages=set()
for row in graphs['graphs']:
 text=Path(row['current_stdout']['path']).read_text()
 assert sha(text.encode())==row['current_stdout']['sha256']
 for match in re.finditer(r'\('+re.escape(str(r))+r'/([^\)]+)\)',text):packages.add(match.group(1))
assert 'crates/incant_game_player' in packages and 'crates/incant_render' not in packages
changed=git('diff','--name-only',producer,head,'--','crates','tools','Cargo.toml','Cargo.lock','.cargo','rust-toolchain.toml','sdk','schemas','third_party').decode().splitlines()
rows=[]
for p in changed:
 if p=='Cargo.lock':reason='Only incant_render gains the already-pinned rustix edge; selected normal target graphs and all package identities are unchanged.'
 elif p=='crates/incant_script/src/restart_request_tests.rs':
  assert '#[cfg(all(test, feature = "compiler"))]\nmod restart_request_tests;' in (r/'crates/incant_script/src/lib.rs').read_text()
  reason='Test-only quota budget regression, cfg(all(test, compiler)); absent ordinary player target.'
 elif p in ('crates/incant_cook/src/project_build.rs','crates/incant_cook/src/project_build_wasm.rs'):reason='Reviewed mobile static-Wasm admission only. All new calls are inside mobile predicates; ordinary desktop TypeScript has mobile=false and no scene program.'
 elif p=='crates/incant_cook/tests/mobile_project.rs':reason='Build-host mobile test only; not compiled by ordinary exporter.'
 elif p.startswith(('crates/incant_render/','crates/incant_render_gpu_host/','crates/incant_shader_cook/')):
  assert '/'.join(p.split('/')[:2]) not in packages
  reason='Renderer/compiler GPU family outside all three selected normal player graphs.'
 elif p=='tools/native_wasm.py':reason='Selector extraction preserves TypeScript program=None early return; old/new functions executed below with denied build callback.'
 elif p=='tools/wasm2c.py':reason='Optional audit callback retains default runner; never invoked by TypeScript selector. Existing desktop exporter does not pass audit_translation.'
 elif p=='tools/shipping.py':
  functions=lambda data:{node.name:ast.dump(node,include_attributes=False) for node in ast.parse(data).body if isinstance(node,(ast.FunctionDef,ast.AsyncFunctionDef))}
  old=functions(blob(producer,p));new=functions((r/p).read_bytes());assert old['codegen']==new['codegen']
  assert 'from shipping import codegen' in (r/'tools/export_native.py').read_text()
  reason='Imported codegen parser is AST-identical; only shlex import and new independent player_units probe helper were added, not invoked by export_native.'
 elif p.startswith('tools/'):
  assert p not in ['tools/export_native.py','tools/native_profiling.py','tools/native_package.py','tools/physics_export.py','tools/native_play.py','tools/native_process.py','tools/probes/native-cpu-profile.py']
  reason='Standalone mobile exporter/probe or shipping-test helper not invoked by unchanged desktop CPU-profile helper/exporter.'
 else:raise AssertionError(('unclassified input',p))
 old=git('ls-tree',producer,'--',p).strip();before=sha(blob(producer,p)) if old else None
 rows.append({'path':p,'producer_sha256':before,'current_sha256':sha((r/p).read_bytes()),'qualification':reason})
# Exhaustive source equality for every workspace package in the normal target closure.
runtime=[]
for package in sorted(packages):
 paths=git('ls-tree','-r','--name-only',head,'--',package).decode().splitlines()
 for p in paths:
  if not (p.endswith(('.rs','.toml','.c','.h')) or '/src/' in p):continue
  before=blob(producer,p);now=(r/p).read_bytes()
  if p!='crates/incant_script/src/restart_request_tests.rs':assert before==now,p
  runtime.append({'path':p,'producer_sha256':sha(before),'current_sha256':sha(now),'test_only_exception':p=='crates/incant_script/src/restart_request_tests.rs'})
unchanged=[]
for p in ['Cargo.toml','rust-toolchain.toml','.cargo/config.toml','tools/export_native.py','tools/native_profiling.py','tools/native_package.py','tools/native_play.py','tools/native_process.py','tools/physics_export.py','tools/probes/native-cpu-profile.py','tools/probes/native-cpu-profile/behavior.ts']:
 assert blob(producer,p)==(r/p).read_bytes(),p
 unchanged.append({'path':p,'sha256':sha((r/p).read_bytes())})
# Execute only admission for the exact TypeScript selector cases; no compiler/process can run.
sys.path.insert(0,str(r/'tools'))
modules=[]
for name,source in [('old',blob(producer,'tools/native_wasm.py')),('current',(r/'tools/native_wasm.py').read_bytes())]:
 module=types.ModuleType('profile_native_wasm_'+name);exec(compile(source,name,'exec'),module.__dict__);modules.append(module)
cases=[('typescript-absent',{'scenes':{'scene':{'entities':{}}}},None),('typescript-null',{'scenes':{'scene':{'program':None,'entities':{}}}},'scene'),('missing-scene',{'scenes':{'scene':{}}},'absent'),('ambiguous',{'scenes':{'a':{},'b':{}}},None),('invalid-program',{'scenes':{'scene':{'program':{'kind':'other'}}}},None)]
selector=[]
with tempfile.TemporaryDirectory(prefix='incant-profile-selector-',dir=o) as temporary:
 project=Path(temporary)/'project.json'
 def denied(*a,**kw):raise AssertionError('No build/process may run in a TypeScript selector control')
 for name,document,scene in cases:
  project.write_text(json.dumps(document));outcomes=[]
  for module in modules:
   env={}
   try:value=module.capture(project,scene,Path(temporary)/'never','never-cargo',env,denied,None);outcome={'result':value}
   except ValueError as error:outcome={'error':str(error)}
   assert env=={};outcomes.append(outcome)
  assert outcomes[0]==outcomes[1],(name,outcomes)
  if name.startswith('typescript'):assert outcomes[0]=={'result':None}
  else:assert 'error' in outcomes[0]
  selector.append({'case':name,'both_versions':outcomes[0],'no_build_or_environment_mutation':True})
patch=git('diff',producer,head,'--','crates/incant_cook/src/project_build.rs','crates/incant_cook/src/project_build_wasm.rs','tools/native_wasm.py','tools/wasm2c.py')
(o/'source-relevance.patch').write_bytes(patch)
result={'passed':True,'producer':producer,'integrated_head':head,'packages':sorted(packages),'runtime_source_rows':runtime,'unchanged_export_inputs':unchanged,'changed_inputs':rows,'selector_controls':selector,'patch_sha256':sha(patch),'shipping_evidence_scope':'Producer packages/processes and partial Tracy capture remain explicitly producer-scoped; no new archive, player or capture is claimed. Current integration adds CPU/Clippy/wasm verification and exact target dependency/source-relevance evidence.'}
(o/'source-relevance.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps({'passed':True,'runtime_inputs':len(runtime),'changed_inputs':len(rows),'selector_controls':len(selector),'head':head}))
