#!/usr/bin/env python3
"""Retain the standalone QuickJS functional experiment, including failed attempts."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path.home() / '.codex/incant-evidence/quickjs-f64-store-001'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    rows, payloads = [], {}
    for path in sorted(E.rglob('*')):
        if not path.is_file():
            continue
        data = path.read_bytes()
        name = path.relative_to(E).as_posix()
        embedded = '/binaries/' not in name and path.suffix not in ('.o', '.a') and len(data) <= 8 * 1024 * 1024
        digest = sha(data)
        rows.append({'name': name, 'original': str(path), 'sha256': digest,
                     'bytes': len(data), 'mode': path.stat().st_mode & 0o777,
                     'embedded': embedded})
        if embedded:
            payloads[digest] = base64.b64encode(data).decode('ascii')
    archive = gzip.compress(json.dumps({'records': rows, 'payloads': payloads},
                                      sort_keys=True, separators=(',', ':')).encode(), mtime=0)
    (HERE / 'records.json.gz').write_bytes(archive)
    final = json.loads((E / 'execution-004/result.json').read_text())
    manifest = {'schema': 'incant-quickjs-f64-functional-v1', 'source': final['source'],
                'scope': final['scope'], 'final_execution': 'execution-004',
                'archive_sha256': sha(archive), 'records': len(rows), 'payloads': len(payloads),
                'external_scope': 'Retained executables and compiled native objects/libraries; --external rehashes all original evidence files. No test is rerun.',
                'historical_failures': {
                    'execution-001': 'metadata --no-deps left lock unreconciled; checker refused before build',
                    'execution-002': '7.0 canonicalized to INT; Rust float-tag witness failed before JS controls',
                    'execution-003': 'common target reused baseline for all variants; wrong-store reachability failed; not candidate evidence'},
                'limits': 'CI standalone interpreter controls only. No timing samples, native host/borrowed-loan integration, conformance or shipping gate acceptance. Production dependency unchanged.'}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(rows), 'payloads': len(payloads), 'archive_bytes': len(archive),
                      'manifest_sha256': sha((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
