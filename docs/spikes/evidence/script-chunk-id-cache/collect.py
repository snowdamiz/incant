#!/usr/bin/env python3
"""Collect only retained cache source/tests; never runs a compiler or game."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
RAW = Path.home() / '.codex/incant-evidence/script-chunk-id-cache-001'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    records, payloads = [], {}
    for path in sorted(RAW.rglob('*')):
        if not path.is_file() or '__pycache__' in path.parts:
            continue
        data = path.read_bytes()
        digest = sha(data)
        rel = str(path.relative_to(RAW))
        embedded = not rel.startswith('binaries/')
        records.append({'path': rel, 'bytes': len(data), 'sha256': digest, 'embedded': embedded,
                        'mode': oct(path.stat().st_mode & 0o777)})
        if embedded:
            payloads[digest] = base64.b64encode(data).decode()
    source = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    frozen = json.loads((RAW / 'source-001/freeze.json').read_text())
    rows = [{'path': p, 'sha256': h} for p, h in frozen['source_paths'].items()]
    assert all(sha((ROOT / row['path']).read_bytes()) == row['sha256'] for row in rows)
    archive = gzip.compress(json.dumps({'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
    (HERE / 'artifacts.json.gz').write_bytes(archive)
    manifest = {'format': 'incant-script-chunk-id-cache-evidence', 'version': 1, 'source_commit': source,
                'base_commit': frozen['source_base'], 'raw_root': str(RAW), 'records': records, 'archive_sha256': sha(archive),
                'sources': rows, 'verifier_sha256': sha((HERE / 'verify.py').read_bytes()),
                'passing_commands': [mode + '-' + kind for mode in ('deterministic', 'fast') for kind in ('lib', 'project', 'clippy')],
                'setup_note': 'Initial unqualified rustfmt lookup failed; pinned 1.99.0 rustfmt succeeded before source freeze.',
                'limits': ['Explicit native ci/compiler scope in both physics modes; no full-workspace claim.',
                           'No shipping measurement, GPU, changed workload or altered runtime budgets.',
                           'Immediate copied executables are externally hash-bound; portable mode verifies their recorded identities only.']}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(records), 'payloads': len(payloads), 'archive_bytes': len(archive),
                      'manifest_sha256': sha((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
