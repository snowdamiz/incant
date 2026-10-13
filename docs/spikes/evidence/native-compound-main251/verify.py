"""Read-only main251 join qualification; never builds or executes native code."""
import argparse, base64, gzip, hashlib, importlib.util, json, subprocess
from pathlib import Path
if not __debug__: raise RuntimeError('assertions required')
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
D=Path(__file__).resolve().parent;R=D.parents[3]
M=json.loads((D/'manifest.json').read_bytes())
h=lambda b:hashlib.sha256(b).hexdigest()
def checked(row):
 b=(R/row['path']).read_bytes();assert h(b)==row['sha256'] and len(b)==row['bytes'];return b
def git(*args):return subprocess.check_output(['git',*args],cwd=R)
Q=json.loads(checked(M['qualification']));checked(M['producer_review'])
A=json.loads(gzip.decompress(checked(M['archive'])));raw={}
for row in A:
 b=base64.b64decode(row['data'],validate=True);assert len(b)==row['bytes'] and h(b)==row['sha256'];assert row['path'] not in raw;raw[row['path']]=b
 if a.originals:assert Path(row['path']).read_bytes()==b
assert len(raw)==M['records']==11
assert Q['producer']=='9b8d58863ac7fede21086c0b7aec25e5f577a1d5'
assert Q['main']=='b2ad9d5acc7a83bbe79e11157da0b9735b17788a'
assert Q['builds_started'] is False and Q['graphs_exact']==3 and len(Q['exceptions'])==1
roots=['crates','third_party','Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo','tools','editor','app','sdk/rust']
assert git('diff','--name-only',Q['producer'],Q['composition'],'--',*roots).decode().splitlines()==['crates/incant_source/src/tests.rs']
for parent in [Q['producer'],Q['main']]:subprocess.run(['git','merge-base','--is-ancestor',parent,'HEAD'],cwd=R,check=True)
ex=Q['exceptions'][0];assert ex['path']=='crates/incant_source/src/tests.rs'
old=git('show',Q['producer']+':'+ex['path']);new=(R/ex['path']).read_bytes()
assert h(old)==ex['old_sha256'] and h(new)==ex['new_sha256'];assert new==git('show',Q['main']+':'+ex['path'])
original_manifest=json.loads(checked(M['producer_manifest']));checked(M['producer_collector'])
producer_archive=R/'docs/spikes/evidence/native-compound-checkpoints/records.json.gz'
assert h(producer_archive.read_bytes())==original_manifest['archive_sha256']
producer_raw={r['path']:base64.b64decode(r['data']) for r in json.loads(gzip.decompress(producer_archive.read_bytes()))}
freeze=json.loads(producer_raw['consumers-001-fast8/freeze.json'])['source'];assert len(freeze)==Q['inputs']==3155
for path,digest in freeze.items():assert h((R/path).read_bytes())==(ex['new_sha256'] if path==ex['path'] else digest),path
assert len(Q['commands'])==8
for c in Q['commands']:
 assert c['exit']==0 and c['reaped'] and h(raw[c['path']])==c['sha256']
 if c['name'] in ['graph-det','graph-fast8','graph-ordinary']:
  current=raw[c['path']].decode().replace('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant','<workspace>')
  prior=producer_raw['qualification-001/'+c['name']+'.log'].decode().replace('/Users/andreyyurlov/.codex/worktrees/physics-failure-isolation/incant','<workspace>')
  assert current==prior
# Replay original immutable assertions against only the explicitly bound old
# authoring test bytes. The actual current file was separately checked above.
# Every other Path read, source, record and executable remains unmodified.
collector=R/M['producer_collector']['path']
spec=importlib.util.spec_from_file_location('compound_producer',collector);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
original_read=Path.read_bytes

def scoped_read(path):
 return old if path==R/ex['path'] else original_read(path)
try:
 Path.read_bytes=scoped_read
 module.verify(a.originals,a.local_binaries)
finally:Path.read_bytes=original_read
assert (R/ex['path']).read_bytes()==new
print(json.dumps({'verified':True,'inputs':len(freeze),'explicit_test_exception':ex['path'],'graphs_exact':3,'checks':8,'records':len(raw),'new_build_or_execution':False}))
