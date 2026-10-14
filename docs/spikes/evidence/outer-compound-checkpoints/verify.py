#!/usr/bin/env python3
"""Read-only verification of retained scoped outer-checkpoint evidence."""
from pathlib import Path
import base64,gzip,hashlib,json
E=Path(__file__).resolve().parent
R=E.parents[3]
m=json.loads((E/'manifest.json').read_text())
a=(E/'records.json.gz').read_bytes()
assert hashlib.sha256(a).hexdigest()==m['archive_sha256']
rows=json.loads(gzip.decompress(a));assert len(rows)==m['records']
for row in rows:
 b=base64.b64decode(row['data']);assert len(b)==row['bytes'];assert hashlib.sha256(b).hexdigest()==row['sha256']
for row in m['sources']:
 b=(R/row['path']).read_bytes();assert len(b)==row['bytes'];assert hashlib.sha256(b).hexdigest()==row['sha256'],row['path']
assert all(c['exit_code']==0 for c in m['checks'])
print(json.dumps({'records':len(rows),'sources':len(m['sources']),'successful_commands':len(m['checks']),'unique_focused_tests':m['unique_focused_tests'],'continuation':'pending lower compound union'}))
