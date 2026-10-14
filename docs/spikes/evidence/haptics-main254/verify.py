"""Read-only haptics current-main audit: no builds, tests, hosts or devices."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import re
import subprocess

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
M = json.loads((D / 'manifest.json').read_bytes())
def checked(row):
    b = (R / row['path']).read_bytes()
    assert len(b) == row['bytes'] and h(b) == row['sha256'], row['path']
    return b
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
read = lambda n: json.loads(raw[n])
S = read('source-map.json')
assert S['main'] == Q['main'] == '7c746ebec7e5553bf34c2643ab4c952a1a433db7'
assert S['source'] == Q['tested_source'] == '88e52c482f13aa1ee55c2c9c5506319f618d2d3c'
assert S['producer'] == Q['producer'] == '44b9aead71fe83252445264adb597f51b984843b'
assert Q['packet'] == '08abf8af1fa21f8b6660d1a0918bb9deebb35650'
for ref in [Q['main'], Q['tested_source'], Q['packet']]:
    subprocess.run(['git', 'merge-base', '--is-ancestor', ref, 'HEAD'], cwd=R, check=True)
join = read('ancestry.json')
assert join['before_tree'] == join['after_tree']
assert git('rev-parse', join['head'] + '^{tree}').decode().strip() == join['after_tree']
assert git('rev-parse', join['head'] + '^1^{tree}').decode().strip() == join['before_tree']
freeze = read('inputs.json')
assert len(freeze) == S['current_inputs'] == 3844
proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=R, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
def blob(ref, path):
    proc.stdin.write(f'{ref}:{path}\n'.encode()); proc.stdin.flush()
    header = proc.stdout.readline().split(); assert len(header) == 3 and header[1] == b'blob', path
    b = proc.stdout.read(int(header[2])); assert proc.stdout.read(1) == b'\n'
    return b
try:
    for path, digest in freeze.items():
        assert h((R / path).read_bytes()) == digest, path
        assert h(blob(Q['tested_source'], path)) == digest, path
    assert len(S['owned_exact']) == 27
    for row in S['owned_exact']:
        assert checked(row) == blob(Q['producer'], row['path'])
    roots = ['crates', 'third_party', 'tools', '.cargo', 'sdk', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']
    assert git('diff', '--name-only', Q['main'], Q['tested_source'], '--', *roots).decode().splitlines() == Q['changed_inputs']
    assert git('diff', '--name-only', Q['main'], 'HEAD', '--', *roots).decode().splitlines() == Q['changed_inputs']
    expected_exceptions = {'Cargo.lock', 'crates/incant_game_player/Cargo.toml', 'crates/incant_game_player/src/lib.rs', 'crates/incant_game_player/src/package.rs'}
    assert {r['path'] for r in S['exceptions']} == expected_exceptions
    for row in S['exceptions']:
        path = row['path']
        assert h(blob(Q['producer'], path)) == row['producer_sha256']
        assert h(blob(Q['main'], path)) == row['current_sha256'] == freeze[path]
    producer_archive = json.loads(gzip.decompress((R / 'docs/spikes/evidence/native-haptics/artifacts.json.gz').read_bytes()))
    files = {}
    # Producer stores payloads separately from hash-only duplicate rows.
    for row in producer_archive['records']:
        b = base64.b64decode(producer_archive['payloads'][row['sha256']], validate=True)
        assert len(b) == row['bytes'] and h(b) == row['sha256']
        files[row['relative']] = b
    context = json.loads(files['final-source-context.json'])
    assert len(context['source_files']) == S['producer_inputs'] == 3839
    actual_exceptions = []
    for row in context['source_files']:
        assert h(blob(Q['producer'], row['path'])) == row['sha256']
        if h((R / row['path']).read_bytes()) != row['sha256']:
            actual_exceptions.append(row['path'])
    assert set(actual_exceptions) == expected_exceptions
finally:
    proc.stdin.close(); proc.stdout.close(); assert proc.wait() == 0
result = read('result.json')
assert result['head'] == Q['tested_source'] and result['source_unchanged']
assert result['inputs_sha256'] == h(raw['inputs.json'])
assert result['child_reaped'] and result['target_released']
commands = result['commands'] + read('followup-final.json')['commands'] + read('graphs.json')['commands']
assert len(commands) == 19
for c in commands:
    assert c['reaped'] and c['exit'] == (1 if c['name'] == 'sdk' else 0)
    assert h(raw[Path(c['log']).name]) == c['sha256']
failed = read('followup-sdk-missing-module.json')['commands']
assert len(failed) == 1 and failed[0]['exit'] == 1 and failed[0]['reaped']
assert h(raw[Path(failed[0]['log']).name]) == failed[0]['sha256']
assert len(result['stages']) == len(result['binaries']) == 2
for stage, binary in zip(result['stages'], result['binaries']):
    mode = stage['mode']; numerical = 'physics-' + mode
    assert stage['counts'] == [85, 0, 0]
    assert re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw['test-' + mode + '.log']) == [(b'85', b'0', b'0')]
    artifact = binary['artifact']; messages = []
    for line in raw['build-' + mode + '.log'].splitlines():
        try:
            messages.append(json.loads(line))
        except ValueError:
            pass
    assert artifact in messages and artifact['reason'] == 'compiler-artifact'
    assert artifact['target']['kind'] == ['lib'] and artifact['target']['name'] == 'incant_game_player' and artifact['profile']['test']
    assert set(artifact['features']) == {numerical, 'mobile-host', 'mobile-ffi', 'native-checkpoints', 'native-window', 'render-input', 'render-gpu', 'static-module'}
    assert stage['features'] == numerical + ',native-window,mobile-host,mobile-ffi,native-checkpoints,static-module'
    if a.local_binaries:
        p = Path(binary['path']); b = p.read_bytes()
        assert len(b) == binary['bytes'] and h(b) == binary['sha256'] and p.stat().st_mode & 0o111
G = read('graphs.json'); assert G['exact_graphs'] == 3 and len(G['commands']) == 6
for mode in ['ordinary', 'deterministic', 'fast-simd8']:
    pair = [next(r for r in G['commands'] if r['name'] == f'graph-{mode}-{side}') for side in ['producer', 'current']]
    normalized = [raw[Path(r['log']).name].decode().replace(r['cwd'], '<workspace>') for r in pair]
    assert normalized[0] == normalized[1]
    if mode == 'ordinary':
        assert 'incant_physics_snapshot' not in normalized[0]
ts = read('typescript.json')
assert h((R / 'package-lock.json').read_bytes()) == ts['package_lock_sha256']
assert json.loads((R / 'package-lock.json').read_bytes())['packages']['node_modules/typescript']['version'] == ts['version'] == '5.9.3'
if a.originals:
    for path, digest in ts['files'].items():
        assert h(Path(path).read_bytes()) == digest
    fixture = read('fixture.json'); root = Path(fixture['root'])
    assert h((root / 'manifest.json').read_bytes()) == fixture['manifest_sha256']
    for path, digest in fixture['files'].items():
        assert h((root / path).read_bytes()) == digest
argv = ['python3', str(R / 'docs/spikes/evidence/native-haptics/collect.py'), '--verify-only']
if a.local_binaries:
    argv.append('--local-binaries')
subprocess.run(argv, cwd=R, check=True)
print(json.dumps({'verified': True, 'source_inputs': len(freeze), 'owned_exact': 27, 'producer_inputs': 3839, 'accepted_mobile_exceptions': 4, 'current_player_cases_per_mode': 85, 'successful_commands': 18, 'historical_harness_failures': 2, 'exact_graphs': 3, 'records': len(raw), 'producer_executions_not_relabelled': True}))
