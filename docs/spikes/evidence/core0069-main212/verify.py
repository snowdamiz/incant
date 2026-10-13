"""Verify all lossless records and original external identities. No execution or image decoding."""
from pathlib import Path
import argparse,gzip,hashlib,json,tarfile
p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);a=p.parse_args();R=a.root
M=json.loads((R/'docs/spikes/evidence/core0069-main212.json').read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
archive=R/M['archive']['path'];assert sha(archive.read_bytes())==M['archive']['sha256']
map_path=R/M['artifact_map']['path'];assert sha(map_path.read_bytes())==M['artifact_map']['sha256']
raw=gzip.decompress(map_path.read_bytes());assert sha(raw)==M['artifact_map']['raw_sha256'];rows=json.loads(raw)
with tarfile.open(archive,'r:gz') as t:
 payload={m.name:t.extractfile(m).read() for m in t.getmembers() if m.isfile()}
assert len(payload)==M['archive']['unique_payloads']
for name,data in payload.items():assert name=='payload/'+sha(data)
for row in rows:
 data=b''.join(payload[x] for x in row['chunks']) if 'chunks' in row else payload[row['payload']]
 assert len(data)==row['bytes'] and sha(data)==row['sha256'],row['path']
 assert sha(Path(row['original']).read_bytes())==row['sha256'],row['path']
for row in M['external_artifacts']:
 p=Path(row['original']);assert p.stat().st_size==row['bytes'] and sha(p.read_bytes())==row['sha256']
print(json.dumps({'passed':True,'records':len(rows),'payloads':len(payload),'external_artifacts':len(M['external_artifacts'])},indent=2))
