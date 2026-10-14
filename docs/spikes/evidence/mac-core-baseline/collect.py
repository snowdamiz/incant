#!/usr/bin/env python3
"""Package existing completed runs only. Never starts a compiler or player."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
SOURCE = 'bff83af61807435926b10d77b5a04718a464aae5'
RAW = Path.home() / '.codex/incant-evidence'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    records, payloads = [], {}
    def add(label, path, embedded):
        data = path.read_bytes()
        sha = digest(data)
        records.append({'path': label, 'original': str(path), 'bytes': len(data), 'sha256': sha, 'embedded': embedded})
        if embedded:
            payloads[sha] = base64.b64encode(data).decode()
    for generation in ('001', '002'):
        folder = RAW / ('mac-core-shipping-' + generation)
        for path in sorted(folder.rglob('*')):
            if not path.is_file():
                continue
            rel = str(path.relative_to(folder))
            embedded = not (rel.startswith(('authoring/', 'package/player/', 'package/symbols/')) or path.suffix == '.pcm')
            if rel in ('authoring/core.json', 'authoring/gameplay.ts', 'package/player/data/package.json'):
                embedded = True
            add(generation + '/' + rel, path, embedded)
    freeze = json.loads((RAW / 'mac-core-shipping-002/freeze.json').read_text())
    for name in ('win.json', 'win-input.json', 'win-observe.json', 'win.pcm'):
        add('historical/' + name, Path(freeze['original']) / name, name != 'win.pcm')
    for name in ('native-profile-distributions-root-20261013.json', 'profile-distributions-main257-final-root-20261014.json'):
        add('reviews/' + name, Path.home() / '.codex/incant-reviews' / name, True)
    setup = json.loads((RAW / 'mac-core-shipping-002/setup.json').read_text())
    node = Path(setup['node']['path'])
    assert digest(node.read_bytes()) == setup['node']['sha256']
    add('tools/node', node, False)
    paths = ['tools/mac_performance.py', 'tools/export_native.py', 'tools/native_profiling.py', 'tools/native_process.py',
             'Cargo.lock', 'Cargo.toml', '.cargo/config.toml']
    paths += [p for p in freeze['repository_files'] if p.startswith('crates/incant_profile/')]
    for path in paths:
        data = subprocess.check_output(['git', 'show', SOURCE + ':' + path], cwd=ROOT)
        sha = digest(data)
        assert sha == freeze['repository_files'][path]
        records.append({'path': 'source/' + path, 'git_commit': SOURCE, 'bytes': len(data), 'sha256': sha, 'embedded': True})
        payloads[sha] = base64.b64encode(data).decode()
    archive = gzip.compress(json.dumps({'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
    (HERE / 'artifacts.json.gz').write_bytes(archive)
    manifest = {'format': 'incant-mac-core-baseline-evidence', 'version': 1, 'executed_source': SOURCE,
                'records': records, 'archive_sha256': digest(archive),
                'audit_files': {p.name: digest(p.read_bytes()) for p in sorted(HERE.glob('*.py'))},
                'scope': 'Three shipping Mac deterministic original Core runs; repeatability baseline, not fast10k or display/GPU qualification.',
                'external_scope': 'Player, symbols, reused authoring/cooked assets and PCM remain at original hash-bound paths. --external checks every original file; --git-source checks all 4724 frozen source rows against immutable Git.'}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(records), 'payloads': len(payloads), 'external': sum(not r['embedded'] for r in records),
                      'archive_bytes': len(archive), 'manifest_sha256': digest((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
