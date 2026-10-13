"""Read-only C2 source, CPU and retained-failure verification; never starts GPU."""
from pathlib import Path
import argparse
import base64
import gzip
import hashlib
import json
import re

parser = argparse.ArgumentParser()
parser.add_argument('--verify-only', action='store_true', required=True)
parser.parse_args()
root = Path(__file__).resolve().parents[4]
manifest = json.loads((root / 'docs/spikes/evidence/ssao-c2-sampling.json').read_text())
digest = lambda data: hashlib.sha256(data).hexdigest()
item = manifest['archive']
data = (root / item['path']).read_bytes()
assert digest(data) == item['sha256'] and len(data) == item['bytes']
archive = json.loads(gzip.decompress(data))
assert len(archive['records']) == item['records'] and len(archive['payloads']) == item['payloads']
raw = {}
for row in archive['records']:
    data = base64.b64decode(archive['payloads'][row['sha256']], validate=True)
    assert digest(data) == row['sha256'] and len(data) == row['bytes']
    assert Path(row['path']).read_bytes() == data
    assert row['logical'] not in raw
    raw[row['logical']] = data
freeze = json.loads(raw['source-002/freeze.json'])
assert digest(raw['source-002/freeze.json']) == manifest['source_freeze']['sha256']
for row in freeze['sources'] + freeze['baselines']:
    assert digest((root / row['path']).read_bytes()) == row['sha256']
    assert digest(raw['source-002/files/' + row['path']]) == row['sha256']
old = json.loads(raw['source-001/freeze.json'])
for row in old['sources'] + old['baselines']:
    assert digest(raw['source-001/files/' + row['path']]) == row['sha256']
    assert digest((root / row['path']).read_bytes()) == row['sha256']
checks = json.loads(raw['qualification-002/checks.json'])
assert len(checks) == 7
for row in checks:
    assert row['exit'] == 0 and row['reaped']
    for stream in ('stdout', 'stderr'):
        assert digest(raw['qualification-002/' + row['name'] + '.' + stream]) == row[stream]['sha256']
for label, expected in [('default-cpu', (105, 0, 38)), ('observer-cooked-cpu', (119, 0, 88))]:
    text = raw['qualification-002/' + label + '.stdout'].decode()
    counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    assert tuple(sum(int(c[i]) for c in counts) for i in range(3)) == expected
    output = raw['qualification-002/' + label + '.stderr'].decode()
    measurements = [json.loads(line.removeprefix('SSAO_N8_EXACT '))
                    for line in output.splitlines() if line.startswith('SSAO_N8_EXACT ')]
    assert len(measurements) == 14
    assert all(x['model_pass'] and x['required_ratio'] == 0.25 and
               x['ratio'] <= 0.25 and not x['production_admission'] for x in measurements)
    for projection, first in [(False, 124), (True, 110)]:
        assert sorted(x['row'] for x in measurements if x['orthographic'] == projection) == list(range(first, first + 7))
initial = json.loads(raw['qualification-001/checks.json'])
assert [x['exit'] for x in initial] == [0, 101]
assert b'test result: FAILED. 104 passed; 1 failed; 38 ignored;' in raw['qualification-001/default-cpu.stdout']
controls = json.loads(raw['n6t-control-replay.json'])['perspective']
assert not controls['build003_device']['n6t_pass']
assert controls['control_fine_limit_filtered_r8']['n6t_pass']
assert not controls['control_fine_limit_filtered_r8']['n6_pass']
for name in ('banded_control_1_16', 'hatch_control_pm_1_32'):
    assert not controls[name]['n6t_pass']
assert not controls['injected_step_codes']['4']['n6t_pass']
for row in manifest['binaries']:
    path = Path(row['path'])
    assert digest(path.read_bytes()) == row['sha256'] and path.stat().st_size == row['bytes']
    assert path.stat().st_mode & 0o111
assert not manifest['gpu_executed'] and not manifest['quality_accepted']
print(json.dumps(dict(verified=True, records=item['records'], payloads=item['payloads'],
                     inputs=8, checks=7, default_cpu=105, observer_cooked_cpu=119,
                     model_rows_each=14, gpu_executed=False, quality_accepted=False)))
