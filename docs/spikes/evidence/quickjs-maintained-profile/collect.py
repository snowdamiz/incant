#!/usr/bin/env python3
"""Retain the maintained native profile qualification without copying large executables."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path.home() / '.codex/incant-evidence/quickjs-maintained-profile-001'


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
    commands = [dict(command, generation=path.parent.name)
                for path in sorted(E.glob('execution-*/checks.json'))
                for command in json.loads(path.read_text())]
    binaries = [dict(binary, generation=path.parent.name)
                for path in sorted(E.glob('execution-*/binaries.json'))
                for binary in json.loads(path.read_text())]
    manifest = {
        'schema': 'incant-quickjs-maintained-profile-v1',
        'source': '1b20ad5a9a772d334cb03a4ee1572627813454fa',
        'source_generations': ['9c4ba9f731f2d1148be23cbad4d69ac99250f00a',
                               '9de22976b7dc1d845919d5792f4967080602af5e',
                               '1b20ad5a9a772d334cb03a4ee1572627813454fa'],
        'base': '62a3d09ff7d8ae7282c57bc49620950650337b7c',
        'scope': 'Additive maintained QuickJS counters under the existing runtime guard; opt-in profiling v3 with explicit sampled logical counters and optional detailed estimated used-size.',
        'archive_sha256': sha(archive), 'records': len(rows), 'payloads': len(payloads),
        'commands': len(commands), 'success_commands': sum(c.get('exit_code') == 0 for c in commands),
        'retained_failed_commands': [{'generation': c['generation'], 'name': c['name'],
                                     'exit_code': c.get('exit_code')} for c in commands if c.get('exit_code') != 0],
        'copied_executables': len(binaries),
        'external_scope': 'Immediate executable copies and compiled native objects/archives remain external and hash-bound. Embedded sources/logs/receipts are deduplicated losslessly. --external rehashes retained originals; --git-source binds reviewed source copies to immutable producer Git objects.',
        'limitations': ['Mac Apple arm64 CI correctness only; no shipping timing or performance/gate acceptance.',
                       'All20 bundled ABI declarations source-audited; only available native/browser checks executed. No mobile/device claim.',
                       'Per-VM tick-end samples are logical allocator counters, not continuous peaks, semantic live heap, reserved arena bytes or RSS.',
                       'Prepared core tests retain a separate offline lock and test-only manifest overlays. Root lock never changes during qualification.',
                       'Historical compile/invocation/dev-cache/test-lint failures retained. No failed command is relabeled passing.']}
    (HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'records': len(rows), 'payloads': len(payloads), 'archive_bytes': len(archive),
                      'manifest_sha256': sha((HERE / 'manifest.json').read_bytes())}))


if __name__ == '__main__':
    main()
