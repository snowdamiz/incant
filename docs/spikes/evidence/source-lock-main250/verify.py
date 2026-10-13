"""Assertion-only verifier; does not build, run tests, or change receipts."""
import argparse, base64, gzip, hashlib, json, re, subprocess
from pathlib import Path
p = argparse.ArgumentParser()
p.add_argument('--verify-only', action='store_true', required=True)
p.add_argument('--local-originals', action='store_true')
p.add_argument('--local-binaries', action='store_true')
a = p.parse_args()
R = Path(__file__).resolve().parents[4]
D = Path(__file__).resolve().parent
M = json.loads((D / 'manifest.json').read_text())
def h(b): return hashlib.sha256(b).hexdigest()
def blob(b): return hashlib.sha1(b'blob ' + str(len(b)).encode() + b'\0' + b).hexdigest()
def git(*args): return subprocess.check_output(['git', *args], cwd=R)
def file(row):
    b = (R / row['path']).read_bytes()
    assert h(b) == row['sha256'] and len(b) == row['bytes'], row['path']
    return b
A = json.loads(gzip.decompress(file(M['archive'])))
assert len(A['records']) == M['archive']['records']
assert len(A['payloads']) == M['archive']['payloads']
payloads = {k: base64.b64decode(v, validate=True) for k, v in A['payloads'].items()}
for k, b in payloads.items(): assert h(b) == k
raw = {}
for row in A['records']:
    b = payloads[row['sha256']]
    assert len(b) == row['bytes'] and row['path'] not in raw
    raw[row['path']] = b
    if a.local_originals: assert Path(row['path']).read_bytes() == b, row['path']
F = json.loads(file(M['freeze']))
assert len(F['files']) == M['input_count'] == 4136
base_blobs = {r.split(b'\t', 1)[1].decode(): r.split()[2].decode()
              for r in git('ls-tree', '-r', '-z', M['base']).split(b'\0') if r}
source = M['source_file']
for row in F['files']:
    b = (R / row['path']).read_bytes()
    assert h(b) == row['sha256'] and len(b) == row['bytes'], row['path']
    assert base_blobs[row['path']] == row['base_blob']
    if row['path'] != source: assert blob(b) == row['base_blob'], row['path']
s = (R / source).read_bytes()
assert h(s) == M['source_sha256']
assert s == git('show', M['source_head'] + ':' + source)
assert git('diff', '--name-only', M['base'], M['source_head']).decode().splitlines() == [source]
patch = gzip.decompress(file(M['patch']))
assert h(patch) == M['patch']['raw_sha256']
assert patch == git('diff', M['base'], M['source_head'], '--', source)
text = s.decode()
assert 'controlled_inherited_source_descriptor' not in text and 'unsafe' not in text
assert '["--exact", TEST, "--test-threads=1", "--nocapture"]' in text
assert 'output.status.success()' in text
assert 'stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")' in text
assert 'SourceStore::open(dir.path(), &dir.path().join("other-journal"), SCRIPT).is_err()' in text
assert 'assert!(actual.is_none(), "actual reopen error: {actual:?}")' in text
assert 'drop(first);' in text
# The remaining recovery tests and production library are unchanged.
baseline = git('show', M['base'] + ':' + source).decode()
assert text.split('#[test]\nfn process_crash_child()', 1)[1] == baseline.split('#[test]\nfn process_crash_child()', 1)[1]
def checked(rows):
    for row in rows:
        assert row['exit'] == 0 and row['reaped']
        for k in ['stdout', 'stderr']: assert h(raw[row[k]['path']]) == row[k]['sha256']
def counts(data):
    rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', data.decode())
    return [tuple(map(int, row)) for row in rows]
checks = M['checks']; assert len(checks) == 15
checked(checks)
byname = {r['name']: r for r in checks}
assert counts(raw[byname['source-tests']['stdout']['path']]) == [(10, 0, 0)]
assert '--all-targets' in byname['source-clippy']['argv']
assert byname['source-clippy']['argv'][-3:] == ['--', '-D', 'warnings']
for i in range(10):
    row = byname['concurrent-' + str(i)]
    assert '--test-threads=10' in row['argv']
    assert counts(raw[row['stdout']['path']]) == [(10, 0, 0)]
diag = M['diagnostic']; checked([diag['result']])
assert counts(raw[diag['result']['stdout']['path']]) == [(11, 0, 0)]
log = raw[diag['result']['stderr']['path']].decode()
assert 'after parent drop, inherited child retained: Io(Custom { kind: Other, error: "WouldBlock" })' in log
assert 'after child release/reap: None' in log and 'controlled child reaped:' in log
assert 'downcast_ref::<std::fs::TryLockError>()' in raw[diag['source']].decode()
assert len(diag['natural_repeats']) == 50
checked(diag['natural_repeats'])
for row in diag['natural_repeats']:
    assert 'controlled_inherited_source_descriptor' in row['argv'] and '--skip' in row['argv']
    assert counts(raw[row['stdout']['path']]) == [(10, 0, 0)]
assert len(M['binaries']) == 2
if a.local_binaries:
    for row in M['binaries']:
        b = Path(row['path']).read_bytes()
        assert h(b) == row['sha256'] and len(b) == row['bytes']
host = M['hosted_failure']
log = raw[host['raw_log']]
assert h(log) == host['sha256']
assert b'tests::root_lock_is_exclusive_across_different_journals ... FAILED' in log
assert b'.is_ok()' in log and b'9 passed; 1 failed;' in log
job = json.loads(raw[host['job']])
assert job['id'] == 115543193346 and job['run_id'] == 38469082532
assert job['head_sha'] == '74e77456133c0aa247c2aee0cdcf1c7c3b719fef'
assert job['conclusion'] == 'failure'
print(json.dumps(dict(verified=True, inputs=len(F['files']), records=len(A['records']),
    payloads=len(payloads), checks=len(checks), cpu_cases=10, concurrent_repeats=10,
    diagnostic_natural_repeats=50, controlled_lock_mechanism=True,
    hosted_cause='inferred; original error not printed', production_changes=False)))
