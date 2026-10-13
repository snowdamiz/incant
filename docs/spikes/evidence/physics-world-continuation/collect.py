from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,shutil,subprocess,sys,tomllib
ROOT=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant')
OUT=Path(__file__).resolve().parent if 'incant-evidence' in str(Path(__file__).resolve()) else Path('/Users/andreyyurlov/.codex/incant-evidence/physics-world-continuation-001')
DEST=ROOT/'docs/spikes/evidence/physics-world-continuation'
MANIFEST=ROOT/'docs/spikes/evidence/physics-world-continuation.json'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=ROOT)
freeze=read(OUT/'source-003.json');assert h(OUT/'source-003.json')=='09936373f129de39423ebc10ed4934f3b960363365bc814d427b5beb1491c6be'
assert freeze['base']=='8c1db376fc8c3e6651a353112c58ba66515f6733'
subprocess.run(['git','merge-base','--is-ancestor',freeze['base'],'HEAD'],cwd=ROOT,check=True)
for row in freeze['files']:assert h(ROOT/row['path'])==row['sha256'],row['path']
checks=read(OUT/'final-003/results.json');assert len(checks)==11 and all(r['exit']==0 for r in checks)
assert read(OUT/'final-003/source-after.json')['changed']==[]
initial=read(OUT/'results.json');assert len(initial)==9 and all(r['exit']==0 for r in initial[:8]) and initial[8]['label']=='manifest-guard' and initial[8]['exit']==1
assert read(OUT/'source-after.json')['changed']==[]
tooling=read(OUT/'tooling-final/results.json');assert len(tooling)==2 and all(r['exit']==0 for r in tooling)
for r in initial+checks+tooling:
 for stream in ('stdout','stderr'):assert h(r[stream]['path'])==r[stream]['sha256']
def counts(path):
 rows=[tuple(map(int,x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',Path(path).read_text())]
 assert rows and all(b==c==0 for a,b,c in rows)
 return {'passed':sum(a for a,b,c in rows),'failed':0,'ignored':0,'groups':len(rows),'nonempty_groups':sum(a>0 for a,b,c in rows)}
actual_counts={r['label']:counts(r['stdout']['path']) for r in checks[:3]}
assert actual_counts['snapshot-default']['passed']==121 and actual_counts['snapshot-default']['groups']==18
assert [actual_counts[k]['passed'] for k in ('continuation-fast-parallel','continuation-simd8-parallel')]==[20,20]
assert counts(OUT/'runtime-tests.stdout')['passed']==18
for path in [OUT/'snapshot-parallel.stdout',*[Path(r['stdout']['path']) for r in checks[:3]]]:
 s=path.read_text()
 for dim,forces in [(2,195),(3,198)]:assert f'trusted_continuation dimension={dim} checkpoints=3 steps=300 collision_events=12 force_events={forces} state_and_scratch_exact=true' in s
 assert 'trusted_continuation' in s
normal=Path(tooling[0]['stdout']['path']).read_text().splitlines();assert len(normal)==1 and normal[0].startswith('incant_physics_snapshot ')
# Original registry package identities and complete modification set.
registry=Path('/Users/andreyyurlov/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f')
allowed={'Cargo.toml','Cargo.toml.orig','src/dynamics/mod.rs','src/dynamics/island_manager/mod.rs','src/dynamics/island_manager/persistent.rs'}
vendors=[]
for name in ('rapier2d','rapier3d'):
 root=ROOT/'third_party'/name;upstream=read(root/'INCANT_UPSTREAM.json');changed=[]
 for row in upstream['files']:
  assert h(registry/(name+'-0.36.0')/row['path'])==row['sha256']
  if h(root/row['path'])!=row['sha256']:changed.append(row['path'])
 assert set(changed)==allowed
 for path in changed:assert 'Incant modification:' in (root/path).read_text()
 license=read(root/'INCANT_LICENSE.json');assert h(root/'LICENSE')==license['sha256']=='ceacfa4d7fa67df64ab09a56fe248c50a3bfc9bc00374d69bd74ad763b14b89c'
 assert b'Copyright 2020 S\xc3\xa9bastien Crozet' in (root/'LICENSE').read_bytes()
 assert h(OUT/'license/license.raw')==license['sha256'] and h(OUT/'license/tree.raw')==license['tree_sha256']
 vendors.append({'name':name,'upstream_files':len(upstream['files']),'changed':changed,'license_sha256':license['sha256']})
# No resolution/version/dependency changes: only patched Rapier source/checksum fields.
lock_rows=[]
for rel in ['Cargo.lock',*[f'tools/probes/{p}/Cargo.lock' for p in ('physics','physics-parity','native-physics','physics-snapshot-boundary')]]:
 before=tomllib.loads(git('show',freeze['base']+':'+rel).decode())['package'];after=tomllib.loads((ROOT/rel).read_text())['package'];key=lambda r:(r['name'],r['version'])
 old={key(r):r for r in before};new={key(r):r for r in after};assert len(old)==len(before) and old.keys()==new.keys()
 changes=[]
 for k,v in old.items():
  if v!=new[k]:
   assert k in [('rapier2d','0.36.0'),('rapier3d','0.36.0')]
   assert {a:b for a,b in v.items() if a not in ('source','checksum')}==new[k]
   changes.append(list(k))
 lock_rows.append({'path':rel,'packages':len(new),'registry_to_exact_path':changes})
# Actual target compiler records prove adapter is absent from ordinary kernels.
sys.path.insert(0,str(ROOT/'tools'));import physics_features
compiled=physics_features.validate_artifacts(physics_features.compiler_messages(OUT/'target-kernels.stdout'),'deterministic',target_dir='/Users/andreyyurlov/.codex/incant-physics-restore-target',target='aarch64-apple-darwin',profile='dev')
assert len(compiled['features'])==5 and all('incant-checkpoint' not in f for f in compiled['features'].values())
assert all(r['exit']==0 for r in read(OUT/'probe-lock-checks.json'))
identity=read(OUT/'identity/result.json');assert len(identity['rows'])==5
baseline=identity['rows'][0]['ids']
for index in (1,3):assert identity['rows'][index]['ids']['INCANT_CHECKPOINT_RUNTIME_ID']!=baseline['INCANT_CHECKPOINT_RUNTIME_ID']
for index in (2,4):assert identity['rows'][index]['ids']==baseline
for r in identity['rows']:
 assert r['exit']==0 and r['ids']['INCANT_SCRIPT_BUILD_ID']==baseline['INCANT_SCRIPT_BUILD_ID']
 assert h(OUT/'identity'/(r['label']+'.stdout'))==r['stdout_sha256']
for r in read(OUT/'binaries.json'):assert h(r['original'])==h(r['copy'])==r['sha256']
review=Path('/Users/andreyyurlov/.codex/incant-reviews/physics-world-continuation-source-root-20261013.json');assert h(review)=='ba2d68eb3283a56bfc7e3dfb357da2d74eb91beaaa868c6e37abae57374511c4'
summary={'passed':True,'frozen_files':len(freeze['files']),'final_checks':11,'tooling_checks':2,'actual_counts':actual_counts,'runtime_tests':18,'modes':'default deterministic, fast parallel, simd8 parallel; earlier deterministic parallel preserved','continuation':'3 checkpoints x100 steps x2 dimensions per mode; exact serialized owner, skipped scan, collision/bitwise force events','wrap':'synthetic counter placement near u32MAX, actual native scan step and32 subsequent steps; serde-only omission must diverge','normal_snapshot_graph':'std-only','normal_solver_adapter_enabled':False,'vendors':vendors,'locks':lock_rows,'live_physics_save_admission':False}
if '--verify-only' in sys.argv:
 m=read(MANIFEST);archive=m['archive'];raw=(ROOT/archive['path']).read_bytes();assert sha(raw)==archive['sha256'];bundle=json.loads(gzip.decompress(raw));assert len(bundle['records'])==archive['records']
 for row in bundle['records']:
  b=base64.b64decode(bundle['payloads'][row['sha256']]);assert sha(b)==row['sha256'] and len(b)==row['bytes'] and Path(row['original']).read_bytes()==b
 assert m['summary']==summary
 print(json.dumps(summary));raise SystemExit(0)
DEST.mkdir(parents=True,exist_ok=True);records=[];payloads={}
for path in sorted(OUT.rglob('*')):
 if not path.is_file() or path.is_relative_to(OUT/'identity/sandbox') or path.is_relative_to(OUT/'binaries') or path==OUT/'identity/build-script-build':continue
 b=path.read_bytes();digest=sha(b);records.append({'original':str(path),'sha256':digest,'bytes':len(b)});payloads.setdefault(digest,base64.b64encode(b).decode())
z=gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0);assert len(z)<16*1024*1024
(DEST/'artifacts.json.gz').write_bytes(z)
for src,name in [(Path(__file__),'collect.py'),(OUT/'source-003.json','source-freeze.json'),(review,'source-review.json')]:shutil.copyfile(src,DEST/name)
m={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'base':freeze['base'],'source_freeze':{'path':str((DEST/'source-freeze.json').relative_to(ROOT)),'sha256':h(DEST/'source-freeze.json')},'summary':summary,'final_checks':checks,'initial_checks':initial,'tooling':tooling,'archive':{'path':str((DEST/'artifacts.json.gz').relative_to(ROOT)),'sha256':sha(z),'bytes':len(z),'records':len(records),'payloads':len(payloads)},'binaries':read(OUT/'binaries.json'),'source_review':{'path':str((DEST/'source-review.json').relative_to(ROOT)),'sha256':h(review)},'limits':['Trusted own-native serde reconstruction only; hostile inert decode is not a live restore constructor. PhysicsUnsupported stays closed.','Joint/soft-body owners, complete collider/manager/CCD/configuration/phase and hostile constructor closure remain open.','Normal runtime/wasm/compiler checks ran source001; final003 differs in upstream comments/licenses plus snapshot dev forwarding and Python negative-control version precision. Runtime algorithm and default feature sources unchanged.','Final default121 includes older115 cases; mode repeats are overlapping, not additive coverage.','Near-wrap counter placement is synthetic. Other pinned split-cooldown wrap paths are not established.','No shipping game, device, performance, renderer or phase gate claim.'],'review_state':'Source001 accepted; final003 license/manifest addendum and final execution packet awaiting independent review'}
MANIFEST.write_text(json.dumps(m,indent=2)+'\n');print(json.dumps(summary));print(str(MANIFEST),h(MANIFEST))
