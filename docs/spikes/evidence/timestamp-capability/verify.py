"""Read-only source, raw evidence and compile-only integrity assertions."""
from pathlib import Path
import gzip,hashlib,json,re,subprocess
R=Path(__file__).resolve().parents[4]
M=json.loads((R/'docs/spikes/evidence/timestamp-capability.json').read_text())
def sha(b):return hashlib.sha256(b).hexdigest()
for row in M['artifacts']:
 compressed=(R/row['path']).read_bytes();assert sha(compressed)==row['compressed_sha256']
 raw=gzip.decompress(compressed);assert len(raw)==row['bytes'] and sha(raw)==row['sha256']
 assert Path(row['original']).read_bytes()==raw
freeze=json.loads(Path(M['freeze']).read_text())
for row in freeze['files']:assert sha((R/row['path']).read_bytes())==row['sha256'],row['path']
assert len(freeze['files'])==M['frozen_inputs']==3065
assert sha((R/M['source_path']).read_bytes())==M['source_sha256']
changed=subprocess.check_output(['git','diff','--name-only',M['base_main'],'--','crates','third_party','Cargo.toml','Cargo.lock','.cargo','.github','tools','sdk','schemas'],cwd=R,text=True).splitlines()
assert changed==[M['source_path']],changed
for c in M['checks']:
 assert c['exit']==0 and sha(Path(c['log']).read_bytes())==c['sha256']
counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;',Path(M['checks'][1]['log']).read_text())
assert counts==[('94','0','36')]
b=M['binary'];p=Path(b['path']);assert p.stat().st_size==b['bytes'] and sha(p.read_bytes())==b['sha256']
review=M['source_review'];assert sha((R/review['path']).read_bytes())==review['sha256']
f=M['initial_hosted_failure'];raw=Path(f['raw']).read_bytes();assert sha(raw)==f['sha256'] and b'Apple Paravirtual device' in raw and b'Err` value: Unsupported' in raw
print(json.dumps({'passed':True,'records':len(M['artifacts']),'frozen_inputs':len(freeze['files']),'checks':len(M['checks']),'cpu':94,'gpu_ignored':36,'actual_new_gpu':False}))
