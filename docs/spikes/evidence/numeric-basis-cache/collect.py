#!/usr/bin/env python3
"""Collect completed correctness evidence without building or executing native code."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-evidence/numeric-basis-cache-001')
rows = []
for path in sorted(E.rglob('*')):
    if not path.is_file() or path.name.startswith('packet-verification'):
        continue
    data = path.read_bytes()
    relative = str(path.relative_to(E))
    row = dict(path=relative, original=str(path), bytes=len(data),
               sha256=hashlib.sha256(data).hexdigest())
    if not relative.startswith('frozen-binaries/'):
        row['payload'] = base64.b64encode(data).decode()
    rows.append(row)
archive = gzip.compress(json.dumps(rows, separators=(',', ':')).encode(), mtime=0)
(HERE / 'records.json.gz').write_bytes(archive)
manifest = dict(version=1, source_commit='66c5d14761caf61f5d2e3be90313bc5dc27bf715',
                base_commit='03bc4b987f3e553249846c4cfce991793d4d2c48',
                archive_sha256=hashlib.sha256(archive).hexdigest(), records=len(rows),
                portable_payloads=sum('payload' in r for r in rows),
                source_freeze='source-002/freeze.json',
                scope='Scoped native correctness and lint; no performance benefit or shipping acceptance yet',
                checks=10, new_tests_per_mode=5, inherited_column_tests_per_mode=7,
                modes=['physics-deterministic', 'physics-fast-simd8'],
                limits=['No shipping comparison, GPU, full runtime suite or gate result.',
                        'Four immediate executable copies are external hash-bound bytes; logs/source are portable.',
                        'Source001 has a test closure mutability omission caught before compilation; source002 fixes only that binding.',
                        'The cache is per-call fixed stack storage, not saved state; all validation and parent/root multiplication remain unchanged.',
                        'Cursor/inspector candidates from the failed combined experiment are excluded.'])
(HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps({k:manifest[k] for k in ['source_commit','archive_sha256','records','portable_payloads']}))
