from pathlib import Path
import hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[4];DEST=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest();git=lambda *a:subprocess.check_output(['git',*a],cwd=ROOT)
producer='922308128de18b8a82882d0b9112f6f3ff0f14dd';base='8c1db376fc8c3e6651a353112c58ba66515f6733';main='70e7aa2ee25f49fe6fcf7e67264a559cc60d33b0'
subprocess.run(['git','merge-base','--is-ancestor',main,'HEAD'],cwd=ROOT,check=True)
scopes=['crates','tools','Cargo.toml','Cargo.lock','.github','.cargo','.gitattributes','sdk','schemas','third_party']
paths=git('diff','--name-only',producer,'HEAD','--',*scopes).decode().splitlines();assert paths==['crates/incant_wasm_runtime/Cargo.toml'],paths
path=paths[0];assert git('show','HEAD:'+path)==git('show',main+':'+path)
assert git('diff',producer,'HEAD','--',path)==git('diff',base,main,'--',path)
freeze=json.loads((ROOT/'docs/spikes/evidence/physics-world-continuation/source-freeze.json').read_text())
for row in freeze['files']:assert sha((ROOT/row['path']).read_bytes())==row['sha256']==sha(git('show',producer+':'+row['path']))
normal=(DEST/'selected-normal-graph.txt').read_text();assert 'incant_wasm_runtime ' not in normal and 'embedded-fixture' not in normal
review=DEST/'producer-final-review.json';assert sha(review.read_bytes())=='dd4b9380714988c7c4b6f1ef449f01eed4222492fb7e7dc7ebf93ed3e59ae1a1'
run=subprocess.run([sys.executable,str(ROOT/'docs/spikes/evidence/physics-world-continuation/collect.py'),'--verify-only'],cwd=ROOT,capture_output=True);assert run.returncode==0,run.stderr.decode();summary=json.loads(run.stdout)
result={'producer':producer,'main':main,'base':base,'frozen_inputs':len(freeze['files']),'all_frozen_inputs_exact':True,'changed_build_source_paths':paths,'old_new_path_hashes':{'old':sha(git('show',producer+':'+path)),'new':sha(git('show',main+':'+path))},'normal_graph_sha256':sha(normal.encode()),'collector_summary':summary,'producer_final_review_sha256':sha(review.read_bytes()),'relevance':'The only added executable/build input is the reviewed embedded-fixture cc/parallel feature. That package/feature is absent from the tested snapshot, physics runtime and script normal graph. Physics sources/tests, all five locks, root manifests and compatibility build script remain exact. No new Rust execution claimed.','live_restore_or_save':False}
if '--verify-only' in sys.argv:assert json.loads((DEST/'qualification.json').read_text())==result
else:(DEST/'qualification.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
