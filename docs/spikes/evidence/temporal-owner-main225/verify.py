from pathlib import Path
import base64, datetime, gzip, hashlib, json, os, re, shutil, subprocess, sys, tempfile

R = Path('/Users/andreyyurlov/.codex/worktrees/grid-navigation/incant')
E = Path.home() / '.codex/incant-integration-evidence'
A = E / 'temporal-owner-main224-001'
B = E / 'temporal-owner-main224-002'
D = R / 'docs/spikes/evidence/temporal-owner-main225'
BASE = '0aa025591ab5c82e00d6d2b70002024ed3462e2a'
MAIN = 'd62976cc5adc740b5b0541abcbfd59d4069b4a45'
TESTED = '9e9abec425033c59b9691503448d14d190a95780'
JOIN = '11ce57b5'
PRODUCER = 'af7efed1a89f51e33bfc2fa75f2da046b428dcf5'
PRIVATE = '2fe23dceb4cbedfae860011ad79eab7da95511fc'
EXEMPT = 'crates/incant_cmd/src/agent_budget_replay_tests.rs'
sha = lambda data: hashlib.sha256(data).hexdigest()
h = lambda path: sha(Path(path).read_bytes())
read = lambda path: json.loads(Path(path).read_text())
git = lambda *args: subprocess.check_output(['git', *args], cwd=R)

f = read(B / 'freeze.json')
assert f == read(A / 'freeze.json') and f['head'] == TESTED and f['base'] == BASE
assert git('merge-base', '--is-ancestor', MAIN, 'HEAD') == b''
assert subprocess.run(['git', 'merge-base', '--is-ancestor', PRIVATE, 'HEAD'], cwd=R).returncode == 1
exceptions = []
for row in f['files']:
    assert sha(git('show', TESTED + ':' + row['path'])) == row['sha256']
    if h(R / row['path']) != row['sha256']:
        assert row['path'] == EXEMPT
        assert (R / row['path']).read_bytes() == git('show', MAIN + ':' + row['path'])
        exceptions.append(dict(path=row['path'], tested_sha256=row['sha256'], current_sha256=h(R / row['path'])))
assert len(exceptions) == 1
mapping = read(A / 'source-map.json')
patch = (A / 'producer-rust.patch').read_bytes()
assert sha(patch) == mapping['patch_sha256'] == 'ee4971ccea816c4fb7cb2c111ae8b890a39373b809d33f694691b9f1d16f743d'
with tempfile.TemporaryDirectory() as temp:
    env = dict(os.environ, GIT_INDEX_FILE=str(Path(temp) / 'index'))
    subprocess.run(['git', 'read-tree', BASE], cwd=R, env=env, check=True)
    subprocess.run(['git', 'apply', '--cached', '-'], input=patch, cwd=R, env=env, check=True)
    for row in mapping['files']:
        blob = subprocess.check_output(['git', 'show', ':' + row['path']], cwd=R, env=env)
        assert sha(blob) == row['sha256'] == h(R / row['path'])
expected = sorted(row['path'] for row in mapping['files'])
scope = ['crates', 'tools', 'Cargo.toml', 'Cargo.lock', '.cargo', '.github', 'third_party', 'sdk', 'schemas', 'editor', 'rust-toolchain.toml']
assert sorted(git('diff', '--name-only', MAIN, 'HEAD', '--', *scope).decode().splitlines()) == expected
for path in git('ls-tree', '-r', '--name-only', TESTED, '--', 'crates/incant_render').decode().splitlines():
    if path.endswith('.wgsl'):
        assert (R / path).read_bytes() == git('show', MAIN + ':' + path)
assert git('diff', '--name-only', TESTED, JOIN, '--', *scope).decode().splitlines() == [EXEMPT]

old = read(A / 'results.json')
checks = read(B / 'results.json')
assert [(x['name'], x['exit']) for x in old] == [('scoped-clean', 0), ('default-cpu', 101)]
assert len(checks) == 12 and all(x['exit'] == 0 and x['reaped'] for x in checks)
for row in old + checks:
    for key in ['stdout', 'stderr']:
        assert h(row[key]['path']) == row[key]['sha256']
counts = {}
for name, expected_counts in [('default-cpu', [143, 0, 103]), ('cooked-cpu', [172, 0, 103])]:
    row = next(x for x in checks if x['name'] == name)
    text = Path(row['stdout']['path']).read_text()
    groups = [list(map(int, x)) for x in re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)]
    assert list(map(sum, zip(*groups))) == expected_counts
    counts[name] = dict(passed=expected_counts[0], failed=0, ignored=103, groups=len(groups), nonempty_groups=sum(x[0] > 0 for x in groups))
binaries = read(B / 'binaries.json')
assert len(binaries) == 4
for row in binaries:
    assert h(row['path']) == row['sha256'] and Path(row['path']).stat().st_size == row['bytes']

p = read(R / 'docs/spikes/evidence/temporal-owner-provenance.json')
z = (R / p['archive']['path']).read_bytes()
assert sha(z) == p['archive']['sha256']
raw = gzip.decompress(z)
assert sha(raw) == p['archive']['raw_sha256']
producer_archive = json.loads(raw)
assert len(producer_archive['records']) == 56 and len(producer_archive['payloads']) == 54
for row in producer_archive['records']:
    data = base64.b64decode(producer_archive['payloads'][row['sha256']])
    assert sha(data) == row['sha256'] and len(data) == row['bytes']
    assert Path(row['path']).read_bytes() == data
for row in p['binaries']:
    assert h(row['copy']) == row['sha256'] and Path(row['copy']).stat().st_size == row['bytes']
review = R / 'docs/spikes/evidence/temporal-owner-provenance-final-review.json'
assert h(review) == '88b8a6e0dcb63003546530dcf93054943b4205e8678d1b8ee817d6fe4ef401ac'

summary = dict(frozen_inputs=len(f['files']), current_source_paths=len(expected), checks=12,
               prior_failure_preserved=True, counts=counts, cpu_suites_overlap=True,
               producer_records=56, producer_payloads=54, producer_binaries=8,
               current_binaries=4, current_gpu_execution=False, private_ancestry_included=False)
print(json.dumps(summary), flush=True)
manifest_path = R / 'docs/spikes/evidence/temporal-owner-main225.json'
if '--verify-only' in sys.argv:
    m = read(manifest_path)
    data = (R / m['archive']['path']).read_bytes()
    assert sha(data) == m['archive']['sha256']
    archive = json.loads(gzip.decompress(data))
    assert len(archive['records']) == m['archive']['records']
    for row in archive['records']:
        b = base64.b64decode(archive['payloads'][row['sha256']])
        assert sha(b) == row['sha256'] and len(b) == row['bytes']
        assert Path(row['original']).read_bytes() == b
    assert m['summary'] == summary
    raise SystemExit(0)

D.mkdir(parents=True, exist_ok=True)
records, payloads = [], {}
for root in [A, B]:
    for path in sorted(root.rglob('*')):
        if not path.is_file() or 'binaries' in path.relative_to(root).parts:
            continue
        data = path.read_bytes()
        s = sha(data)
        records.append(dict(original=str(path), sha256=s, bytes=len(data)))
        payloads.setdefault(s, base64.b64encode(data).decode())
archive = gzip.compress(json.dumps(dict(records=records, payloads=payloads), separators=(',', ':')).encode(), mtime=0)
assert len(archive) < 16 * 1024 * 1024
(D / 'records.json.gz').write_bytes(archive)
shutil.copyfile(__file__, D / 'verify.py')
shutil.copyfile(B / 'freeze.json', D / 'freeze.json')
shutil.copyfile(B / 'run.py', D / 'run.py')
manifest = dict(schema='incant-temporal-owner-main225-v1', tested_head=TESTED,
                source_ancestry=git('rev-parse', JOIN).decode().strip(), main=MAIN, producer=PRODUCER,
                summary=summary, source_map=mapping, current_main_exemptions=exceptions,
                checks=checks, initial_attempt=old, binaries=binaries,
                archive=dict(path=str((D / 'records.json.gz').relative_to(R)), sha256=sha(archive),
                             bytes=len(archive), records=len(records), payloads=len(payloads)),
                freeze=dict(path=str((D / 'freeze.json').relative_to(R)), sha256=h(D / 'freeze.json')),
                producer_manifest=dict(path='docs/spikes/evidence/temporal-owner-provenance.json',sha256=h(R / 'docs/spikes/evidence/temporal-owner-provenance.json')),
                review_state='Current-main independent final review pending',
                limits=['CPU foundation only; GPU tests remain ignored and no new GPU/appearance/device/performance result.',
                        'Eight isolated Rust deltas applied to actual main; private upscaling/MetalFX ancestry is absent.',
                        'Current main225 differs from tested source only in the independently accepted journal test within executable scope; no renderer build is claimed on the ancestry child.',
                        'Initial build reused stale incant_text/render_data artifacts lacking current halo APIs. Scoped cleanup of incant_text/render_data/ui recovered unchanged source. Original failure, hashes and logs retained.',
                        'No shader or default/resource-budget changes. Integer shader transport, per-pixel ownership and temporal quality repair remain open.'])
manifest_path.write_text(json.dumps(manifest, indent=2) + '\n')
print(str(manifest_path), h(manifest_path), flush=True)
