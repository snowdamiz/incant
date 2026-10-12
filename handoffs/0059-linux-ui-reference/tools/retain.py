#!/usr/bin/env python3
"""Copy 0059 run logs into the packet as committed text (0059).

Usage: python3 -I -B retain.py LABEL...

The repository ignores *.log, and libtest output carries trailing blanks that
`git diff --check` refuses, so each ~/.codex/incant-evidence/0059/logs/LABEL.log
becomes evidence/checks/LABEL.txt with trailing whitespace and final blank lines
removed. evidence/checks/manifest.json records both hashes and the ledger entry.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = Path.home() / '.codex/incant-evidence/0059'
CHECKS = ROOT / 'evidence/checks'


def main(labels):
    CHECKS.mkdir(parents=True, exist_ok=True)
    manifest_path = CHECKS / 'manifest.json'
    manifest = json.loads(manifest_path.read_text(encoding='utf-8')) if manifest_path.exists() else {}
    ledger = {}
    for line in (EVIDENCE / 'commands.jsonl').read_text(encoding='utf-8').splitlines():
        entry = json.loads(line)
        ledger[entry['label']] = entry
    for label in labels:
        raw = (EVIDENCE / 'logs' / f'{label}.log').read_bytes()
        text = '\n'.join(line.rstrip() for line in raw.decode('utf-8').splitlines()).rstrip('\n')
        text += '\n' if text else ''
        out = CHECKS / f'{label}.txt'
        out.write_text(text, encoding='utf-8')
        entry = ledger.get(label, {})
        manifest[label] = {
            'raw_log': str(EVIDENCE / 'logs' / f'{label}.log'),
            'raw_sha256': hashlib.sha256(raw).hexdigest(),
            'committed': f'evidence/checks/{label}.txt',
            'committed_sha256': hashlib.sha256(text.encode('utf-8')).hexdigest(),
            'argv': entry.get('argv'),
            'exit': entry.get('exit'),
            'start': entry.get('start'),
            'end': entry.get('end'),
        }
        print(f'{label}: exit {entry.get("exit")} -> {out.name}')
    manifest_path.write_text(json.dumps(manifest, indent=1, sort_keys=True) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main(sys.argv[1:])
