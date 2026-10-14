"""Read-only grouped mobile source/evidence audit. No builds, tests or devices."""
import argparse
import base64
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
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
M = json.loads((D / 'manifest.json').read_bytes())
def checked(row):
    b = (R / row['path']).read_bytes()
    assert len(b) == row['bytes'] and h(b) == row['sha256'], row['path']
    return b
checked(M['verifier'])
Q = json.loads(checked(M['qualification']))
for row in M['reviews'] + M['producer_files']:
    checked(row)
archive = json.loads(gzip.decompress(checked(M['archive'])))
raw = {}
for row in archive['records']:
    b = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
    assert len(b) == row['bytes'] and h(b) == row['sha256']
    assert row['path'] not in raw
    raw[row['path']] = b
    if a.originals:
        assert Path(row['original']).read_bytes() == b
assert len(raw) == M['records']
read = lambda name: json.loads(raw[name])
source = 'a928652ad67a714c9709cfb12db813a0f3163378'
initial = 'd0777898cd60b20197fa72cb02b7a81c94c73721'
main = 'd0b371d454951543fab7822e739149e35b5b5364'
assert Q['source'] == source and Q['main'] == main
for ref in [source, main, Q['apple_packet'], Q['android_packet']]:
    subprocess.run(['git', 'merge-base', '--is-ancestor', ref, 'HEAD'], cwd=R, check=True)
for row in read('ancestry.json'):
    assert row['before_tree'] == row['after_tree']
    assert git('rev-parse', row['head'] + '^{tree}').decode().strip() == row['after_tree']
    assert git('rev-parse', row['head'] + '^1^{tree}').decode().strip() == row['before_tree']
assert git('rev-parse', main + '^{tree}') == git('rev-parse', Q['reviewed_haptics'] + '^{tree}')
S = read('source-map.json'); F = read('final-002/inputs.json'); I = read('inputs.json')
assert len(F) == len(I) == S['inputs'] == 3875
assert len(S['paths']) == 28 and S['source'] == initial
proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=R, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
def blob(ref, path):
    proc.stdin.write(f'{ref}:{path}\n'.encode()); proc.stdin.flush()
    header = proc.stdout.readline().split(); assert len(header) == 3 and header[1] == b'blob', path
    b = proc.stdout.read(int(header[2])); assert proc.stdout.read(1) == b'\n'
    return b
try:
    for path, digest in F.items():
        assert h((R / path).read_bytes()) == h(blob(source, path)) == digest, path
        assert h(blob(initial, path)) == I[path], path
    assert set(p for p in F if F[p] != I[p]) == {'crates/incant_game_player/src/mobile_native.rs', 'crates/incant_game_player/src/window/mod.rs'}
    assert git('diff', initial, source, '--', 'crates') == raw['clock-composition.patch']
    exact = 0
    for row in S['paths']:
        path = row['path']; assert h(blob(initial, path)) == row['sha256']
        for label, ref in [('apple', Q['apple_packet']), ('android', Q['android_packet'])]:
            if label + '_sha256' in row:
                digest = h(blob(ref, path)); assert digest == row[label + '_sha256']
                assert row[label + '_exact'] == (digest == row['sha256'])
        exact += row.get('apple_exact', False) or row.get('android_exact', False)
    assert exact == 26
    roots = ['crates', 'third_party', 'tools', '.cargo', 'sdk', 'platforms', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']
    changed = git('diff', '--name-only', main, source, '--', *roots).decode().splitlines()
    assert changed == Q['changed_inputs'] and len(changed) == 29
    assert git('diff', '--name-only', source, 'HEAD', '--', *roots) == b''
    locks = [tomllib.loads(blob(ref, 'Cargo.lock').decode()) for ref in [main, source]]
    ids = [{(p['name'], p['version'], p.get('source'), p.get('checksum')) for p in lock['package']} for lock in locks]
    assert ids[0] == ids[1] and len(ids[1]) == 968
    packages = [{(p['name'], p['version'], p.get('source')): p for p in lock['package']} for lock in locks]
    delta = [key for key in packages[0] if packages[0][key] != packages[1][key]]
    assert delta == [('incant_game_player', '0.0.1', None)]
    before, after = [set(p[delta[0]]['dependencies']) for p in packages]
    assert before <= after and after - before == {'jni 0.22.4', 'ndk', 'ndk-context'}
finally:
    proc.stdin.close(); proc.stdout.close(); assert proc.wait() == 0
old = read('result.json'); final = read('final-002/result.json')
assert old['head'] == initial and final['head'] == source
assert old['source_unchanged'] and final['source_unchanged']
assert old['inputs_sha256'] == h(raw['inputs.json']) and final['inputs_sha256'] == h(raw['final-002/inputs.json'])
assert old['outcome'] == 'incomplete' and final['outcome'] == 'passed'
assert len(old['commands']) == 17 and old['commands'][-1]['name'] == 'clippy-deterministic'
assert b'clippy::duplicate_mod' in raw['clippy-deterministic.log']
for prefix, result in [('', old), ('final-002/', final)]:
    assert result['child_reaped'] and result['target_released']
    for i, c in enumerate(result['commands']):
        expected = 101 if not prefix and i == 16 else 0
        assert c['exit'] == expected and c['reaped']
        assert h(raw[prefix + Path(c['log']).name]) == c['sha256']
    for stage in result['stages']:
        mode = stage['mode']; test_rows = [b for b in result['binaries'] if b['artifact']['profile']['test'] and f'/{mode}/' in b['path']]
        assert len(test_rows) == stage['harnesses'] == 15
        count = 0; packages = {}
        for i, binary in enumerate(test_rows):
            log = raw[prefix + f'test-{mode}-{i}.log']
            matches = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', log)
            assert len(matches) == 1 and matches[0][1:] == (b'0', b'0')
            n = int(matches[0][0]); count += n
            package = 'incant_game_player' if '/incant_game_player/' in binary['artifact']['target']['src_path'] else 'incant_game_runtime'
            packages[package] = packages.get(package, 0) + n
        expected = {'incant_game_player': 106 if not prefix else 103, 'incant_game_runtime': 45}
        assert packages == stage['packages'] == expected and stage['counts'] == [sum(expected.values()), 0, 0]
    for binary in result['binaries']:
        artifact = binary['artifact']; mode = next(m for m in ['deterministic', 'fast-simd8'] if f'/{m}/' in binary['path'])
        messages = []
        for line in raw[prefix + 'build-' + mode + '.log'].splitlines():
            try: messages.append(json.loads(line))
            except ValueError: pass
        assert artifact in messages and artifact['reason'] == 'compiler-artifact'
        features = set(artifact['features'])
        assert 'physics-' + mode in features and 'native-checkpoints' in features and 'static-module' in features
        if '/incant_game_player/' in artifact['target']['src_path']:
            assert {'mobile-apple', 'mobile-android', 'mobile-ffi', 'native-window'} <= features
        if a.local_binaries:
            path = Path(binary['path']); b = path.read_bytes()
            assert h(b) == binary['sha256'] and len(b) == binary['bytes'] and path.stat().st_mode & 0o111
assert len(final['stages']) == 2
assert len(final['commands']) == Q['successful_commands']
for row in read('producer-replay.json'):
    assert row['exit'] == 0 and row['reaped'] and h(raw[row['log']]) == row['log_sha256']
# Replay the unchanged Apple collector in a temporary exact producer file view.
# Its live-source assertion must never be weakened to pretend this union is old source.
AD = R / 'docs/spikes/evidence/mobile-native-platform'
AM = json.loads((AD / 'manifest.json').read_bytes())
AA = json.loads(gzip.decompress((AD / 'records.json.gz').read_bytes()))
AP = {r['path']: base64.b64decode(AA['payloads'][r['sha256']], validate=True) for r in AA['records']}
AF = json.loads(AP['006/inputs.json'])
with tempfile.TemporaryDirectory(prefix='incant-apple-verified-source-') as folder:
    root = Path(folder)
    gitdir = git('rev-parse', '--absolute-git-dir').decode().strip()
    env = dict(os.environ, GIT_DIR=gitdir, GIT_WORK_TREE=folder)
    proc = subprocess.Popen(['git', 'cat-file', '--batch'], cwd=R, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    try:
        for path, digest in AF.items():
            b = blob(AM['source'], path); assert h(b) == digest
            target = root / path; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(b)
    finally:
        proc.stdin.close(); proc.stdout.close(); assert proc.wait() == 0
    for rel in ['tools/probes/mobile-native-platform/collect.py', 'docs/spikes/evidence/mobile-native-platform/manifest.json', 'docs/spikes/evidence/mobile-native-platform/records.json.gz']:
        target = root / rel; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes((R / rel).read_bytes())
    args = ['python3', str(root / 'tools/probes/mobile-native-platform/collect.py'), '--verify-only']
    if a.originals: args.append('--originals')
    if a.local_binaries: args.append('--local-binaries')
    subprocess.run(args, cwd=root, env=env, timeout=180, check=True)
args = ['python3', str(R / 'docs/spikes/evidence/android-device-host/verify.py')]
if a.local_binaries: args.append('--local-binaries')
subprocess.run(args, cwd=R, timeout=180, check=True)
print(json.dumps({'verified': True, 'source_inputs': len(F), 'source_paths': len(changed), 'cases_per_mode': final['stages'][0]['counts'][0], 'successful_current_commands': len(final['commands']), 'retained_current_lint_failure': 1, 'records': len(raw), 'current_binary_copies': len(final['binaries']), 'device_execution': False}))
