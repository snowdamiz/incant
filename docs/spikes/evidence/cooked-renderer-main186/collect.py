from pathlib import Path
import base64, gzip, hashlib, json, re, shutil, subprocess, sys

r = Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o = Path.home()/'.codex/incant-integration-evidence/cooked-renderer-main186-001'
d = r/'docs/spikes/evidence/cooked-renderer-main186'
sha = lambda b: hashlib.sha256(b).hexdigest()
h = lambda p: sha(Path(p).read_bytes())
read = lambda p: json.loads(Path(p).read_text())
git = lambda *args: subprocess.check_output(['git', *args], cwd=r).decode().strip()
freeze = read(o/'freeze.json')
assert freeze['head'] == '0afd24a88c01161f6e27438853229b0f33b0506a'
for name, digest in freeze['files'].items():
    assert h(r/name) == digest, name
checks = read(o/'results.json')
assert len(checks) == 10 and all(c['exit'] == 0 for c in checks)
assert read(o/'terminal.json')['exit'] == 0
for c in checks:
    assert h(c['log']) == c['sha256']
counts = {}
for name in ('default-cpu', 'cooked-cpu'):
    groups = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', (o/(name+'.log')).read_text())
    counts[name] = dict(zip(('passed', 'failed', 'ignored'), (sum(int(g[i]) for g in groups) for i in range(3))))
    counts[name]['groups'] = len(groups)
    assert counts[name]['failed'] == 0 and counts[name]['ignored'] == 89
assert counts['default-cpu']['passed'] == 87
assert counts['cooked-cpu']['passed'] == 113

# The accepted cache commit contains the final renderer/compiler source union.
# The only new source delta is the separately reviewed backend test wrapper/CI.
roots = ['crates/incant_render', 'crates/incant_render_gpu_host', 'crates/incant_shader_cook']
names = git('ls-files', '--', *roots).splitlines()
rows = []
for name in names:
    accepted = subprocess.check_output(['git', 'show', '6fe64538:'+name], cwd=r)
    tested_base = subprocess.check_output(['git', 'show', freeze['head']+':'+name], cwd=r)
    assert accepted == tested_base, name
    actual = (r/name).read_bytes()
    rows.append({'path': name, 'accepted_sha256': sha(accepted), 'tested_sha256': sha(actual), 'test_wrapper_changed': accepted != actual})
assert sum(x['test_wrapper_changed'] for x in rows) == 38
assert h(o/'backend-ci.patch') == '489105e494a0c42d56dc74ec1e47fcc8ff41c3451b980e6aaeabefd7a4b8f791'
assert subprocess.check_output(['git', 'diff', freeze['head'], '--', *roots, '.github/workflows/ci.yml', '.github/workflows/desktop.yml'], cwd=r) == (o/'backend-ci.patch').read_bytes()
reviews = [
    ('cooked-renderer-backend-ci-source-20261013.json', '85cfc7cf53eed1de059eeb710fac21f3aec68166d18e757fd0422e88f1a6bf83'),
    ('claude0062-final-20261013.json', 'fceeb23c37025d8d819fa76d7d4ec6f82204e5d597e05f9b43cb1e7829d33a25'),
    ('claude0063-final-20261013.json', 'bd76d6f64c0cb99d32195f9886b07dd89a91aab7a50997461af752dc3f7e3ecc'),
    ('claude0064-final-20261013.json', '2e62c42502205aa03fc235a59169aa38c8b6a8d805012f59901bb757a427f7a0'),
]
for name, digest in reviews:
    assert h(Path.home()/'.codex/incant-reviews'/name) == digest
summary = {'checks': 10, 'frozen_inputs': len(freeze['files']), 'source_rows': len(rows), 'counts': counts, 'gpu_executed': False}
print(json.dumps(summary), flush=True)
if '--verify-only' in sys.argv:
    raise SystemExit(0)
d.mkdir(parents=True, exist_ok=True)
records, payloads = [], {}
for path in sorted(o.iterdir()):
    if not path.is_file():
        continue
    raw = path.read_bytes()
    digest = sha(raw)
    records.append({'path': str(path), 'sha256': digest, 'bytes': len(raw)})
    payloads.setdefault(digest, base64.b64encode(raw).decode())
compressed = gzip.compress(json.dumps({'records': records, 'payloads': payloads}, separators=(',', ':')).encode(), mtime=0)
parts = []
for index, start in enumerate(range(0, len(compressed), 16*1024*1024)):
    path = d/f'artifacts.json.gz.part-{index:03}'
    raw = compressed[start:start+16*1024*1024]
    path.write_bytes(raw)
    parts.append({'path': str(path.relative_to(r)), 'sha256': sha(raw), 'bytes': len(raw)})
for src, name in [(o/'freeze.json', 'freeze.json'), (o/'backend-ci.patch', 'backend-ci.patch'), (Path(__file__), 'collect.py'), (o.parent/'cooked-renderer-main186-checks.py', 'checks.py')]:
    shutil.copyfile(src, d/name)
retained_reviews = []
for name, digest in reviews:
    shutil.copyfile(Path.home()/'.codex/incant-reviews'/name, d/name)
    retained_reviews.append({'path': str((d/name).relative_to(r)), 'sha256': digest})
manifest = {
    'tested_base': freeze['head'], 'tested_source_delta': 'backend-ci.patch',
    'review_head': git('rev-parse', 'HEAD'),
    'source_freeze': {'path': str((d/'freeze.json').relative_to(r)), 'sha256': h(d/'freeze.json'), 'count': len(freeze['files'])},
    'sources': rows, 'summary': summary, 'checks': checks,
    'reviews': retained_reviews,
    'archive': {'parts': parts, 'sha256': sha(compressed), 'bytes': len(compressed), 'records': len(records), 'payloads': len(payloads)},
    'cache_producer': {'commit': '6fe64538a8268a9f6919d1f9c611fad7dc80558a', 'path': 'docs/spikes/evidence/persistent-pipeline-cache.json', 'sha256': h(r/'docs/spikes/evidence/persistent-pipeline-cache.json')},
    'review_state': 'Final current-main integration audit pending',
    'limits': [
        'Current integration executed CPU tests only; 89 GPU tests intentionally ignored in each variant. Counts overlap across default and cooked builds.',
        'The three preserved Claude receipts approve their immutable producer Metal builds and appearance, not a newly rendered integrated build.',
        'Cache remains default-off and supports trusted Linux/Android Vulkan storage only; actual Linux fresh-process persistence test remains a release-CI requirement.',
        'Default GPU tests are retained. Added cooked release tests require exact Metal/Vulkan/Dx12 adapter identity; neither current Linux nor Windows cooked execution is claimed.',
        'No full-renderer offline adoption, cache hit/performance, physical-device or phase-gate approval.',
    ],
}
path = r/'docs/spikes/evidence/cooked-renderer-main186.json'
path.write_text(json.dumps(manifest, indent=2)+'\n')
print(str(path), h(path), flush=True)
