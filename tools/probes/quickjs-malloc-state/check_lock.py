#!/usr/bin/env python3
"""Check reconciled diagnostic dependencies against the exact workspace lock."""
import sys
import tomllib
from pathlib import Path
root, actual = (tomllib.loads(Path(p).read_text())['package'] for p in sys.argv[1:])
expected = {(r['name'], r['version'], r.get('source'), r.get('checksum')) for r in root}
for row in actual:
    if row['name'] == 'incant-quickjs-malloc-state-probe':
        assert row['version'] == '0.1.0' and 'source' not in row
    elif row['name'] in ('rquickjs-sys', 'rquickjs-core'):
        assert row['version'] == '0.14.0' and 'source' not in row and 'checksum' not in row
    else:
        assert (row['name'], row['version'], row.get('source'), row.get('checksum')) in expected, row
for name in ['rquickjs-sys', 'rquickjs-core']:
    assert sum(r['name'] == name for r in actual) == 1
print(f'{len(actual)} package identities bound; only diagnostic root and two local pinned copies differ')
