"""Read-only joined world source and actual native qualification verification."""
from pathlib import Path
import argparse, base64, gzip, hashlib, json, re, subprocess

arg = argparse.ArgumentParser()
arg.add_argument('--verify-only', action='store_true', required=True)
arg.parse_args()
R = Path(__file__).resolve().parents[4]
M = json.loads((R / 'docs/spikes/evidence/physics-world-main225.json').read_text())
h = lambda b: hashlib.sha256(b).hexdigest()
git = lambda *a: subprocess.check_output(['git', *a], cwd=R)

def file(row):
    b = (R / row['path']).read_bytes()
    assert h(b) == row['sha256']
    return b

def archive(row, producer_source_fallback=False):
    data = file(row)
    assert len(data) == row['bytes']
    a = json.loads(gzip.decompress(data))
    assert len(a['records']) == row['records'] and len(a['payloads']) == row['payloads']
    payloads = {k: base64.b64decode(v, validate=True) for k, v in a['payloads'].items()}
    for key, value in payloads.items():
        assert h(value) == key
    result = {}
    for row in a['records']:
        b = payloads[row['sha256']]
        assert len(b) == row['bytes']
        original = Path(row['path']).read_bytes()
        if original != b:
            assert producer_source_fallback
            assert row['path'] == '/Users/andreyyurlov/.codex/worktrees/color-mip-roles/incant/STATUS.md'
            assert git('show', M['producer'] + ':STATUS.md') == b
        result[row['path']] = b
    return result

raw = archive(M['archive'])
freeze = json.loads(file(M['freeze']))
exceptions = {x['path']: x for x in M.get('main_join_exemptions', [])}
seen = set()
for row in freeze['files']:
    assert h(git('show', M['source_head'] + ':' + row['path'])) == row['sha256']
    actual = h((R / row['path']).read_bytes())
    if actual != row['sha256']:
        x = exceptions[row['path']]
        assert x['tested_sha256'] == row['sha256'] and actual == x['current_sha256']
        assert actual == h(git('show', M['main_ancestor'] + ':' + row['path']))
        seen.add(row['path'])
assert seen == set(exceptions)
scope = ['crates', 'tools', 'third_party', 'Cargo.toml', 'Cargo.lock', '.cargo', '.github', 'sdk', 'schemas', 'editor', 'rust-toolchain.toml']
joined = git('diff', '--name-only', M['source_head'], M['ancestry_head'], '--', *scope).decode().splitlines()
assert sorted(joined) == sorted(x['path'] for x in M['main_join_paths'])
assert len(joined) == 8 and all(x.startswith('crates/incant_render/') for x in joined)
for row in M['main_join_paths']:
    assert h((R / row['path']).read_bytes()) == row['sha256'] == h(git('show', M['main_ancestor'] + ':' + row['path']))
identity = json.loads(file(M['source_identity']))
for row in identity['files']:
    actual = h((R / row['path']).read_bytes())
    assert actual == row['sha256']
    if row['path'] != 'crates/incant_physics_snapshot/src/lib.rs':
        assert actual == h(git('show', M['producer'] + ':' + row['path']))
    else:
        assert row['role'] == 'reviewed-current-error-order'
        old = git('show', M['producer'] + ':' + row['path']).decode()
        new = (R / row['path']).read_text()
        # Preserve current-main declaration order without changing any enum member.
        def normalized(s):
            a = s.index('pub enum Error {')
            b = s.index('\n}', a)
            body = s[a:b].splitlines()
            return s[:a] + '\n'.join([body[0], *sorted(body[1:])]) + s[b:]
        assert normalized(old) == normalized(new)
assert not git('diff', '--name-only', M['base'], 'HEAD', '--', 'Cargo.lock', 'Cargo.toml', 'crates/incant_physics_snapshot/Cargo.toml').strip()
assert git('merge-base', '--is-ancestor', M['main_ancestor'], 'HEAD') == b''
assert len(M['checks']) == 13
for row in M['checks']:
    assert row['exit'] == 0 and row['reaped']
    for key in ['stdout', 'stderr']:
        assert h(raw[row[key]['path']]) == row[key]['sha256']

def messages(text):
    for line in text.splitlines():
        if line.startswith('{'):
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                pass

for label, det in [('default', True), ('fast8', False)]:
    for suffix, count, parallel in [('tests', 202, False), ('two-worker-world', 10, True)]:
        row = next(x for x in M['checks'] if x['name'] == label + '-' + suffix)
        if parallel:
            assert row['argv'][:2] == ['env', 'RAYON_NUM_THREADS=2']
        text = raw[row['stdout']['path']].decode()
        counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
        assert len(counts) == (1 if parallel else 24)
        assert tuple(sum(int(x[i]) for x in counts) for i in range(3)) == (count, 0, 0)
        artifacts = list(messages(text))
        for name in ['rapier2d', 'rapier3d']:
            rows = [x for x in artifacts if x.get('reason') == 'compiler-artifact' and x['target']['name'] == name]
            assert rows
            for artifact in rows:
                features = set(artifact['features'])
                assert 'incant-checkpoint' in features and 'serde-serialize' in features
                assert ('enhanced-determinism' in features) == det
                assert ('simd8' in features) == (not det)
                if parallel:
                    assert 'parallel' in features
        stderr = raw[row['stderr']['path']].decode()
        for dim in [2, 3]:
            for steps in [96, 32, 12]:
                assert f'WORLD_BOUNDARY dimension={dim} steps={steps} ' in stderr
        assert stderr.count('typed_clone=true inert_bytes_not_installed=true') == 2
        assert stderr.count('canonical_sections=22 hostile=true') == 2
graph = next(x for x in M['checks'] if x['name'] == 'normal-graph')
lines = raw[graph['stdout']['path']].decode().splitlines()
assert len(lines) == 1 and lines[0].startswith('incant_physics_snapshot v0.0.1')
producer = json.loads(file(M['producer_manifest']))
producer_raw = archive(producer['archive'], producer_source_fallback=True)
assert len(producer_raw) == 608
for row in producer['binaries'] + M['binaries']:
    b = Path(row['path']).read_bytes()
    assert h(b) == row['sha256'] and len(b) == row['bytes']
assert h(file(M['producer_review'])) == 'd9f5dce4c382cd4e1a1b1f796151c2cfdf18d1b81d58aa7e40e2e0eb74014619'
print(json.dumps(dict(verified=True, current_records=M['archive']['records'],
                     frozen_inputs=len(freeze['files']), physics_inputs=len(identity['files']),
                     checks=13, default=202, fast8=202, parallel_each=10,
                     continuation_steps_each_mode=280, main_join_exemptions=len(exceptions),
                     normal_graph='std-only', decoded_native_install=False, live_save_admission=False)))
