#!/usr/bin/env python3
"""Package completed evidence only; never compile, export or execute a player."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
RAW = Path.home() / '.codex/incant-evidence'
SOURCES = {'original': 'bff83af61807435926b10d77b5a04718a464aae5',
           'baseline': '3c81bbe6763f78c10c7d13d08975f2ffeff7caa3',
           'candidate': '031a7173ac865a99964e12538926e02ca64ccd71'}
FOLDERS = {'original': 'mac-core-fast10k-shipping-001', 'baseline': 'mac-core-fast10k-static-baseline-001',
           'candidate': 'mac-core-fast10k-cache-001', 'pair': 'mac-core-fast10k-pair-001'}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    records, payloads = [], {}
    def add(label, path, embedded):
        raw = path.read_bytes()
        sha = digest(raw)
        records.append({'path': label, 'original': str(path), 'bytes': len(raw), 'sha256': sha, 'embedded': embedded})
        if embedded:
            payloads[sha] = base64.b64encode(raw).decode()
    for label, folder in FOLDERS.items():
        for path in sorted((RAW / folder).rglob('*')):
            if not path.is_file() or '__pycache__' in path.parts:
                continue
            rel = str(path.relative_to(RAW / folder))
            embedded = not (rel.startswith(('authoring/', 'package/player/', 'package/symbols/')) or path.suffix == '.pcm')
            if rel in ('authoring/core.json', 'authoring/gameplay.ts', 'authoring/stress.ts', 'package/player/data/package.json'):
                embedded = True
            add(label + '/' + rel, path, embedded)
    add('reference/core-prepared.json', RAW / 'mac-core-shipping-002/prepared.json', True)
    add('reference/stress-prepared.json', RAW / 'core-script-stress-001/fast-stress-002/receipt.json', True)
    add('reference/shipping-inputs.json', RAW / 'core-script-stress-001/fast-stress-002/shipping-inputs.json', True)
    for name in ('mac-core-fast10k-pair-protocol-independent-20261014.json',
                 'script-caches-main259-final-root-20261014.json', 'script-bindings-static-final-root-20261014.json'):
        add('reviews/' + name, Path.home() / '.codex/incant-reviews' / name, True)
    paths = ['tools/mac_performance.py', 'tools/export_native.py', 'tools/native_profiling.py', 'tools/native_process.py',
             'samples/core/binding_oracle.py', 'samples/core/stress.py', 'samples/core/stress_verify.py', 'Cargo.toml', 'Cargo.lock', '.cargo/config.toml']
    paths += subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', SOURCES['candidate'], 'crates/incant_profile'], cwd=ROOT, text=True).splitlines()
    freeze = json.loads((RAW / FOLDERS['candidate'] / 'freeze.json').read_text())
    for path in paths:
        raw = subprocess.check_output(['git', 'show', SOURCES['candidate'] + ':' + path], cwd=ROOT)
        sha = digest(raw)
        assert sha == freeze['repository_files'][path], path
        records.append({'path': 'source/' + path, 'git_commit': SOURCES['candidate'], 'bytes': len(raw), 'sha256': sha, 'embedded': True})
        payloads[sha] = base64.b64encode(raw).decode()
    archive = gzip.compress(json.dumps({'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
    (HERE / 'artifacts.json.gz').write_bytes(archive)
    manifest = {'format': 'incant-mac-fast10k-shipping-evidence', 'version': 1, 'sources': SOURCES,
                'records': records, 'archive_sha256': digest(archive),
                'audit_files': {p.name: digest(p.read_bytes()) for p in sorted(HERE.glob('*.py'))},
                'scope': 'Three matched pairs of CPU-only fast shipping Core with 10000 bound script attachments. Every candidate fails 2 ms script p95; no display/GPU/full-phase claim.',
                'external_scope': 'Native executables, symbols, binary authored/cooked assets and PCM are hash/size-bound external originals. Text/logs/freezes/generated old Rust are retained losslessly. --external checks all originals; --git-source checks both complete successful source inventories against immutable Git.'}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(records), 'payloads': len(payloads), 'external': sum(not r['embedded'] for r in records),
                      'archive_bytes': len(archive), 'manifest_sha256': digest((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
