"""Read-only persisted physics/session composition audit; never compiles or runs native code."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

if not __debug__:
    raise RuntimeError('assertions must remain enabled')
p = argparse.ArgumentParser()
p.add_argument('--verify-only', required=True, action='store_true')
p.add_argument('--originals', action='store_true')
p.add_argument('--local-binaries', action='store_true')
a = p.parse_args()
D = Path(__file__).resolve().parent
R = D.parents[3]
h = lambda b: hashlib.sha256(b).hexdigest()
git = lambda *args: subprocess.check_output(['git', *args], cwd=R)

def checked(row):
    b = (R / row['path']).read_bytes()
    assert len(b) == row['bytes'] and h(b) == row['sha256'], row['path']
    return b

M = json.loads((D / 'manifest.json').read_bytes())
checked(M['verifier'])
Q = json.loads(checked(M['qualification']))
for row in M['reviews'] + M['producer_manifests'] + M['producer_archives'] + M['producer_collectors']:
    checked(row)
raw = {}
for row in json.loads(gzip.decompress(checked(M['archive']))):
    b = base64.b64decode(row['data'], validate=True)
    assert len(b) == row['bytes'] and h(b) == row['sha256']
    assert row['path'] not in raw
    raw[row['path']] = b
    if a.originals:
        assert Path(row['path']).read_bytes() == b
assert len(raw) == M['records']
assert Q['main'] == 'c1988638dc6e6b85c58fe24f09525eada43c5aac'
assert Q['executed_source'] == '2352679e52ecbd544a43400e212715000612fa3d'
assert Q['new_builds_or_native_executions'] is False
for parent in [Q['main'], Q['composition'], *Q['producer_packets']]:
    subprocess.run(['git', 'merge-base', '--is-ancestor', parent, 'HEAD'], cwd=R, check=True)
for rows in [json.loads(v) for k, v in raw.items() if k.endswith('/ancestry-joins.json')]:
    for row in rows:
        assert row['before_tree'] == row['after_tree']
        assert git('rev-parse', row['merge'] + '^{tree}').decode().strip() == row['after_tree']
        assert git('rev-parse', row['merge'] + '^1^{tree}').decode().strip() == row['before_tree']
roots = ['crates', 'third_party', 'tools', '.cargo', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'AGENTS.md', 'PLAN.md', 'editor', 'app', 'sdk/rust']
ex = Q['exception']
assert ex['path'] == 'crates/incant_source/src/tests.rs'
assert git('diff', '--name-only', Q['executed_source'], Q['composition'], '--', *roots).decode().splitlines() == [ex['path'], *Q['additive_evidence_collectors']]
old = git('show', Q['executed_source'] + ':' + ex['path'])
new = (R / ex['path']).read_bytes()
assert h(old) == ex['old_sha256'] and h(new) == ex['new_sha256']
assert new == git('show', Q['main'] + ':' + ex['path'])
assert b'#[cfg(test)]\nmod tests;' in (R / 'crates/incant_source/src/lib.rs').read_bytes()
assert len(Q['owned_exact']) == Q['implementation_paths'] == 93
for path, digest in Q['owned_exact'].items():
    assert h((R / path).read_bytes()) == digest == h(git('show', Q['executed_source'] + ':' + path)), path
G = R / 'docs/spikes/evidence/persisted-game-checkpoints'
game_archive = json.loads(gzip.decompress((G / 'records.json.gz').read_bytes()))
game_raw = {r['path']: base64.b64decode(game_archive['payloads'][r['sha256']], validate=True) for r in game_archive['records']}
freeze = json.loads(game_raw['native-all-001/inputs.json'])
assert len(freeze) == Q['source_inputs'] == 3745
for path, digest in freeze.items():
    assert h((R / path).read_bytes()) == (ex['new_sha256'] if path == ex['path'] else digest), path
assert len(Q['commands']) == 12 and Q['graphs_exact'] == 3
for c in Q['commands']:
    assert c['exit'] == 0 and c['reaped'] and h(raw[c['path']]) == c['sha256']
for mode in ['det', 'fast8', 'ordinary']:
    rows = {c['name'].rsplit('-', 1)[1]: c for c in Q['commands'] if c['name'].startswith('graph-' + mode + '-')}
    outputs = {key: raw[c['path']].decode().replace(c['cwd'], '<workspace>') for key, c in rows.items()}
    assert outputs['producer'] == outputs['current']
    if mode == 'ordinary':
        assert 'incant_physics_snapshot' not in outputs['current']
# Native and outer source subtrees remain producer-exact. The only lock delta
# below is the already-reviewed game-runtime checksum edge, included by tested2352679e.
for source, subtrees in [
    ('b78ff58d', ['third_party', 'crates/incant_physics_snapshot', 'Cargo.toml', '.cargo', 'rust-toolchain.toml']),
    ('7f368565', ['third_party', 'crates/incant_physics_runtime', 'crates/incant_physics_2d_runtime', 'crates/incant_runtime', 'crates/incant_physics', 'crates/incant_core', 'crates/incant_types', 'Cargo.toml', '.cargo', 'rust-toolchain.toml']),
]:
    assert not git('diff', '--name-only', source, Q['composition'], '--', *subtrees)
    before = tomllib.loads(git('show', source + ':Cargo.lock').decode())['package']
    after = tomllib.loads((R / 'Cargo.lock').read_text())['package']
    assert len(before) == len(after)
    changed = []
    for b, c in zip(before, after):
        if b != c:
            changed.append(b['name'])
            expected = dict(b)
            expected['dependencies'] = sorted(b['dependencies'] + ['sha2 0.10.9'])
            assert c == expected
    assert changed == ['incant_game_runtime']
O = R / 'docs/spikes/evidence/physics-persisted-outer'
om = json.loads((O / 'manifest.json').read_bytes())
oa = json.loads(gzip.decompress((O / 'records.json.gz').read_bytes()))
assert len(oa['records']) == 165 and len(oa['payloads']) == 119
for row in oa['records']:
    b = base64.b64decode(oa['payloads'][row['sha256']], validate=True)
    assert len(b) == row['bytes'] and h(b) == row['sha256']
    if a.originals:
        assert Path(row['path']).read_bytes() == b
if a.local_binaries:
    for row in om['binary_copies']:
        b = Path(row['path']).read_bytes()
        assert len(b) == row['bytes'] and h(b) == row['sha256']
for name in ['physics-native-persisted', 'persisted-game-checkpoints']:
    argv = ['python3', str(R / 'tools/probes' / name / 'collect.py'), '--verify-only']
    if a.originals:
        argv.append('--originals')
    if a.local_binaries:
        argv.append('--local-binaries')
    subprocess.run(argv, cwd=R, check=True)
print(json.dumps({'verified': True, 'inputs': len(freeze), 'implementation_paths': 93, 'graphs_exact': 3, 'checks': 12, 'retained_current_records': len(raw), 'new_build_or_native_execution': False}))
