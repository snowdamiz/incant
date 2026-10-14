#!/usr/bin/env python3
"""Verify retained pin migration composition; never build or execute test binaries."""
import argparse
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--verify-only', action='store_true')
p.add_argument('--current', action='store_true')
p.add_argument('--originals', action='store_true')
p.add_argument('--local-binaries', action='store_true')
a = p.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
m = json.loads((D/'manifest.json').read_bytes())
assert h(Path(__file__).read_bytes()) == m['verifier_sha256']

def archive(directory, manifest, original):
    raw = (directory/'records.json.gz').read_bytes()
    assert h(raw) == manifest['archive_sha256']
    v = json.loads(gzip.decompress(raw))
    assert len(v['records']) == manifest['records'] and len(v['payloads']) == manifest['payloads']
    payload = {k: base64.b64decode(b, validate=True) for k, b in v['payloads'].items()}
    assert all(h(b) == k for k, b in payload.items())
    out = {}
    for row in v['records']:
        assert row['path'] not in out
        b = payload[row['sha256']]; assert len(b) == row['bytes']
        if a.originals: assert (Path(original)/row['path']).read_bytes() == b, row['path']
        out[row['path']] = b
    return out

def git_rows(source, rows, key='sha256'):
    v = subprocess.run(['git', 'cat-file', '--batch'], cwd=R, check=True, capture_output=True,
        input=''.join(source+':'+q['path']+'\n' for q in rows).encode())
    stream = io.BytesIO(v.stdout)
    for q in rows:
        header = stream.readline().split(); assert len(header) == 3 and header[1] == b'blob', q['path']
        data = stream.read(int(header[2])); assert stream.read(1) == b'\n'
        assert h(data) == q[key], (source, q['path'])
    assert stream.read() == b''

def counts(data):
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', data.decode(errors='replace'))
    return [sum(int(q[i]) for q in rows) for i in range(3)]

x = archive(D, m, m['evidence_root'])
obj = lambda n: json.loads(x[n])
freezes = {g: obj('freeze-'+g+'.json') for g in ['001','002','003','004','005']}
for f in freezes.values():
    assert len(f['rows']) == 3559
    git_rows(f['source'], f['rows'])
f = freezes['005']; assert f['source'] == m['source']
if a.current:
    for q in f['rows']: assert h((R/q['path']).read_bytes()) == q['sha256'], q['path']
for ref in [m['base'], freezes['001']['source']]:
    subprocess.run(['git','merge-base','--is-ancestor',ref,m['source']], cwd=R,check=True)
def changed(left,right):
    l = {q['path']:q['sha256'] for q in freezes[left]['rows']}
    return {q['path'] for q in freezes[right]['rows'] if q['sha256'] != l[q['path']]}
assert changed('001','005') == {'crates/incant_cmd/src/journal.rs','crates/incant_cmd/src/project_file_tests.rs','crates/incant_cmd/tests/engine_version.rs'}
assert changed('004','005') == {'crates/incant_cmd/tests/engine_version.rs'}
patch = subprocess.check_output(['git','diff',freezes['001']['source'],m['source'],'--',*sorted(changed('001','005'))],cwd=R)
assert patch == x['windows-test-repair.patch']
scope = obj('source-relevance-final.json')
assert scope['final_source'] == m['source'] and scope['producer'] == m['producer']
assert len(scope['final_owned']) == 12
for key, rev in [('producer_sha256',m['producer']),('final_sha256',m['source'])]:
    git_rows(rev,scope['final_owned'],key)
assert len(scope['final_producer_differences']) == 10
for q in scope['final_producer_differences']:
    assert h(x['producer-exceptions/'+q['path']]) == q['producer_sha256']
# The original seven source joins remain exact to main except the reviewed command union.
git_rows(m['base'], [q for q in scope['producer_input_changes'] if q['path'] != 'crates/incant_cmd/src/lib.rs'], 'current_sha256')

binaries = []
for result, runner, generation, names, exits in [
 ('results.json','run.py','001',['source-doc-cmd','headless','clippy','schema','sdk-fresh','sdk-types','fmt','conventions','diff'],[0]*9),
 ('results-003.json','run-003.py','003',['engine-version-003'],[101]),
 ('results-004.json','run-004.py','004',['cache-invalidate-004','engine-version-004','project-file-004','clippy-004'],[0,0,0,101]),
 ('results-005.json','run-005.py','005',['engine-version-005','clippy-005','fmt-005','conventions-005','diff-005'],[0]*5)]:
    checks = obj(result); assert [q['name'] for q in checks] == names
    assert [q['exit_code'] for q in checks] == exits
    for c in checks:
        name = c['name']; assert c == obj(name+'.json')
        assert c['source_unchanged'] and c['managed_child_tree_reaped']
        assert c['runner_sha256'] == h(x[runner]) and c['freeze_sha256'] == h(x['freeze-'+generation+'.json'])
        assert c['stdout_sha256'] == h(x[name+'.stdout']) and c['stderr_sha256'] == h(x[name+'.stderr'])
        assert c['environment']['CARGO_BUILD_JOBS'] == '2'
        binaries += c['binaries']
assert counts(x['source-doc-cmd.stdout']) == [127,0,0]
assert counts(x['headless.stdout']) == [2,0,0]
assert counts(x['engine-version-004.stdout']) == counts(x['engine-version-005.stdout']) == [4,0,0]
assert counts(x['project-file-004.stdout']) == [15,0,0]
assert b'could not find `prefab` in `incant_doc`' in x['engine-version-003.stderr']
assert b'variable does not need to be mutable' in x['clippy-004.stderr']
clean = obj('cache-invalidate-004.json')
assert clean['argv'] == ['./tools/cargo','clean','--profile','ci','--target','aarch64-apple-darwin','-p','incant_doc']
assert clean['environment']['CARGO_TARGET_DIR'] == '/tmp/incant-agent-font-target'
schema = obj('schema-comparison.json'); assert schema['all_equal'] and len(schema['rows']) == 74
git_rows(m['source'],schema['rows'])
for row in schema['rows']:
    assert h(x['generated-schemas/'+Path(row['path']).name]) == row['sha256']
assert schema['binary'] in obj('headless.json')['binaries']
assert h(x['producer-review.json']) == m['producer_review_sha256']
assert h(x['windows-source-review.json']) == m['windows_source_review_sha256']
assert b'code: 33' in x['checkpoint33-windows/job.log']
assert obj('checkpoint33-windows/job.json')['conclusion'] == 'failure'

PD = R/'docs/spikes/evidence/project-pin-migration'
assert h((PD/'manifest.json').read_bytes()) == m['producer_manifest_sha256']
pm = json.loads((PD/'manifest.json').read_bytes())
assert h((PD/'collect.py').read_bytes()) == pm['collector_sha256']
y = archive(PD,pm,m['producer_evidence_root']); load = lambda n:json.loads(y[n])
assert len(pm['final_inputs']) == 3542; git_rows(m['producer'],pm['final_inputs'])
final = {q['path']:q['sha256'] for q in f['rows']}
assert {q['path'] for q in pm['final_inputs'] if q['sha256'] != final[q['path']]} == {q['path'] for q in scope['final_producer_differences']}
for generation in ['source-001','source-002','source-003']:
    pf = load(generation+'/freeze.json')
    for q in pf['files']: assert h(y[generation+'/inputs/'+q['path']]) == q['sha256']
    assert h(y[generation+'/tracked.patch']) == pf['tracked_patch_sha256']
for name, expected in pm['rust_attempts'].items():
    c = load(name+'/receipt.json'); assert c['exit_code'] == expected and c['source_unchanged']
    assert c.get('owned_process_group_cleaned_and_reaped') or (c['reaped'] and not c['remaining_group'])
    assert h(y[name+'/output.log']) == c['log_sha256'] and h(y[name+'/freeze.json']) == c['freeze_sha256']
    pf = load(name+'/freeze.json')
    assert h(y[pf['source_freeze'].removeprefix(m['producer_evidence_root']+'/')]) == c['source_freeze_sha256']
assert counts(y['all-001/output.log']) == [87,0,0] and counts(y['lib-002/output.log']) == [36,0,0]
assert b'19 passed; 1 failed' in y['lib-001/output.log'] and b'project abort projection mismatch' in y['lib-001/output.log']
for c in load('lean-001/checks.json'):
    assert c['exit_code'] == 0 and c['owned_process_group_cleaned_and_reaped']
    assert h(y['lean-001/'+c['name']+'.log']) == c['log_sha256']
assert load('crdt-diagnosis-002/receipt.json')['build_exit'] == 1
assert load('crdt-diagnosis-003/receipt.json')['build_exit'] == load('crdt-diagnosis-003/receipt.json')['run_exit'] == 0
assert b'observed_mismatches=14/64' in y['crdt-diagnosis-003/run.log']
assert y['crdt-diagnosis-003/durable.loro'] != y['crdt-diagnosis-003/imported.loro']
assert y['crdt-diagnosis-003/imported.loro'] == y['crdt-diagnosis-003/after-stage.loro']
assert load('abort-diagnosis-001/receipt.json')['exact_equal']
assert load('release.json')['builder_slot_released']
if a.local_binaries:
    for q in binaries:
        path = Path(q['copy']); assert h(path.read_bytes()) == q['sha256']
        assert path.stat().st_size == q['size'] and path.stat().st_mode & 0o111
    for q in pm['binaries']:
        path = Path(q['path']); assert h(path.read_bytes()) == q['sha256'] and path.stat().st_mode & 0o111
print(json.dumps({'verified':True,'current_records':m['records'],'current_inputs':3559,'current_001_cases':129,
    'affected_004_cases':19,'affected_005_cases':4,'current_binary_copies':len(binaries),'producer_records':pm['records'],
    'producer_inputs':3542,'producer_copies':len(pm['binaries']),'windows_execution_of_repair':False}))
