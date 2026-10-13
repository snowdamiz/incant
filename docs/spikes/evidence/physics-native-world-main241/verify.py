from pathlib import Path
import base64,datetime,gzip,hashlib,json,re,subprocess,sys,tomllib
E=Path(__file__).resolve().parent
R=Path('/Users/andreyyurlov/.codex/worktrees/grid-navigation/incant');D=R/'docs/spikes/evidence/physics-native-world-main241'
BASE='e281fe7456c7fbb85349e109fcdff3643c701256';SOURCE='08377c173c5e89d0a9cd2c990bdc4fa3cd84cae6';PRODUCER='ea4f6e80196f49e153248d22aed83d5412cae87b'
sha=lambda b:hashlib.sha256(b).hexdigest();h=lambda p:sha(Path(p).read_bytes());read=lambda p:json.loads(Path(p).read_text());git=lambda *a:subprocess.check_output(['git',*a],cwd=R)
verify='--verify-only' in sys.argv
f=read(E/'freeze.json');review=read(E/'source-freeze.json');assert len(f['files'])==3699 and f['head']==SOURCE and f['base']==BASE
for x in f['files']:assert h(R/x['path'])==x['sha256'],x['path']
assert len(review['files'])==174
for x in review['files']:
 assert h(R/x['path'])==x['sha256']==h(E/'source'/x['path'])
 assert sha(git('show',PRODUCER+':'+x['path']))==x['producer_sha256']
 assert x['exact_producer']==(x['sha256']==x['producer_sha256'])
 assert x['exact_producer']
assert h(E/'integration-seam.patch')==review['seam_sha256']
assert len(review['closure'])==2098
for x in review['closure']:
 assert h(R/x['path'])==x['sha256'] and sha(git('show',PRODUCER+':'+x['path']))==x['producer_sha256']
 assert x['sha256']==x['producer_sha256'] or x['path']=='crates/incant_physics_snapshot/src/lib.rs'
p='crates/incant_physics_snapshot/src/lib.rs';old=git('show',PRODUCER+':'+p).decode();base=git('show',BASE+':'+p).decode();pattern=r'pub enum Error \{.*?\n\}'
assert len(re.findall(pattern,old,re.S))==1 and len(re.findall(pattern,base,re.S))==1
assert re.sub(pattern,re.search(pattern,base,re.S).group(0),old,flags=re.S)==(R/p).read_text()
assert (R/'Cargo.lock').read_bytes()==git('show',PRODUCER+':Cargo.lock')
assert (R/'Cargo.toml').read_bytes()==git('show',BASE+':Cargo.toml')
assert git('merge-base','--is-ancestor',BASE,'HEAD')==b''
checks=read(E/'results.json');assert len(checks)==28,len(checks)
for x in checks:
 assert x['exit']==0 and x['reaped']
 for k in ['stdout','stderr']:assert h(x[k]['path'])==x[k]['sha256']
counts={};features={}
for label,mode in [('default','enhanced-determinism'),('fast8','simd8')]:
 for tail,want in [('tests',227),('native-tests',138)]:
  c=next(x for x in checks if x['name']==label+'-'+tail);s=Path(c['stdout']['path']).read_text();g=[list(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',s)];assert g and all(x[1:]==[0,0] for x in g)
  counts[label+'-'+tail]={'passed':sum(x[0] for x in g),'groups':len(g),'failed':0,'ignored':0};assert counts[label+'-'+tail]['passed']==want
  if tail=='tests':
   err=Path(c['stderr']['path']).read_text()
   for dim in [2,3]:
    assert f'CHECKED_BVH dimension={dim} paired_mutation_steps=226 exact_order=true fresh_workspace=true' in err
    assert f'CHECKED_BVH_PENDING dimension={dim} refit=1 optimize=32 unnormalized_bits=true' in err
  if tail=='native-tests':
   err=Path(c['stderr']['path']).read_text()
   for dim in [2,3]:
    assert f'CHECKED_ISLANDS dimension={dim} decoded_frames=4 actual_paired_steps=156 exact_slots_and_order=true retained_scan=true other_owners=trusted_clones' in err
    assert f'CHECKED_FIELDS dimension={dim} primitive_steps=48' in err and f'CHECKED_FIELDS dimension={dim} actual_steps=96' in err
   assert 'other_owners=trusted_clones' in err and 'exact_bincode=true' in err
   for dim in [2,3]:
    assert f'CHECKED_BROAD dimension={dim} decoded_frames=4 actual_paired_steps=156 exact_tree_pairs_adjacency_queue=true fresh_scratch=true other_owners=trusted_clones' in err
    assert f'CHECKED_BROAD_POLICY dimension={dim} actual_paired_steps=64 collision_events=8 strategies=2 adaptive=2 wrapped_frame=true' in err
    maps=re.findall(rf'CHECKED_BROAD_MAP dimension={dim} deterministic=(true|false) entries=(\d+) actual_bytes=(\d+) charged_bytes=(\d+) calls=(\d+)',err)
    assert len(maps)==13 and [int(x[1]) for x in maps]==[0,1,3,4,7,8,14,15,28,29,127,128,4096]
    for flag,n,actual,charged,calls in maps:
     assert (flag=='true')==(label=='default') and 0<=int(actual)<=int(charged)
     assert int(calls)==(0 if int(n)==0 else 2 if flag=='true' else 1)
    assert f'CHECKED_NARROW dimension={dim} decoded_frames=4 actual_paired_steps=156 nonzero_warmstart=true populated_retired_pool=true other_owners=trusted_clones' in err
    assert f'CHECKED_NARROW_EVENTS dimension={dim} frames=4 paired_steps=156 collision_records=25 force_records=102 force_started=5 force_ongoing=97 retired_at_capture=true omitted_force_status=native_postsolve_events_differ sorted_multiset=true all_flags_handles_generations_scalar_vector_bits=true' in err
    requests=re.findall(rf'CHECKED_NARROW_ALLOC dimension={dim} charged_bytes=(\d+) actual_bytes=(\d+) calls=(\d+)',err)
    assert len(requests)==8 and all(int(charged)==int(actual)>0 and int(calls)>0 for charged,actual,calls in requests)
    size=40 if dim==2 else 72 if label=='default' else 96
    assert f'CHECKED_NARROW_SUBSHAPE dimension={dim} actual_bytes={size} charged_bytes={size} allocations=1 some_none_distinct=true' in err
   assert 'CHECKED_NARROW_CLUSTER_SCRATCH dimension=3 populated_previous=true nonzero_current_warmstart=true paired_steps=32 constructor_admission=false compound_owners=trusted_native_clones' in err
   for dim in (2,3):
    assert f'CHECKED_WORLD dimension={dim} frames=4 paired_steps=156 collisions=25 forces=102 trusted_owner_clones=0' in err
    assert f'CHECKED_WORLD_REFUSED_TICKS dimension={dim} ticks={{7}}' in err
    assert f'CHECKED_WORLD_CCD dimension={dim} paired_steps=20 exact_retained=true' in err
    allocation=re.findall(rf'CHECKED_WORLD_ALLOC dimension={dim} charged_bytes=(\d+) actual_bytes=(\d+) calls=(\d+)',err)
    assert allocation and all(0<int(actual)<=int(charged) and int(calls)>0 for charged,actual,calls in allocation)
   for model in ('Simplified','Coulomb'):
    assert f'CHECKED_WORLD_FRICTION dimension=3 model={model} paired_steps=32 warm=true events=32' in err
   selected=c['selected_environment'];assert selected['RAYON_NUM_THREADS']=='2'
   assert selected['INCANT_PRIVATE_WORLD_CAPTURE']==str(E/('native-workspace-'+label)/'capture.rs')
 graph=(E/(label+'-native-graph.stdout')).read_text();rows={}
 for line in graph.splitlines():
  for pkg in ['rapier2d','rapier3d','parry2d','parry3d','simba']:
   if line.startswith(pkg+' '):
    fs=set(line.split('|',1)[1].replace(' (*)','').split(','))
    if pkg in rows:assert fs==set(rows[pkg])
    rows[pkg]=sorted(fs)
 assert set(rows)=={'rapier2d','rapier3d','parry2d','parry3d','simba'}
 for pkg in ['rapier2d','rapier3d','parry2d','parry3d']:
  assert mode in rows[pkg] and 'parallel' in rows[pkg]
  assert ('simd8' if mode=='enhanced-determinism' else 'enhanced-determinism') not in rows[pkg]
 for pkg in ['rapier2d','rapier3d']:assert {'incant-checked-restore','incant-checkpoint','serde-serialize'}<=set(rows[pkg])
 assert 'wide' in rows['simba'] and ('libm_force' in rows['simba'])==(mode=='enhanced-determinism')
 old=read(Path('/Users/andreyyurlov/.codex/incant-evidence/physics-native-world-construction-001')/('native-graph-det-002.json' if label=='default' else 'native-graph-fast8-002.json'));assert h(old['log'])==old['sha256']
 prior={}
 for line in Path(old['log']).read_text().splitlines():
  for pkg in rows:
   if line.startswith(pkg+' '):prior[pkg]=sorted(set(line.split('|',1)[1].replace(' (*)','').split(',')))
 assert rows==prior;features[label]=rows
for label,mode in [('default','enhanced-determinism'),('fast8','simd8')]:
 artifacts=[]
 for line in (E/(label+'-native-tests.stdout')).read_text().splitlines():
  if not line.startswith('{'):continue
  x=json.loads(line)
  if x.get('reason')=='compiler-artifact' and x['target']['name'] in ['rapier2d','rapier3d','parry2d','parry3d']:
   assert mode in x['features'] and ('simd8' if mode=='enhanced-determinism' else 'enhanced-determinism') not in x['features']
   artifacts.append(x['target']['name'])
 assert set(artifacts)=={'rapier2d','rapier3d','parry2d','parry3d'}
assert (E/'normal-graph.stdout').read_text().strip().startswith('incant_physics_snapshot v0.0.1')
assert len((E/'normal-graph.stdout').read_text().splitlines())==1
assert len((E/'construction-normal-graph.stdout').read_text().splitlines())==1
assert (E/'construction-normal-graph.stdout').read_text().startswith('incant_physics_snapshot v0.0.1')
ids=lambda p:{(x['name'],x['version'],x.get('source')):x.get('checksum') for x in tomllib.loads(p.read_text())['package']}
for label in ['default','fast8']:
 P=E/('native-workspace-'+label);copies=read(P/'source-copies.json');assert len(copies['sources'])==766 and copies['root_lock_sha256']==h(R/'Cargo.lock')
 fixture=read(E/('fixture-binding-'+label+'.json'));assert fixture['mode']==label and len(fixture['rows'])==16
 allowed={x['copy']:x for x in fixture['rows']}
 for x in fixture['rows']:
  assert h(x['source'])==h(x['copy'])==x['sha256']
  if label=='fast8':assert h(x['producer_copy'])==x['sha256']
 for row in copies['sources']:
  a=Path(row['source']).read_bytes();b=Path(row['copy']).read_bytes();assert sha(a)==row['source_sha256']
  if label=='fast8' and row['copy'] in allowed:
   x=allowed[row['copy']];assert sha(a)==x['before_sha256']==row['copy_sha256'];assert sha(b)==x['sha256']
  elif row['manifest_path_only']:
   assert sha(b)==row['copy_sha256'];old='path = "../../crates/incant_physics_snapshot"';new='path = '+json.dumps(str(R/'crates/incant_physics_snapshot'));assert a.decode().count(old)==1 and a.decode().replace(old,new)==b.decode()
  else:assert sha(b)==row['copy_sha256'] and a==b
 capture=read(P/'capture-sources.json');assert len(capture['sources'])==32
 for row in capture['sources']:
  assert h(row['source'])==row['source_sha256']
  if 'copy' in row:assert h(row['copy'])==row['copy_sha256']
 a=ids(R/'Cargo.lock');b=ids(P/'Cargo.lock');assert len(a)==968 and len(set(a)&set(b))==88 and all(a[k]==b[k] for k in set(a)&set(b))
 assert set(b)-set(a)=={('bincode','1.3.3','registry+https://github.com/rust-lang/crates.io-index'),('oorandom','11.1.5','registry+https://github.com/rust-lang/crates.io-index'),('incant-private-construction-native-tests','0.0.0',None)}
binaries=read(E/'binaries.json');assert len(binaries)==10,len(binaries)
for row in binaries:assert h(row['path'])==row['sha256'] and Path(row['path']).stat().st_size==row['bytes']
archives=[]
for name in ['physics-native-world-construction']:
 manifest=R/('docs/spikes/evidence/'+name+'.json');m=read(manifest);p=manifest.with_name(name+'-logs.json.gz');assert h(p)==m['archive']['sha256'];raw=json.loads(gzip.decompress(p.read_bytes()));rows=raw['records'] if isinstance(raw,dict) else raw
 assert len(rows)==m['archive']['records']
 for row in rows:
  content=base64.b64decode(raw['payloads'][row['sha256']] if isinstance(raw,dict) else row['base64']);assert sha(content)==row['sha256'] and len(content)==row['bytes'];assert Path(row['path']).read_bytes()==content
 assert manifest.read_bytes()==git('show',PRODUCER+':'+str(manifest.relative_to(R)))
 archives.append(dict(path=str(p.relative_to(R)),sha256=h(p),records=len(rows)))
 assert len(m['binaries'])==4
 for binary in m['binaries']:
  assert binary['source_generation']==3 and binary['executed_log'] in ['focused-final-det-003.log','focused-final-fast8-003.log']
  assert h(binary['copy'])==binary['sha256']
 assert 'overwritten' in m['limits'][0]
reviews={'source':('physics-native-world-main241-source-root-20261013.json','f2e0d9d13cad69ae283a0049a71ffd082a99d3bd6b6735affc74326463eb3569'),'producer':('physics-native-world-final-root-20261013.json','1bc3326504f99dc64419777300385efc661e58d620b58ebcbe1f1a06b962d3a7')}
for name,digest in reviews.values():assert h(Path('/Users/andreyyurlov/.codex/incant-reviews')/name)==digest
paths=sorted(p for p in E.rglob('*') if p.is_file() and 'binaries' not in p.relative_to(E).parts and '__pycache__' not in p.relative_to(E).parts)
paths += [Path('/Users/andreyyurlov/.codex/incant-reviews')/n for n,_ in reviews.values()]
records=[];payloads={}
for p in paths:
 bb=p.read_bytes();d=sha(bb);records.append(dict(path=str(p),sha256=d,bytes=len(bb)));payloads[d]=base64.b64encode(bb).decode()
manifest=R/'docs/spikes/evidence/physics-native-world-main241.json';archive=D/'records.json.gz'
if not verify:
 D.mkdir(parents=True,exist_ok=True);raw=(json.dumps(dict(records=records,payloads=payloads),sort_keys=True,separators=(',',':'))+'\n').encode();archive.write_bytes(gzip.compress(raw,mtime=0));assert archive.stat().st_size<24*1024*1024
 report=dict(schema='incant-physics-native-world-current-main-v1',at=datetime.datetime.now(datetime.UTC).isoformat(),base=BASE,source=SOURCE,producer=PRODUCER,freeze=dict(path=str(E/'freeze.json'),sha256=h(E/'freeze.json'),inputs=3699),source_review=review,checks=checks,counts=counts,features=features,binaries=binaries,producer_archives=archives,reviews=reviews,native_copies=1532,root_lock_identities=968,selected_root_identities=88,world_steps_per_dimension_per_mode=176,archive=dict(path=str(archive.relative_to(R)),sha256=h(archive),raw_sha256=sha(raw),records=len(records),payloads=len(payloads)),limits=['Private captured primitive-world reconstruction only; no public bytes-to-live installer or runtime save admission.', 'No reconstructed physical owner is a trusted clone in the new world controls. The inherited island/narrow controls still retain their documented clone scope. A trusted Rayon pool Arc remains shared execution policy.', 'Deterministic inherited controls use checked-in fixtures; fast8 uses sixteen accepted island/narrow captures with explicit hashes. New world controls capture fresh actual owners in each current numerical mode.', 'Original producer source002 broad binaries were not retained; historical broad logs remain valid and only four source003 focused binaries have retained executed identities. Current composed broad tests have separate immediately copied executables.', 'Events compare exact bit records as multisets, not parallel callback scheduling order. Capture provenance is harness-only; ICNATIV1 is not authentication.', 'Wake tick7, nonempty or empty-but-used joint/soft families, compounds/custom state, deferred work, quarantine, zero dt and pending journals remain refused. Outer ECS, stable IDs, external event queues and public save integration remain separate.', 'No GPU, phone or performance claim. Allocation requests are logical, not RSS/allocator overhead. Scoped vendored library Clippy only.'])
 manifest.write_text(json.dumps(report,indent=2)+'\n')
else:
 report=read(manifest);assert report['checks']==checks and report['counts']==counts and report['features']==features
 info=report['archive'];assert h(R/info['path'])==info['sha256'];raw=gzip.decompress((R/info['path']).read_bytes());assert sha(raw)==info['raw_sha256'];z=json.loads(raw);assert len(z['records'])==info['records'] and len(z['payloads'])==info['payloads']
 for row in z['records']:
  bb=base64.b64decode(z['payloads'][row['sha256']]);assert sha(bb)==row['sha256'] and len(bb)==row['bytes'] and Path(row['path']).read_bytes()==bb
print(json.dumps(dict(verified=True,checks=len(checks),counts=counts,inputs=3699,source_files=174,source_copies=1532,root_identities=968,world_steps_each=176,archive=report['archive']),indent=2))
