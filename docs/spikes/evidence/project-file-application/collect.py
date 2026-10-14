#!/usr/bin/env python3
"""Replay application project-file evidence; this never builds or runs engine code."""
import argparse,base64,gzip,hashlib,json,pathlib,re,subprocess
p=argparse.ArgumentParser();p.add_argument('--root',type=pathlib.Path,required=True);p.add_argument('--verify-only',action='store_true');p.add_argument('--originals',action='store_true');p.add_argument('--local-binaries',action='store_true');a=p.parse_args()
R=a.root.resolve();E=pathlib.Path('/Users/andreyyurlov/.codex/incant-evidence/project-file-application-001');D=R/'docs/spikes/evidence/project-file-application';M=D/'manifest.json';A=D/'records.json.gz'
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(v):return (json.dumps(v,indent=2,sort_keys=True)+'\n').encode()
attempts={'all-001':101,'all-002':101,'all-003':0,'lint-003':0,'headless-004':0,'editor-005':0,'boundary-lint-005':0}
node={'bridge-generation-001':0,'schemas-generation-001':0,'sdk-generation-001':0,'bundle-001':2,'bundle-002':0,'bridge-tests-001':0,'sdk-check-001':0,'bridge-check-001':0,'sdk-types-001':0}
if not a.verify_only:
 records=[];payloads={};binaries=[]
 for f in sorted(E.rglob('*')):
  if not f.is_file() or 'binaries' in f.parts or f.name=='collect-draft.py':continue
  b=f.read_bytes();h=sha(b);records.append({'path':str(f.relative_to(E)),'sha256':h,'bytes':len(b)});payloads[h]=base64.b64encode(b).decode()
 for name in attempts:binaries.extend(json.loads((E/name/'receipt.json').read_bytes())['binaries'])
 paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],cwd=R,text=True).splitlines()
 inputs=[{'path':x,'sha256':sha((R/x).read_bytes())} for x in paths if (R/x).is_file() and (x.endswith(('.rs','.toml','.lock','.wgsl')) or x in ['tools/cargo','tools/native_process.py'])]
 final=json.loads((E/'source-005/freeze.json').read_bytes())
 known={x['path'] for x in inputs}
 inputs += [{'path':x['path'],'sha256':x['sha256']} for x in final['sources'] if x['path'] not in known and not x['path'].startswith('docs/')]
 A.write_bytes(gzip.compress(dump({'records':records,'payloads':payloads}),mtime=0))
 m={'version':1,'base':'b2475341baca4144f66cb5805a6c5b66baf27176','collector_sha256':sha(pathlib.Path(__file__).read_bytes()),'archive_sha256':sha(A.read_bytes()),'records':len(records),'payloads':len(payloads),'final_inputs':inputs,'source_freeze':'source-005/freeze.json','rust_attempts':attempts,'node_attempts':node,'binaries':binaries,'scope':'Mac CPU application binding: shared host-owned pin review/apply and v4 checkpoint save, exact causal/history retention and crash recovery. Legacy project.save omitted-revision adapter uses same capability. No new UI controls, older-format conversion, GPU, performance or phase-gate claim.'}
 M.write_bytes(dump(m))
m=json.loads(M.read_bytes());assert sha(pathlib.Path(__file__).read_bytes())==m['collector_sha256'];assert sha(A.read_bytes())==m['archive_sha256']
b=json.loads(gzip.decompress(A.read_bytes()));assert len(b['records'])==m['records'] and len(b['payloads'])==m['payloads'];payloads={h:base64.b64decode(v,validate=True) for h,v in b['payloads'].items()};assert all(sha(v)==h for h,v in payloads.items());rows={x['path']:x for x in b['records']};assert len(rows)==m['records']
def data(name):return payloads[rows[name]['sha256']]
def load(name):return json.loads(data(name))
for name,row in rows.items():
 assert len(data(name))==row['bytes']
 if a.originals:assert (E/name).read_bytes()==data(name),name
for row in m['final_inputs']:assert sha((R/row['path']).read_bytes())==row['sha256'],row['path']
for generation in range(1,6):
 name=f'source-{generation:03d}';f=load(name+'/freeze.json')
 assert rows[name+'/tracked.patch']['sha256']==f['tracked_patch_sha256']
 for row in f['sources']:
  assert rows[name+'/inputs/'+row['path']]['sha256']==row['sha256']
  if row['baseline_sha256'] is not None:assert rows[name+'/baseline/'+row['path']]['sha256']==row['baseline_sha256']
for name,code in attempts.items():
 r=load(name+'/receipt.json');assert r['exit_code']==code and r['source_unchanged'] and r['owned_process_group_cleaned_and_reaped']
 assert rows[name+'/output.log']['sha256']==r['log_sha256'];assert rows[name+'/freeze.json']['sha256']==r['freeze_sha256']
 f=load(name+'/freeze.json');assert rows[f['source_freeze'].removeprefix(str(E)+'/')]['sha256']==r['source_freeze_sha256']
for name,count in [('all-003',107),('headless-004',39),('editor-005',4)]:
 summaries=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',data(name+'/output.log').decode());assert [sum(int(x[i]) for x in summaries)for i in range(3)]==[count,0,0],name
assert b'28 passed; 1 failed' in data('all-002/output.log')
assert b'create_only_backup_failure_never_advances_project_history_or_journal' in data('all-002/output.log')
for name,code in node.items():
 r=load(name+'/receipt.json');assert r['exit_code']==code and r['reaped'];assert rows[name+'/output.log']['sha256']==r['log_sha256'];assert rows[name+'/inputs.json']['sha256']==r['inputs_sha256']
assert b'189 passed' in data('bridge-tests-001/output.log')
for row in load('source-005/editor-assets.json'):assert rows['source-005/built-inputs/'+row['path']]['sha256']==row['sha256']
for x in load('lean-001/checks.json'):
 assert x['exit_code']==0 and x['reaped'];assert rows['lean-001/'+x['name']+'.log']['sha256']==x['log_sha256']
assert data('final-report.md') == (R/'docs/spikes/project-file-application.md').read_bytes()
identity=load('schema-identity.json');assert len(identity['unchanged_existing_schemas'])==74 and len(identity['new_schemas'])==4
for row in identity['unchanged_existing_schemas']+identity['new_schemas']:assert sha((R/row['path']).read_bytes())==row['sha256']
assert load('release.json')['all_owned_supervisors_terminal_and_reaped']
for row in m['binaries']:
 if a.local_binaries:
  f=pathlib.Path(row['path']);assert sha(f.read_bytes())==row['sha256'] and f.stat().st_size==row['bytes']
print(json.dumps({'verified':True,'records':m['records'],'payloads':m['payloads'],'inputs':len(m['final_inputs']),'binary_copies':len(m['binaries']),'rust_attempts':len(attempts),'node_attempts':len(node)}))
