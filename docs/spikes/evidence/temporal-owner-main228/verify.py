from pathlib import Path
import base64, datetime, gzip, hashlib, json, re, shutil, subprocess, sys
R=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
E=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/owner-attachment-main228-001')
D=R/'docs/spikes/evidence/temporal-owner-main228'
MAIN='b190d826a3df02466df18bf860f127228d375016'
SOURCE='a692ef6e8fea2d8e5e0870e2c0676c40a0a788cd'
PRIVATE='4c4310874405f569b60e32a0d8d5f21073546c9d'
sha=lambda b:hashlib.sha256(b).hexdigest()
h=lambda p:sha(Path(p).read_bytes())
read=lambda p:json.loads(Path(p).read_text())
git=lambda *a:subprocess.check_output(['git',*a],cwd=R)
verify='--verify-only' in sys.argv

def assert_archive(path, info):
 z=Path(path).read_bytes();assert sha(z)==info['sha256'];raw=gzip.decompress(z);assert sha(raw)==info['raw_sha256'];a=json.loads(raw)
 assert len(a['records'])==info['records'] and len(a['payloads'])==info['payloads']
 for row in a['records']:
  b=base64.b64decode(a['payloads'][row['sha256']]);assert sha(b)==row['sha256'] and len(b)==row['bytes']
 return a

f=read(E/'freeze.json');sf=read(E/'source-freeze.json')
assert f['head']==SOURCE and f['base']==MAIN and len(f['files'])==3545
assert h(E/'source-freeze.json')=='1c99b960538a2dd09d94882c5ba15f84b6d5582667cfae9f257f488c34530866'
for row in f['files']:
 assert h(R/row['path'])==row['sha256']==sha(git('show',SOURCE+':'+row['path'])),row['path']
assert git('merge-base','--is-ancestor',MAIN,'HEAD')==b''
assert subprocess.run(['git','merge-base','--is-ancestor',PRIVATE,'HEAD'],cwd=R).returncode==1
assert len(sf['files'])==18 and len(sf['baseline'])==204
source_paths=[]
for row in sf['files']:
 p=row['path'];source_paths.append(p);assert h(R/p)==h(E/'source'/p)==row['sha256'];b=git('show','613ee59a:'+p)
 assert sha(b)==row['producer_sha256'] and (b==(R/p).read_bytes())==row['producer_exact']
for row in sf['baseline']:assert h(R/row['path'])==row['sha256']==sha(git('show',MAIN+':'+row['path']))
assert sorted(git('diff','--name-only',MAIN,SOURCE,'--','crates').decode().splitlines())==source_paths
scope=['tools','sdk','Cargo.toml','Cargo.lock','.cargo','.github','third_party','rust-toolchain.toml']
assert git('diff','--name-only',MAIN,SOURCE,'--',*scope)==b''
patch=git('diff','--binary',sf['documentation_head'],SOURCE,'--','crates');assert sha(patch)==sf['patch_sha256']==h(E/'integration-source.patch')
assert not (R/'crates/incant_render/src/upscaling').exists() and not (R/'crates/incant_render/src/metalfx').exists()
shaders=git('ls-tree','-r','--name-only',MAIN,'--','crates/incant_render').decode().splitlines()
for p in shaders:
 if p.endswith('.wgsl'):assert (R/p).read_bytes()==git('show',MAIN+':'+p)
assert (R/'crates/incant_render/src/motion/owner.wgsl').read_bytes()==git('show','613ee59a:crates/incant_render/src/motion/owner.wgsl')

initial=read(E/'results.json')
assert len(initial)==11 and initial[-1]['name']=='partial-cooked-refusal'
assert 'test result: ok. 0 passed;' in Path(initial[-1]['stdout']['path']).read_text()
assert read(E/'initial-controller.json')['exit']==1
for row in initial:
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
checks=initial[:-1]+read(E/'results-resume.json')
expected=['scoped-renderer-clean','shader-cooker','default-cpu','default-clippy','cooked-cpu','offline-recook','cooked-clippy','shader-clippy','native-window-consumer','wasm','partial-test-list','partial-cooked-refusal-exact','normal-graph','runtime-guard','fmt','conventions','diff']
assert [x['name'] for x in checks]==expected
for row in checks:
 assert row['exit']==0 and row['reaped']
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
counts={}
for name in ['shader-cooker','default-cpu','cooked-cpu','partial-cooked-refusal-exact']:
 row=next(x for x in checks if x['name']==name);text=Path(row['stdout']['path']).read_text()
 groups=[list(map(int,m)) for m in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text)]
 assert groups and all(g[1]==0 for g in groups)
 counts[name]=dict(passed=sum(x[0] for x in groups),failed=0,ignored=sum(x[2] for x in groups),groups=len(groups),nonempty_groups=sum(x[0]>0 for x in groups))
assert counts['partial-cooked-refusal-exact']['passed']==1
assert counts['default-cpu']['passed']==146 and counts['default-cpu']['ignored']==104
assert counts['cooked-cpu']['passed']==175 and counts['cooked-cpu']['ignored']==104
assert counts['shader-cooker']['passed']==33
binaries=read(E/'binaries.json');assert len(binaries)==4
for row in binaries:assert h(row['path'])==row['sha256'] and Path(row['path']).stat().st_size==row['bytes']
off=read(E/'offline/result.json');assert off['complete'] and not off['gpu_execution'] and len(off['commands'])==9
assert all(c['exit_code']==0 for c in off['commands'])
for row in off['artifacts']:assert h(E/'offline'/row['path'])==row['sha256'] and (E/'offline'/row['path']).stat().st_size==row['bytes']
assert len(off['old_payload_equality'])==27
for row in off['old_payload_equality']:assert h(row['prior_path'])==row['sha256']==h(E/'offline'/row['path'])
assert h(off['baseline_report']['path'])==off['baseline_report']['sha256']

pm=read(R/'docs/spikes/evidence/temporal-owner-attachment.json');pa=assert_archive(R/pm['archive']['path'],pm['archive'])
assert len(pa['records'])==213 and len(pa['payloads'])==127
owner_payloads=[row for row in pa['records'] if '/offline-owner-001/' in row['path'] and row['path'].endswith('.shader')]
assert len(owner_payloads)==6
for row in owner_payloads:
 current=E/'offline/temporal-motion-owner'/Path(row['path']).name
 assert current.read_bytes()==base64.b64decode(pa['payloads'][row['sha256']])
assert pm['counts']['default-cpu']==[[143,0,41]] and pm['counts']['cooked-native-cpu']==[[158,0,42]]
assert (R/'docs/spikes/evidence/temporal-owner-attachment.json').read_bytes()==git('show',PRIVATE+':docs/spikes/evidence/temporal-owner-attachment.json')
for row in pm['binaries']:assert h(row['copy'])==row['sha256'] and Path(row['copy']).stat().st_size==row['bytes']
gpu_path=R/'handoffs/0073-upscaling-review/owner-attachment-numeric-evidence.json';gpu=read(gpu_path)
assert gpu_path.read_bytes()==git('show','bf39fd83:'+str(gpu_path.relative_to(R)))
assert gpu['verdict']['accepted'] and all(gpu['verdict']['checks'].values())
assert len(gpu['files'])==14 and gpu['child']['exit']==0 and gpu['child']['reaped'] and gpu['child']['group_empty_after']
for row in gpu['files']:
 b=row['content'].encode() if row['encoding']=='utf-8' else base64.b64decode(row['content'])
 assert len(b)==row['bytes'] and sha(b)==row['sha256'] and Path(row['path']).read_bytes()==b
reviews={
 'source':('docs/spikes/evidence/temporal-owner-main228-source-independent.json','fcfc04e6e9a3afccc616ff8f425b08693515dab3f3a59daf31785170767b65ab'),
 'producer_cpu':('docs/spikes/evidence/temporal-owner-attachment-final-cpu-root.json','d232a2241667c7258a4211489b851b2b7bdb7f72bf72b245d613fa678d23d3e2'),
 'producer_gpu':('docs/spikes/evidence/temporal-owner-attachment-gpu-root.json','ad7cde4b3991e3f80040ccb04a97ba81ab5ba1d84d17b6ff84fc1cc125be7954')}
for p,digest in reviews.values():assert h(R/p)==digest
# Separate release evidence is verified against actual objects, not renderer output.
subprocess.run(['python3',str(R/'docs/spikes/evidence/release-checkpoint-24/verify.py')],cwd=R,check=True)

paths=sorted(p for p in E.rglob('*') if p.is_file() and 'binaries' not in p.relative_to(E).parts and '__pycache__' not in p.relative_to(E).parts)
paths += [Path(__file__),Path(off['baseline_report']['path'])]
records=[];payloads={}
for p in paths:
 b=p.read_bytes();digest=sha(b);records.append(dict(path=str(p),sha256=digest,bytes=len(b)));payloads[digest]=base64.b64encode(b).decode()
manifest_path=R/'docs/spikes/evidence/temporal-owner-main228.json';archive_path=D/'artifacts.json.gz'
if not verify:
 D.mkdir(parents=True,exist_ok=True)
 raw=(json.dumps(dict(records=records,payloads=payloads),sort_keys=True,separators=(',',':'))+'\n').encode();archive_path.write_bytes(gzip.compress(raw,mtime=0))
 report=dict(schema='incant-temporal-owner-current-main-v1',at=datetime.datetime.now(datetime.UTC).isoformat(),base=MAIN,source=SOURCE,producer=PRIVATE,
  freeze=dict(path=str(E/'freeze.json'),sha256=h(E/'freeze.json'),inputs=len(f['files'])),source_freeze=dict(path=str(E/'source-freeze.json'),sha256=h(E/'source-freeze.json')),
  source_paths=source_paths,counts=counts,checks=checks,binaries=binaries,retained_nonqualifying_commands=initial[-1:],initial_controller=read(E/'initial-controller.json'),offline=dict(path=str(E/'offline/result.json'),sha256=h(E/'offline/result.json'),commands=9,old_exact_payloads=27,owner_exact_producer_payloads=6),
  producer_evidence=dict(records=213,payloads=127,numeric_records=14,render_submissions=12,readback_submissions=12,source_specific=True),reviews=reviews,
  archive=dict(path=str(archive_path.relative_to(R)),sha256=h(archive_path),raw_sha256=sha(raw),records=len(records),payloads=len(payloads)),
  limits=['No current GPU execution or appearance; producer numeric proof is source-specific attachment substrate only.','Main owner-only path excludes private upscaler/MetalFX ancestry; fixed TAA combination refused.','Old 27 native shader payloads exact; HLSL retained without DXC validation.','Logical accepted/candidate allocation bounds exclude queued physical GPU residency.','Checkpoint24 release evidence separately selects main226, not this renderer change.'])
 manifest_path.write_text(json.dumps(report,indent=2)+'\n')
else:
 report=read(manifest_path);assert report['source']==SOURCE and report['counts']==counts and report['checks']==checks
 archive=assert_archive(R/report['archive']['path'],report['archive'])
 for row in archive['records']:assert h(row['path'])==row['sha256'],row['path']
print(json.dumps(dict(verified=True,source=SOURCE,inputs=3545,checks=len(checks),counts=counts,old_payloads=27,owner_payloads=6,records=report['archive']['records'],payloads=report['archive']['payloads'],gpu='producer-only'),indent=2))
