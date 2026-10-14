#!/usr/bin/env python3
"""Package immutable current-main receipts; test executables remain external."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
D=Path(__file__).resolve().parent
E=Path('/Users/andreyyurlov/.codex/incant-integration-evidence/project-pin-main273')
h=lambda b:hashlib.sha256(b).hexdigest()
dump=lambda v:(json.dumps(v,indent=2,sort_keys=True)+'\n').encode()
records=[];payloads={}
for p in sorted(E.rglob('*')):
    if not p.is_file() or 'binaries' in p.relative_to(E).parts or p.name.startswith('verification'):continue
    b=p.read_bytes();digest=h(b);records.append({'path':str(p.relative_to(E)),'sha256':digest,'bytes':len(b)})
    payloads[digest]=base64.b64encode(b).decode()
raw=gzip.compress(dump({'records':records,'payloads':payloads}),mtime=0);(D/'records.json.gz').write_bytes(raw)
f=json.loads((E/'freeze-005.json').read_bytes())
m={'version':1,'source':f['source'],'base':f['base'],'producer':f['producer'],'archive_sha256':h(raw),
   'records':len(records),'payloads':len(payloads),'verifier_sha256':h((D/'verify.py').read_bytes()),
   'collector_sha256':h(Path(__file__).read_bytes()),'evidence_root':str(E),
   'producer_evidence_root':'/Users/andreyyurlov/.codex/incant-evidence/project-pin-migration-001',
   'producer_manifest_sha256':h((D.parent/'project-pin-migration/manifest.json').read_bytes()),
   'producer_review_sha256':h((E/'producer-review.json').read_bytes()),
   'windows_source_review_sha256':h((E/'windows-source-review.json').read_bytes()),
   'scope':'Host-only legacy-unpinned exact-current file migration plus preserved prefab authoring. Source001:129 cases/nine successful checks. Test-only successor004:19 affected cases;005:four overlapping cases and strict lint. Retained Windows failure; no current Windows execution or release/phase acceptance.'}
(D/'manifest.json').write_bytes(dump(m));print(json.dumps({'manifest_sha256':h(dump(m)),'records':len(records),'payloads':len(payloads)}))
