#!/usr/bin/env python3
"""Verify retained game checkpoint sources/results; execute no runtime code."""
import argparse
import base64
import gzip
import hashlib
import json
import re
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--local-binaries', action='store_true')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[3]
    meta = json.loads((here / 'manifest.json').read_bytes())
    packed = (here / meta['archive']['path']).read_bytes()
    assert sha(packed) == meta['archive']['sha256']
    rows = json.loads(gzip.decompress(packed))
    assert len(rows) == meta['archive']['records']
    records = {}
    for row in rows:
        name = row['path']
        assert not Path(name).is_absolute() and '..' not in Path(name).parts
        assert name not in records
        raw = base64.b64decode(row['data_base64'], validate=True)
        assert len(raw) == row['bytes'] and sha(raw) == row['sha256']
        records[name] = raw
    assert sha(records['source-003/freeze.json']) == meta['freeze_sha256']
    for row in meta['sources']:
        assert sha((root / row['path']).read_bytes()) == row['sha256']
        assert sha(records['source-003/' + row['path']]) == row['sha256']
    for item in meta['checks']:
        name = item['name']
        receipt = records[name + '.json']
        assert sha(receipt) == item['receipt_sha256']
        r = json.loads(receipt)
        assert r['exit_code'] == item['exit_code']
        assert r['reaped'] and not r['changed_inputs']
        log = records[name + '.log']
        assert sha(log) == r['log_sha256']
        assert sum(map(int, re.findall(rb'test result: ok\. (\d+) passed;', log))) == item['passed']
        executed = re.findall(r'Running .* \(([^)]+)\)', log.decode())
        assert executed == [b['source'] for b in r['binaries']]
        assert len(executed) == item['executables']
        if args.local_binaries:
            for binary in r['binaries']:
                p = Path(binary['copy'])
                assert sha(p.read_bytes()) == binary['sha256']
                assert p.stat().st_mode & 0o111
    print(f'Verified {len(rows)} records and {len(meta["sources"])} game source identities. '
          'Historical results only; no new build, device or phase-gate claim.')


if __name__ == '__main__':
    main()
