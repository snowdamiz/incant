#!/usr/bin/env python3
"""Retain existing Core sample evidence. No export, player or measurement runs."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
RAW = Path.home() / '.codex/incant-evidence'
SOURCES = {'baseline': '031a7173ac865a99964e12538926e02ca64ccd71', 'candidate': '051e0de7f26c6f38c470765ae86b8bc415d19fcc', 'ci': '4e7986aae4e5cfa8ab144860f7052c57baf1b499'}
FOLDERS = {'baseline': 'mac-core-fast10k-cache-001', 'candidate': 'core-bulk-row-metadata-003', 'pair': 'core-bulk-row-metadata-pair-001', 'failed001': 'core-bulk-row-metadata-001', 'failed002': 'core-bulk-row-metadata-002'}
OWNED = ['package.json', 'samples/core/README.md', 'samples/core/gameplay.ts', 'samples/core/stress.ts', 'samples/core/gameplay_test.mjs', 'samples/core/stress_test.mjs']
H = lambda b: hashlib.sha256(b).hexdigest()


def main():
    rows, payloads = [], {}
    def add(name, p, embedded):
        raw = p.read_bytes(); sha = H(raw)
        rows.append(dict(path=name, original=str(p), bytes=len(raw), sha256=sha, embedded=embedded))
        if embedded:
            payloads[sha] = base64.b64encode(raw).decode()
    for label, folder in FOLDERS.items():
        for p in sorted((RAW/folder).rglob('*')):
            if not p.is_file() or '__pycache__' in p.parts:
                continue
            rel = p.relative_to(RAW/folder).as_posix()
            if rel.startswith('integration-audit-'):
                continue
            if label == 'baseline' and rel.startswith('paired-'):
                continue  # Those earlier measurements stay in their original packet.
            embed = not (rel.startswith(('authoring/', 'package/player/', 'package/symbols/')) or p.suffix == '.pcm')
            if rel in ('authoring/core.json', 'authoring/gameplay.ts', 'authoring/stress.ts') or rel.startswith('authoring/.incant-source-transactions/'):
                embed = True
            add(label+'/'+rel, p, embed)
    for name, p in [('reference/core-prepared.json', RAW/'mac-core-shipping-002/prepared.json'), ('reference/shipping-inputs.json', RAW/'core-script-stress-001/fast-stress-002/shipping-inputs.json'), ('reference/stress-prepared.json', RAW/'core-script-stress-001/fast-stress-002/receipt.json'), ('reference/authoring-binary', Path.home()/'.codex/incant-integration-evidence/core0069-main212-001/frozen-binaries/incant_headless')]:
        add(name, p, name != 'reference/authoring-binary')
    current = Path.home()/'.codex/incant-integration-evidence/core-bulk-main261-001'
    for p in sorted(current.glob('check*')):
        if p.is_file():
            add('current/'+p.name, p, True)
    owned = {}
    for name in OWNED:
        raw = subprocess.check_output(['git', 'show', SOURCES['ci']+':'+name], cwd=ROOT)
        assert (ROOT/name).read_bytes() == raw
        owned[name] = H(raw)
    auditor_paths = list(HERE.glob('*.py')) + [HERE.parent/'mac-core-fast10k'/n for n in ('verify.py', 'metrics.py', 'build.py')]
    archive = gzip.compress(json.dumps(dict(payloads=payloads), separators=(',', ':')).encode(), mtime=0)
    (HERE/'artifacts.json.gz').write_bytes(archive)
    manifest = dict(version=1, sources=SOURCES, owned=owned, records=rows, archive_sha256=H(archive), audit_files={str(p.relative_to(ROOT)):H(p.read_bytes()) for p in sorted(auditor_paths)}, scope='Changed-script Core sample comparison, actual CPU-only Mac shipping; all 2ms gates failed; no engine-only, FPS or phase claim.')
    (HERE/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps(dict(records=len(rows), payloads=len(payloads), external=sum(not r['embedded'] for r in rows), archive_bytes=len(archive))))


if __name__ == '__main__':
    main()
