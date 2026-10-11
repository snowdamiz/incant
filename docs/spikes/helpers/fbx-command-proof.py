#!/usr/bin/env python3
"""CPU-only public CLI import/transaction controls. No renderer or project writes."""
import hashlib,json,pathlib,shutil,subprocess,sys
repo=pathlib.Path(sys.argv[1]).resolve(); out=pathlib.Path(sys.argv[2]).resolve();out.mkdir(parents=True,exist_ok=False)
binary=repo/'target/release/incant_headless'; project=out/'model.incant.json'; journal=project.with_suffix('.journal.jsonl')
fixture=repo/'crates/incant_assets/tests/fixtures/fbx'
calls=[]
def run(*args,ok=True,stdin=None):
 r=subprocess.run([str(binary),*map(str,args)],input=stdin,text=True,capture_output=True,timeout=60)
 calls.append({'args':list(map(str,args)),'exit':r.returncode})
 assert (r.returncode==0)==ok,(args,r.stdout,r.stderr)
 return json.loads(r.stdout) if ok else r.stderr.strip()
def doc():return json.loads(project.read_text())
def revision():
 r=run('rpc',project,'--journal',journal,stdin=json.dumps({'id':1,'method':'project.read'})+'\n');return r['result']['revision']
def history(method):
 requests=[{'id':1,'method':method},{'id':2,'method':'project.save'}]
 r=subprocess.run([str(binary),'rpc',str(project),'--journal',str(journal)],input=''.join(json.dumps(v)+'\n' for v in requests),text=True,capture_output=True,check=True,timeout=60)
 rows=[json.loads(v) for v in r.stdout.splitlines()];assert len(rows)==2 and all('error' not in v for v in rows),rows
 calls.append({'args':['rpc',method,'project.save'],'exit':r.returncode})
 return rows[0]['result']
run('init',project,'--name','FBX command proof','--entities',0)
source=out/'cube.FBX'; original=(fixture/'maya_cube_7500_ascii.fbx').read_text();source.write_text(original)
a=run('import',project,'cube.FBX'); first=doc();r1=revision();assert a['changed'] and a['asset']['kind']=='model'
b=run('import',project,'cube.FBX'); assert b['cache_hit'] and not b['changed'] and revision()==r1
source.write_text(original.replace('-0.5','-0.75'))
c=run('import',project,'cube.FBX'); second=doc(); assert c['changed'] and c['asset']['id']==a['asset']['id'] and c['asset']['sha256']!=a['asset']['sha256']
history('history.undo');assert doc()==first
history('history.redo');assert doc()==second
prior={p.name:p.read_bytes() for p in [project,journal]}
source.write_bytes(b'truncated FBX');invalid=run('import',project,'cube.FBX',ok=False)
assert {p.name:p.read_bytes() for p in [project,journal]}==prior
shutil.copy(fixture/'max2009_cube_anim_6100_ascii.fbx',out/'animated.fbx');unsupported=run('import',project,'animated.fbx',ok=False);assert 'animation' in unsupported
assert {p.name:p.read_bytes() for p in [project,journal]}==prior
source.unlink();(out/'animated.fbx').unlink();assert run('validate',project)['valid']
cache=out/'.incant/cache/models'; blobs=[{'path':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(cache.iterdir()) if p.is_file()];assert len(blobs)==2
report={'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'processes':len(calls),'initial':a,'unchanged':b,'reimport':c,'stable_asset_id':True,'undo_redo_exact':True,'rejected_import_project_and_journal_unchanged':True,'sources_deleted':True,'validation_after_deletion':True,'cached_runtime_reader_executed':'separate Rust integration tests','cache':blobs,'invalid_error':invalid,'unsupported_error':unsupported,'calls':calls,'rendering':False,'visual_review':False}
(out/'report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'processes':len(calls),'stable_id':a['asset']['id'],'report':str(out/'report.json')}))
