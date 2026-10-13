from pathlib import Path
import base64,datetime,gzip,hashlib,json,subprocess,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');d=r/'docs/spikes/evidence/physics-attachments-main197';h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();sha=lambda b:hashlib.sha256(b).hexdigest();g=lambda *a:subprocess.check_output(['git',*a],cwd=r);read=lambda p:json.loads(Path(p).read_text())
old='487037a32c729a99af743214033b9f854d45b2bb';main='14fa7675168a0fd2d1845fa5e7b9b02352f1e9f3';p=read(r/'docs/spikes/evidence/physics-attachments-main197.json');f=read(d/'freeze.json')
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=r,check=True)
scope=['crates','editor/app','tools','sdk','schemas','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','.gitattributes','.github']
changed=g('diff','--name-only',old,'HEAD','--',*scope).decode().splitlines();accepted=read(r/'docs/spikes/evidence/clusters-drs-main197.json');expected=[x['path'] for x in accepted['sources']];assert sorted(changed)==sorted(expected) and len(changed)==28
rows=[]
for path in changed:
 current=(r/path).read_bytes();assert current==g('show',main+':'+path)
 x=subprocess.run(['git','show',old+':'+path],cwd=r,capture_output=True);oldbytes=x.stdout if x.returncode==0 else None
 rows.append({'path':path,'old_sha256':sha(oldbytes) if oldbytes is not None else None,'current_sha256':sha(current),'current_exact_main':main})
for path,s in f['files'].items():
 assert sha(g('show',f['head']+':'+path))==s,path
 if path not in changed:assert h(r/path)==s,path
 else:assert h(r/path)==sha(g('show',main+':'+path)),path
for row in p['sources']:
 assert h(r/row['path'])==row['current_sha256'],row['path']
assert len(g('diff','--name-only',old,'HEAD','--','Cargo.toml','Cargo.lock','crates/incant_physics_snapshot').strip())==0
for row in p['checks']:
 assert row['exit']==0
 for k in ['stdout','stderr']:assert h(row[k]['path'])==row[k]['sha256']
a=json.loads(gzip.decompress((r/p['archive']['path']).read_bytes()));assert h(r/p['archive']['path'])==p['archive']['sha256']
for row in a['records']:
 b=base64.b64decode(a['payloads'][row['sha256']]);assert len(b)==row['bytes'] and sha(b)==row['sha256'] and Path(row['original']).read_bytes()==b
review=Path.home()/'.codex/incant-reviews/physics-attachments-main197-final-20261013.json';assert h(review)=='cb8e6f6e935808eb8eac32cd96012c40df4f1ac077abc78ae659a12b6b0302ed'
result={'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'accepted_review_head':old,'actual_main':main,'current_head':g('rev-parse','HEAD').decode().strip(),'passed':True,'original_frozen_inputs':len(f['files']),'unchanged_attachment_source_rows':len(p['sources']),'changed_paths':rows,'source_relevance':'Exactly the28 independently accepted renderer/CI paths from PR198 differ; snapshot source, root/package manifests, Cargo.lock, all dependencies and prior normal-only graph are unchanged. No Rust rebuild claimed.','original_checks':8,'original_counts':p['summary']['counts'],'archive_records':len(a['records']),'new_build_or_gpu_execution':False}
print(json.dumps({k:v for k,v in result.items() if k!='changed_paths'}))
if '--verify-only' not in sys.argv:(d/'main198-qualification.json').write_text(json.dumps(result,indent=2)+'\n')
