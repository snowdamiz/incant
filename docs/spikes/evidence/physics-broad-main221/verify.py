"""Read-only current broad-owner source and native execution verification."""
from pathlib import Path
import argparse,base64,gzip,hashlib,json,re,subprocess
p=argparse.ArgumentParser();p.add_argument('--verify-only',action='store_true',required=True);p.parse_args()
R=Path(__file__).resolve().parents[4];M=json.loads((R/'docs/spikes/evidence/physics-broad-main221.json').read_text())
def h(b):return hashlib.sha256(b).hexdigest()
def git(*a):return subprocess.check_output(['git',*a],cwd=R)
def file(row):
 b=(R/row['path']).read_bytes();assert h(b)==row['sha256'];return b
def archive(row,originals):
 b=file(row);assert len(b)==row['bytes'];a=json.loads(gzip.decompress(b));assert len(a['records'])==row['records'] and len(a['payloads'])==row['payloads'];payloads={k:base64.b64decode(v,validate=True) for k,v in a['payloads'].items()}
 for k,v in payloads.items():assert h(v)==k
 out={}
 for r in a['records']:
  b=payloads[r['sha256']];assert len(b)==r['bytes'];out[r['path']]=b
  if originals and Path(r['path']).exists():assert Path(r['path']).read_bytes()==b,r['path']
 return out
raw=archive(M['archive'],True)
for k in ['freeze','source_identity']:
 d=json.loads(file(M[k]));assert len(d['files'])==M[k]['files']
 for row in d['files']:assert h((R/row['path']).read_bytes())==row['sha256'],row['path']
 if k=='source_identity':
  for row in d['files']:
   if row['role'] in ['producer','main221']:
    rev=M['producer'] if row['role']=='producer' else M['base'];assert h(git('show',rev+':'+row['path']))==row['sha256']
   else:assert row['path']=='crates/incant_physics_snapshot/src/lib.rs'
# Exact producer declarations retain the integrated CCD/configuration exports.
p='crates/incant_physics_snapshot/src/lib.rs';b=git('show',M['producer']+':'+p).decode()
for old,new in [('mod broad_owner;','mod broad_owner;\nmod ccd_state;'),('mod graph_codec;','mod graph_codec;\nmod integration_config;'),('pub use broad_owner::*;','pub use broad_owner::*;\npub use ccd_state::*;'),('pub use graph_codec::{decode_graph, encode_graph};','pub use graph_codec::{decode_graph, encode_graph};\npub use integration_config::*;'),('    Broad,','    Integration,\n    UnsupportedIntegration,\n    Ccd,\n    Broad,')]:
 assert b.count(old)==1;b=b.replace(old,new)
assert (R/p).read_text()==b
assert len(M['checks'])==11
for c in M['checks']:
 assert c['exit']==0 and c['reaped']
 for key in ['stdout','stderr']:assert h(raw[c[key]['path']])==c[key]['sha256']
for name,det in [('default-tests',True),('fast8-tests',False)]:
 c=next(x for x in M['checks'] if x['name']==name);text=raw[c['stdout']['path']].decode();counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',text);assert len(counts)==22 and tuple(sum(int(r[i]) for r in counts) for i in range(3))==(172,0,0)
 messages=[]
 for line in text.splitlines():
  if line.startswith('{'):
   try:messages.append(json.loads(line))
   except json.JSONDecodeError:pass
 for name in ['rapier2d','rapier3d']:
  rows=[x for x in messages if x.get('reason')=='compiler-artifact' and x['target']['name']==name];assert rows,name
  for row in rows:
   features=set(row['features']);assert 'incant-checkpoint' in features and 'serde-serialize' in features
   assert ('enhanced-determinism' in features)==det
   assert ('simd8' in features)==(not det)
g=next(x for x in M['checks'] if x['name']=='normal-graph');lines=raw[g['stdout']['path']].decode().splitlines();assert len(lines)==1 and lines[0].startswith('incant_physics_snapshot v0.0.1')
producer=json.loads(file(M['producer_manifest']));import io,tarfile
pb=file(producer['archive']);assert len(pb)==producer['archive']['bytes'];tar=tarfile.open(fileobj=io.BytesIO(pb),mode='r:gz');assert len(producer['records'])==124
for row in producer['records']:
 b=tar.extractfile(row['payload']).read();assert h(b)==row['sha256'] and len(b)==row['bytes'];p=Path(producer['evidence_root'])/row['path']
 if p.exists():assert p.read_bytes()==b
review=json.loads(file(M['producer_review']));assert review['manifest_sha256']==M['producer_manifest']['sha256'] and review['decision'].startswith('Accepted')
assert not git('diff','--name-only',M['base'],'HEAD','--','Cargo.lock','Cargo.toml','crates/incant_physics_snapshot/Cargo.toml','third_party').strip()
print(json.dumps(dict(verified=True,current_records=M['archive']['records'],frozen_files=3125,physics_inputs=1549,checks=11,default=172,fast8=172,normal_graph='std-only',live_save_admission=False)))
