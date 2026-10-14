#!/usr/bin/env python3
"""Package completed evidence only; never author, compile or execute a player."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = Path.home() / '.codex/incant-evidence/wasm-ts-cost-001'
SOURCE = '97834ab3282e0afb8b80820caf19394c929cf821'
REVIEWS = ('wasm-ts-cost-source002-root-20261014.json',
           'wasm-ts-cost-source005-root-20261014.json',
           'wasm-ts-cost-timing-protocol-root-20261014.json')


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def file_digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        while block := stream.read(1024 * 1024):
            value.update(block)
    return value.hexdigest()


def main():
    records, payloads = [], {}

    def add(name, path, embedded):
        assert path.is_file() and not path.is_symlink(), path
        sha = file_digest(path)
        records.append({'path': name, 'original': str(path), 'bytes': path.stat().st_size,
                        'mode': oct(path.stat().st_mode & 0o777), 'sha256': sha, 'embedded': embedded})
        if embedded:
            payloads[sha] = base64.b64encode(path.read_bytes()).decode()

    for path in sorted(RAW.rglob('*')):
        if not path.is_file() or '__pycache__' in path.parts:
            continue
        name = str(path.relative_to(RAW))
        # Preserve raw RPC requests/responses and all reports; journals/cache are
        # redundant retained authoring history, while large executable/PCM data
        # is explicitly hash-bound external evidence, not silently omitted.
        external = (path.suffix in ('.pcm', '.wasm', '.rlib')
                    or path.name in ('incant_headless', 'incant_game_player')
                    or '/symbols/' in name or '.incant/cache/' in name
                    or path.name.endswith('journal.jsonl'))
        add(name, path, not external)
    for name in REVIEWS:
        add('reviews/' + name, Path.home() / '.codex/incant-reviews' / name, True)
    freeze = json.loads((RAW / 'export-001/freeze.json').read_text())
    for name, value in freeze['wabt_files'].items():
        path = Path(freeze['wabt']) / name
        assert file_digest(path) == value['sha256']
        add('trusted-wabt/' + name, path, False)
    source = json.loads((RAW / 'source-005/freeze.json').read_text())
    node = Path(source['node']['path'])
    assert file_digest(node) == source['node']['sha256']
    add('trusted-node', node, False)
    raw = gzip.compress(json.dumps({'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
    (HERE / 'artifacts.json.gz').write_bytes(raw)
    manifest = {'format': 'incant-wasm-typescript-cost-evidence', 'version': 1,
                'executed_source': SOURCE, 'source_freeze': 'source-005/freeze.json',
                'records': records, 'archive_sha256': digest(raw),
                'audit_files': {p.name: file_digest(p) for p in sorted(HERE.glob('*.py'))},
                'scope': 'Two fresh shipping exports, eight unprofiled all-row controls, six Mac per-frame measurements for1024 entities.',
                'external_scope': 'Native tools/players/symbols, guest artifacts, redundant journals/cache and PCM stay at hash-bound original paths. Default verification proves retained report/build mathematics. --external rehashes every original. --git-source checks both source freezes against immutable Git.',
                'limits': ['No startup, Core Sample,10000-entity, GPU/FPS, mobile or phase-gate result.',
                           'Backend phases differ; interval quotient is descriptive, not the TypeScript2ms/5percent policy.',
                           'Original inputs remain evidence; only disposable export-input paths/private build trees were removed.']}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(records), 'payloads': len(payloads),
                      'external': sum(not r['embedded'] for r in records), 'archive_bytes': len(raw),
                      'manifest_sha256': file_digest(HERE / 'manifest.json')}))


if __name__ == '__main__':
    main()
