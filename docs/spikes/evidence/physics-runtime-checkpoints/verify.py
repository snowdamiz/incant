#!/usr/bin/env python3
"""Read-only verification of retained sources and local command records."""
from pathlib import Path
import gzip,hashlib,json,re
root=Path(__file__).resolve().parent
manifest=json.loads((root/'manifest.json').read_text())
blob=(root/'records.json.gz').read_bytes()
assert hashlib.sha256(blob).hexdigest()==manifest['records_sha256']
records=json.loads(gzip.decompress(blob))['records']
assert len(records)==manifest['record_count']
assert len({r['name'] for r in records})==len(records)
assert sum(r['kind']=='source' for r in records)==manifest['source_count']
by_name={r['name']:r for r in records}
for r in records:
 assert hashlib.sha256(r['text'].encode()).hexdigest()==r['sha256'],r['name']
for check in manifest['checks']:
 name=check['command'];receipt=json.loads(by_name[name+'.json']['text']);log=by_name[name+'.log']
 assert receipt['exit_code']==check['exit_code']==0
 assert receipt['log_sha256']==log['sha256']
 assert sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log['text'])))==check['tests_passed']
print(f"Verified {len(records)} retained records and {len(manifest['checks'])} successful checks; executable payloads and hosted/device gates are outside this packet.")
