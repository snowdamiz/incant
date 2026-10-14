from pathlib import Path
import json,gzip,base64,hashlib
D=Path(__file__).resolve().parent
h=lambda b:hashlib.sha256(b).hexdigest()
m=json.loads((D/'manifest.json').read_text());v=json.loads((D/'verification.json').read_text())
assert h((D/'verification.json').read_bytes())==m['verification_sha256']
a=m['archive'];blob=(D/a['path']).read_bytes();assert h(blob)==a['sha256'];raw=gzip.decompress(blob);assert h(raw)==a['raw_sha256'];z=json.loads(raw)
assert len(z['records'])==a['records']==10 and len(z['payloads'])==a['payloads']
files={}
for r in z['records']:
 b=base64.b64decode(z['payloads'][r['sha256']]);assert len(b)==r['bytes'] and h(b)==r['sha256'];files[r['path']]=b
assert z['records']==v['records']
run=json.loads(files['terminal-run.json']);assert run['headSha']==m['head'] and run['status']=='completed' and run['conclusion']=='success'
assert files['artifacts/windows-filter-diagnostic/source.txt'].decode().strip()==m['head']
for name in ['exit.txt','strict-exit.txt']:assert files['artifacts/windows-filter-diagnostic/'+name].decode().strip()=='0'
s=files['artifacts/windows-filter-diagnostic/strict.log'].decode();assert '1 passed; 0 failed; 0 ignored' in s and v['adapter'] in s and 'backend: Dx12' in v['adapter']
for marker in v['strict_markers']:assert marker in s and 'checked_depth=285 checked_r8=570 observed_r8=570 contract_asserted=true mode=Production' in marker
assert len(v['strict_markers'])==2 and v['total_submissions']==8 and v['total_render_passes']==24 and v['images']==0
o=files['artifacts/windows-filter-diagnostic/run.log'].decode();assert 'test result: ok. 1 passed; 0 failed; 0 ignored;' in o
assert o.count('SSAO_FILTER_RASTER projection=')==12
assert 'SSAO_FILTER_DIAGNOSTIC observations_only=true projections=2 variants=3 submits=6 maps=6 render_passes=18 no_images=true' in o
print(json.dumps({'verified':True,'head':m['head'],'run':m['run'],'strict_projections':2,'records':10}))
