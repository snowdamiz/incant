#!/usr/bin/env python3
"""Refuse unpinned diagnostic dependencies after offline lock reconciliation."""
import sys
import tomllib
from pathlib import Path

root, actual = (tomllib.loads(Path(p).read_text())['package'] for p in sys.argv[1:])
expected = {(p['name'], p['version'], p.get('source'), p.get('checksum')) for p in root}
for row in actual:
    if row['name'] == 'incant-quickjs-f64-store-probe':
        assert row['version'] == '0.1.0' and 'source' not in row
    elif row['name'] == 'rquickjs-sys':
        assert row['version'] == '0.14.0' and 'source' not in row and 'checksum' not in row
    else:
        assert (row['name'], row['version'], row.get('source'), row.get('checksum')) in expected, row
assert sum(r['name'] == 'rquickjs-sys' for r in actual) == 1
print(f'{len(actual)} package identities bound to root lock plus diagnostic root/local sys copy')
