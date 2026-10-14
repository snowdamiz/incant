#!/usr/bin/env python3
"""Retain current application composition checks; native executables stay external."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

D = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-integration-evidence/project-application-main275')
h = lambda b: hashlib.sha256(b).hexdigest()
dump = lambda x: (json.dumps(x, indent=2, sort_keys=True) + '\n').encode()
records, payloads = [], {}
for path in sorted(E.rglob('*')):
    relative = path.relative_to(E)
    if not path.is_file() or ('binaries' in relative.parts or relative.parts[0] == 'producer-source') or path.name.startswith('verification'):
        continue
    raw = path.read_bytes()
    key = h(raw)
    records.append({'path': str(relative), 'sha256': key, 'bytes': len(raw)})
    payloads[key] = base64.b64encode(raw).decode()
raw = gzip.compress(dump({'records': records, 'payloads': payloads}), mtime=0)
(D / 'records.json.gz').write_bytes(raw)
f = json.loads((E / 'freeze-source-001.json').read_bytes())
producer = D.parent / 'project-file-application'
m = {
    'version': 1, 'source': f['source'], 'base': f['base'], 'producer': f['producer'],
    'inputs': len(f['rows']), 'successor': json.loads((E / 'test-isolation-002/freeze.json').read_bytes())['source'], 'archive_sha256': h(raw), 'records': len(records),
    'payloads': len(payloads), 'collector_sha256': h(Path(__file__).read_bytes()),
    'verifier_sha256': h((D / 'verify.py').read_bytes()), 'evidence_root': str(E),
    'producer_files': {p.name: h(p.read_bytes()) for p in sorted(producer.iterdir()) if p.is_file()},
    'scope': 'Fresh current-main275 application correctness: 107 source/command, 39 headless, four editor and 189 bridge cases; 14 original checks plus five test-only isolation successor checks with39 parallel unit/prefab cases, 78 exact schemas and 19 frozen editor assets. Historical producer evidence remains separate. No GPU, UI appearance, performance or phase claim.',
}
(D / 'manifest.json').write_bytes(dump(m))
print(json.dumps({'manifest_sha256': h(dump(m)), 'records': len(records), 'payloads': len(payloads)}))
