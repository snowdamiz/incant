#!/usr/bin/env python3
"""Retain current composition checks without embedding native executables."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

D = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-integration-evidence/quickjs-profile-main274')
digest = lambda b: hashlib.sha256(b).hexdigest()
dump = lambda x: (json.dumps(x, indent=2, sort_keys=True) + '\n').encode()
records, payloads = [], {}
for path in sorted(E.rglob('*')):
    relative = path.relative_to(E)
    if not path.is_file() or 'binaries' in relative.parts or path.name.startswith('verification'):
        continue
    raw = path.read_bytes()
    key = digest(raw)
    records.append({'path': str(relative), 'sha256': key, 'bytes': len(raw)})
    payloads[key] = base64.b64encode(raw).decode()
raw = gzip.compress(dump({'records': records, 'payloads': payloads}), mtime=0)
(D / 'records.json.gz').write_bytes(raw)
freeze = json.loads((E / 'freeze-001.json').read_bytes())
producer = D.parent / 'quickjs-maintained-profile'
manifest = {
    'version': 1, 'source': freeze['source'], 'base': freeze['base'],
    'producer': freeze['producer'], 'inputs': len(freeze['rows']),
    'archive_sha256': digest(raw), 'records': len(records), 'payloads': len(payloads),
    'collector_sha256': digest(Path(__file__).read_bytes()),
    'verifier_sha256': digest((D / 'verify.py').read_bytes()), 'evidence_root': str(E),
    'producer_files': {p.name: digest(p.read_bytes()) for p in sorted(producer.iterdir()) if p.is_file()},
    'producer_review_sha256': digest((E / 'producer-review.json').read_bytes()),
    'scope': 'Current main274 profile off/summary and actual script/player fixtures in both physics modes; thirteen checks and thirteen immediate executable copies. Producer broad tests remain historical. No shipping measurement or performance gate claim.',
}
(D / 'manifest.json').write_bytes(dump(manifest))
print(json.dumps({'manifest_sha256': digest(dump(manifest)), 'records': len(records), 'payloads': len(payloads)}))
