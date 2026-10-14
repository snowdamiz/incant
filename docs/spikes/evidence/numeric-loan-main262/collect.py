#!/usr/bin/env python3
"""Retain existing CPU and shipping artifacts. Never builds or measures."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
RAW = Path.home() / '.codex/incant-evidence'
REVIEWS = Path.home() / '.codex/incant-reviews'
SOURCES = {'baseline': '051e0de7f26c6f38c470765ae86b8bc415d19fcc', 'candidate': '03bc4b987f3e553249846c4cfce991793d4d2c48'}
FOLDERS = {'baseline': 'core-bulk-row-metadata-003', 'candidate': 'mac-core-numeric-loan-shipping-001', 'pair': 'mac-core-numeric-loan-pair-001', 'cpu001': 'numeric-loan-prepared-globals-001', 'cpu002': 'numeric-loan-prepared-globals-002'}
OWNED = ['crates/incant_runtime/src/columns.rs', 'crates/incant_runtime/src/columns/tests.rs', 'crates/incant_runtime/src/columns/validation_tests.rs', 'crates/incant_runtime/src/world.rs', 'crates/incant_runtime/tests/entity_identity.rs', 'crates/incant_script/src/bindings.rs']
H = lambda b: hashlib.sha256(b).hexdigest()


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def main():
    rows, payloads = [], {}
    def add(name, path, embed):
        size = path.stat().st_size
        digest_ = digest(path)
        rows.append(dict(path=name, original=str(path), bytes=size, sha256=digest_, embedded=embed))
        if embed:
            payloads[digest_] = base64.b64encode(path.read_bytes()).decode()
    for label, folder in FOLDERS.items():
        for path in sorted((RAW/folder).rglob('*')):
            if not path.is_file() or '__pycache__' in path.parts:
                continue
            rel = path.relative_to(RAW/folder).as_posix()
            if rel.startswith('integration-audit-'):
                continue
            embed = not (rel.startswith(('authoring/', 'package/player/', 'package/symbols/', 'binaries/')) or path.suffix == '.pcm')
            if rel in ('authoring/core.json', 'authoring/gameplay.ts', 'authoring/stress.ts'):
                embed = True
            add(label+'/'+rel, path, embed)
    for name, path in [('core-prepared.json', RAW/'mac-core-shipping-002/prepared.json'), ('shipping-inputs.json', RAW/'core-script-stress-001/fast-stress-002/shipping-inputs.json')]:
        add('reference/'+name, path, True)
    for label, source in SOURCES.items():
        raw = subprocess.check_output(['git','show',source+':package.json'],cwd=ROOT)
        digest_ = H(raw)
        rows.append(dict(path='reference/'+label+'-package.json',bytes=len(raw),sha256=digest_,embedded=True,git=source+':package.json'))
        payloads[digest_] = base64.b64encode(raw).decode()
    for name in ['numeric-loan-prepared-globals-source-independent-20261014.json', 'entity-identity-token-check-source-independent-20261014.json', 'numeric-loan-prepared-globals-cpu-independent-20261014.json', 'numeric-loan-format-initial.stdout']:
        add('review/'+name, REVIEWS/name, True)
    current = Path.home()/'.codex/incant-integration-evidence/numeric-loan-main262-001'
    for name in ['composition.json','verify.py']:
        add('current/'+name,current/name,True)
    owned = {}
    for name in OWNED:
        raw = subprocess.check_output(['git','show',SOURCES['candidate']+':'+name],cwd=ROOT)
        assert (ROOT/name).read_bytes() == raw
        owned[name] = H(raw)
    auditor_paths = list(HERE.glob('*.py')) + [HERE.parent/'mac-core-fast10k'/n for n in ('verify.py','metrics.py','build.py')]
    archive = gzip.compress(json.dumps(dict(payloads=payloads),separators=(',',':')).encode(),mtime=0)
    (HERE/'artifacts.json.gz').write_bytes(archive)
    manifest = dict(version=1,sources=SOURCES,owned=owned,records=rows,archive_sha256=H(archive),audit_files={str(p.relative_to(ROOT)):H(p.read_bytes()) for p in sorted(auditor_paths)},scope='Reviewed numeric hierarchy cleanup and opaque entity lifetime checks; actual unchanged-workload CPU-only Mac shipping, all 2ms gates failed; no FPS or full phase claim.')
    (HERE/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps(dict(records=len(rows),payloads=len(payloads),external=sum(not r['embedded'] for r in rows),archive_bytes=len(archive))))


if __name__ == '__main__':
    main()
