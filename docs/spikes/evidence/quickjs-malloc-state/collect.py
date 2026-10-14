#!/usr/bin/env python3
"""Retain the getter-only native diagnostic without copying large executables."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path.home() / '.codex/incant-evidence/quickjs-malloc-state-001'


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
    final = json.loads((E / 'execution-001/result.json').read_text())
    manifest = {'schema': 'incant-quickjs-malloc-state-v1', 'source': final['source'],
                'scope': final['scope'], 'execution': 'execution-001',
                'archive_sha256': sha(archive), 'records': len(rows), 'payloads': len(payloads),
                'observations': 32, 'commands': 15, 'success_commands': 14,
                'expected_negative': 'wrong-count native getter exits 101 at first count assertion',
                'source_review_sha256': '0315a7820863e3c454199d4c84e4245b245f9fafdafe2258c290b9aef6ca11ca',
                'external_scope': 'Executables and compiled native objects/libraries remain external and hash-bound. --external rehashes all originals and tool executables without executing them.',
                'limits': 'Native Apple arm64 CI, default libc/stdlib only. No timing, custom allocator, mutable ECS loan, continuous peak, production integration or gate acceptance. Original preflight PATH/read mistakes remain source-freeze notes; no failed native generation beyond the deliberate negative.'}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(rows), 'payloads': len(payloads), 'archive_bytes': len(archive),
                      'manifest_sha256': sha((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
