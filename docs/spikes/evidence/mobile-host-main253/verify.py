"""Read-only current-main mobile-host audit. Never builds or executes native code."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
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
for row in M['producer_files'] + M['reviews']:
    checked(row)
raw = {}
for row in json.loads(gzip.decompress(checked(M['archive']))):
    b = base64.b64decode(row['data'], validate=True)
    assert len(b) == row['bytes'] and h(b) == row['sha256']
    assert row['path'] not in raw
    raw[row['path']] = b
    if a.originals:
        assert Path(row['original']).read_bytes() == b
assert len(raw) == M['records']
assert Q['main'] == '31519f8be9d09ce53eb93dc68b69ec275915492e'
assert Q['tested_source'] == '3e257acdb7b8173dad70f63675ab39e04cd2fd97'
assert Q['producer'] == 'd3a7b3eda62807db3aaf3d46229b5fb990f2d780'
assert Q['packet'] == '26dd8f696ca79424812720631f625a4eb765a7ac'
for parent in [Q['main'], Q['tested_source'], Q['packet']]:
    subprocess.run(['git', 'merge-base', '--is-ancestor', parent, 'HEAD'], cwd=R, check=True)
join = json.loads(raw['ancestry.json'])
assert join['before_tree'] == join['after_tree']
assert git('rev-parse', join['head'] + '^{tree}').decode().strip() == join['after_tree']
assert git('rev-parse', join['head'] + '^1^{tree}').decode().strip() == join['before_tree']
freeze = json.loads(raw['inputs.json'])
assert len(freeze) == Q['source_inputs']
proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=R, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
for path, digest in freeze.items():
    assert h((R / path).read_bytes()) == digest, path
    proc.stdin.write(f"{Q['tested_source']}:{path}\n".encode())
    proc.stdin.flush()
    header = proc.stdout.readline().split()
    assert len(header) == 3 and header[1] == b'blob', path
    b = proc.stdout.read(int(header[2]))
    assert proc.stdout.read(1) == b'\n' and h(b) == digest, path
proc.stdin.close()
assert proc.wait() == 0
roots = ['crates', 'third_party', 'tools', '.cargo', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'PLAN.md', 'AGENTS.md']
paths = git('diff', '--name-only', Q['main'], Q['tested_source'], '--', *roots).decode().splitlines()
assert paths == Q['changed_inputs']
assert set(paths) == {
    'Cargo.lock', 'crates/incant_game_player/Cargo.toml',
    'crates/incant_game_player/src/lib.rs', 'crates/incant_game_player/src/package.rs',
    'crates/incant_game_player/src/mobile_host.rs', 'crates/incant_game_player/src/mobile_host/tests.rs',
    'tools/probes/mobile-retained-host/collect.py',
}
for path in Q['producer_exact']:
    assert (R / path).read_bytes() == git('show', Q['producer'] + ':' + path), path
assert Q['producer_exact'] == [
    'crates/incant_game_player/src/mobile_host.rs',
    'crates/incant_game_player/src/mobile_host/tests.rs',
    'crates/incant_game_player/src/package.rs',
]
path = 'crates/incant_game_player/src/lib.rs'
before = git('show', Q['main'] + ':' + path)
assert (R / path).read_bytes() == before.replace(b'pub mod mobile;\n', b'pub mod mobile;\n#[cfg(feature = "mobile-host")]\npub mod mobile_host;\n', 1)
path = 'crates/incant_game_player/Cargo.toml'
before = tomllib.loads(git('show', Q['main'] + ':' + path).decode())
after = tomllib.loads((R / path).read_text())
expected = json.loads(json.dumps(before))
expected['features']['mobile-host'] = ['render-input', 'dep:thiserror']
expected['dev-dependencies']['incant_render_data'] = {'workspace': True}
assert after == expected
old = tomllib.loads(git('show', Q['main'] + ':Cargo.lock').decode())
new = tomllib.loads((R / 'Cargo.lock').read_text())
expected = json.loads(json.dumps(old))
player = next(row for row in expected['package'] if row['name'] == 'incant_game_player')
player['dependencies'] = sorted(player['dependencies'] + ['incant_render_data'])
assert new == expected
result = json.loads(raw['result.json'])
assert result['head'] == Q['tested_source'] and result['source_unchanged']
assert result['target_released'] and result['child_reaped']
assert result['inputs_sha256'] == h(raw['inputs.json'])
assert len(result['commands']) == Q['commands'] == 26
names = []
for mode, harnesses in [('mobile', 1), ('native-det', 6), ('native-fast8', 6)]:
    names += ['build-' + mode] + [f'test-{mode}-{i}' for i in range(harnesses)]
names += ['clippy-mobile', 'clippy-native-det', 'clippy-native-fast8', 'clippy-ordinary',
          'ios-mobile', 'manifest', 'runtime-deps', 'conventions', 'fmt', 'diff']
assert [row['name'] for row in result['commands']] == names
for row in result['commands']:
    assert row['exit'] == 0 and row['reaped']
    assert h(raw[Path(row['log']).name]) == row['sha256']
expected_features = {
    'mobile': 'physics-deterministic,mobile-host,mobile-ffi',
    'native-det': 'physics-deterministic,mobile-host,mobile-ffi,native-checkpoints',
    'native-fast8': 'physics-fast-simd8,mobile-host,mobile-ffi,native-checkpoints',
}
assert len(result['binaries']) == Q['binary_copies'] == 15
for stage in result['stages']:
    mode = stage['mode']
    assert stage['features'] == expected_features[mode]
    counts = [0, 0, 0]
    logs = [row for row in result['commands'] if row['name'].startswith('test-' + mode + '-')]
    assert len(logs) == stage['harnesses'] == (1 if mode == 'mobile' else 6)
    for row in logs:
        found = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw[Path(row['log']).name])
        assert len(found) == 1
        counts = [x + int(y) for x, y in zip(counts, found[0])]
    assert counts == stage['counts'] == ([31, 0, 0] if mode == 'mobile' else [34, 0, 0])
    messages = []
    for line in raw['build-' + mode + '.log'].splitlines():
        try:
            messages.append(json.loads(line))
        except ValueError:
            pass
    binaries = [b for b in result['binaries'] if Path(b['path']).parent.name == mode]
    assert len(binaries) == (1 if mode == 'mobile' else 7)
    lib = cli = 0
    for binary in binaries:
        artifact = binary['artifact']
        assert artifact in messages and artifact['reason'] == 'compiler-artifact'
        assert '/crates/incant_game_player/' in artifact['target']['src_path']
        assert '/aarch64-apple-darwin/' in artifact['executable']
        features = set(artifact['features'])
        assert {'mobile-host', 'mobile-ffi', 'render-input'} <= features
        numerical = 'physics-fast-simd8' if mode == 'native-fast8' else 'physics-deterministic'
        assert numerical in features
        assert ('physics-deterministic' if mode == 'native-fast8' else 'physics-fast-simd8') not in features
        assert ('native-checkpoints' in features) == (mode != 'mobile')
        if artifact['target']['kind'] == ['lib'] and artifact['profile']['test']:
            lib += 1
        if artifact['target']['kind'] == ['bin'] and not artifact['profile']['test']:
            cli += 1
            assert artifact['target']['name'] == 'incant_game_player' and binary['path'] == stage['player']
        if a.local_binaries:
            b = Path(binary['path']).read_bytes()
            assert len(b) == binary['bytes'] and h(b) == binary['sha256']
    assert lib == 1 and cli == (0 if mode == 'mobile' else 1)
assert len(result['stages']) == 3
argv = ['python3', str(R / 'tools/probes/mobile-retained-host/collect.py'), '--verify-only']
if a.originals:
    argv.append('--originals')
if a.local_binaries:
    argv.append('--local-binaries')
subprocess.run(argv, cwd=R, check=True)
print(json.dumps({'verified': True, 'source_inputs': len(freeze), 'records': len(raw),
                  'commands': 26, 'binary_copies': 15, 'mobile_library': 31,
                  'native_player_each_mode': 34, 'producer_tests_not_relabelled': True}))
