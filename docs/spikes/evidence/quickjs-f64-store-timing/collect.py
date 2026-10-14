#!/usr/bin/env python3
"""Package one fixed standalone comparison, preserving every raw sample."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path.home() / '.codex/incant-evidence/quickjs-f64-store-timing-001'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    plan = json.loads((E / 'plan.json').read_text())
    files = {p.relative_to(E).as_posix(): p for p in sorted(E.rglob('*')) if p.is_file()}
    files.update({f'functional-{key}.stdout': Path(v['functional_stdout']['path'])
                  for key, v in plan['variants'].items()})
    rows, payloads = [], {}
    for name, path in files.items():
        data = path.read_bytes()
        digest = sha(data)
        rows.append({'name': name, 'original': str(path), 'sha256': digest, 'bytes': len(data)})
        payloads[digest] = base64.b64encode(data).decode('ascii')
    packed = gzip.compress(json.dumps({'records': rows, 'payloads': payloads}, sort_keys=True,
                                     separators=(',', ':')).encode(), mtime=0)
    (HERE / 'records.json.gz').write_bytes(packed)
    manifest = {'schema': 'incant-quickjs-f64-timing-v1', 'source': plan['source'],
                'functional_packet': plan['packet'], 'plan_sha256': sha((E / 'plan.json').read_bytes()),
                'runner_sha256': sha((E / 'measure.py').read_bytes()), 'records': len(rows),
                'payloads': len(payloads), 'archive_sha256': sha(packed),
                'scope': plan['scope'], 'outcome': 'Mixed/slower diagnostic; no adoption or shipping evidence.'}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(rows), 'payloads': len(payloads), 'archive_bytes': len(packed),
                      'manifest_sha256': sha((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
