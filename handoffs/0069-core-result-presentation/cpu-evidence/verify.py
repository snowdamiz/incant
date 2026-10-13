"""Independent read-only verification of the retained 0069 CPU packet."""
from pathlib import Path
import base64,gzip,hashlib,json
HERE=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=json.loads((HERE/'manifest.json').read_text())
archive=(HERE/'artifacts.json.gz').read_bytes()
assert sha(archive)==manifest['archive']['sha256'] and len(archive)==manifest['archive']['bytes']
value=json.loads(gzip.decompress(archive));assert len(value['records'])==manifest['record_count'] and len(value['payloads'])==manifest['payload_count']
for row in value['records']:
 b=base64.b64decode(value['payloads'][row['sha256']],validate=True)
 assert len(b)==row['bytes'] and sha(b)==row['sha256'],row['path']
 actual=Path(row['path']).read_bytes();assert actual==b,row['path']
for row in manifest['external_files']:
 p=Path(row['path']);assert p.stat().st_size==row['bytes'] and sha(p.read_bytes())==row['sha256'],row['path']
print(json.dumps({'passed':True,'manifest_sha256':sha((HERE/'manifest.json').read_bytes()),'archive_sha256':sha(archive),'lossless_records':len(value['records']),'payloads':len(value['payloads']),'external_files':len(manifest['external_files'])}))
