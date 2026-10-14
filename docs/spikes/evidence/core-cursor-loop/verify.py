#!/usr/bin/env python3
"""Replay cursor source/log receipts without executing the workload."""
import argparse, base64, gzip, hashlib, json
from pathlib import Path

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--originals', action='store_true')
    parser.add_argument('--tools', action='store_true')
    args=parser.parse_args()
    d=Path(__file__).resolve().parent; root=d.parents[3]
    sha=lambda b: hashlib.sha256(b).hexdigest()
    manifest=json.loads((d/'manifest.json').read_text())
    archive=(d/'records.json.gz').read_bytes()
    assert sha(archive)==manifest['archive_sha256']
    payload=json.loads(gzip.decompress(archive)); records={}
    for row in manifest['records']:
        data=base64.b64decode(payload[row['sha256']], validate=True)
        assert len(data)==row['bytes'] and sha(data)==row['sha256']
        if args.originals: assert Path(row['original']).read_bytes()==data
        records[row['name']]=data
    for row in manifest['sources']:
        assert sha((root/row['path']).read_bytes())==row['sha256']
    result=json.loads(records['checks-001/result.json'])
    assert result['passed'] and result['source_unchanged']
    assert result['source_commit']==manifest['source_commit']
    assert [r['name'] for r in result['commands']]==['node-version','strict-typescript','stress','python-helpers','diff']
    for row in result['commands']:
        assert row['exit_code']==0
        for stream in ('stdout','stderr'):
            assert sha(records['checks-001/'+row['name']+'.'+stream])==row[stream]['sha256']
    assert b'all 9999 rows/four phases' in records['checks-001/stress.stdout']
    assert b'three omitted-write mutations' in records['checks-001/stress.stdout']
    assert b'OK' in records['checks-001/python-helpers.stderr']
    if args.tools:
        for row in json.loads(records['checks-001/inputs.json']):
            p=Path(row['path']); p=p if p.is_absolute() else root/p
            assert sha(p.read_bytes())==row['sha256'], p
    print(json.dumps(dict(passed=True,records=len(records),commands=5,scope='Node/TypeScript controls; not a native shipping performance gate')))
if __name__=='__main__': main()
