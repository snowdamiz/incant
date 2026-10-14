#!/usr/bin/env python3
"""Package existing records only; never build, run players or measure performance."""
import base64
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
E = Path.home() / '.codex/incant-evidence'
SOURCE = '7f452e9bc1e3d82674051bd8dc366b9464d6f9d1'
BASE = '03bc4b987f3e553249846c4cfce991793d4d2c48'
records = []
payloads = {}


def add(name, path, external=False):
    data = path.read_bytes()
    sha = hashlib.sha256(data).hexdigest()
    records.append(dict(name=name, original=str(path), sha256=sha, bytes=len(data), external=external))
    if not external:
        payloads[sha] = base64.b64encode(data).decode()


for label, dirname in [('shipping', 'core-cursor-inspector-shipping-001'),
                       ('timing', 'core-cursor-inspector-pair-001')]:
    directory = E / dirname
    for path in sorted(directory.rglob('*')):
        if not path.is_file() or '__pycache__' in path.parts or path.name.startswith('packet-verification.'):
            continue
        relative = path.relative_to(directory)
        # Assets, PCMs, executable/symbol storage and the large Cargo JSON stream
        # remain exact external evidence; all raw reports/receipts are portable.
        external = (relative.parts[0] == 'authoring' or path.suffix == '.pcm'
                    or relative.parts[:2] in [('package', 'player'), ('package', 'symbols')]
                    or path.stat().st_size > 8 * 1024**2)
        add(label + '/' + relative.as_posix(), path, external)

baseline = E / 'mac-core-numeric-loan-shipping-001'
for name in ('freeze.json', 'export-result.json', 'post-build-source.json'):
    add('baseline/' + name, baseline / name)
for directory in ('functional-win', 'package'):
    for path in sorted((baseline / directory).rglob('*')):
        if not path.is_file():
            continue
        relative = path.relative_to(baseline)
        external = (path.suffix == '.pcm' or relative.parts[:2] in
                    [('package', 'player'), ('package', 'symbols')] or path.stat().st_size > 8 * 1024**2)
        add('baseline/' + relative.as_posix(), path, external)
for name in json.loads((baseline / 'freeze.json').read_text())['authoring_files']:
    add('baseline/authoring/' + name, baseline / 'authoring' / name, True)
plan = json.loads((E / 'core-cursor-inspector-pair-001/plan.json').read_text())
add('input-receipt.json', Path(plan['input_receipt']))
add('input-receipt-core-prepared.json', E / 'mac-core-shipping-002/prepared.json')
for name in ('core-cursor-loop-final-independent-20261014.json',
             'persistence-inspector-final-independent-20261014.json',
             'persistence-inspector-source-root-20261014.json'):
    add('reviews/' + name, Path.home() / '.codex/incant-reviews' / name)
for label, commit in [('baseline', BASE), ('candidate', SOURCE)]:
    paths = ['samples/core/stress.ts', 'samples/core/stress.py', 'samples/core/art/ids.py',
             'crates/incant_script/src/harden.js', 'crates/incant_script/src/persistence.rs',
             'tools/mac_performance.py', 'tools/native_process.py', 'tools/export_native.py']
    if label == 'candidate':
        paths += ['crates/incant_script/src/persistence/inspector_tests.rs', 'samples/core/stress_test.mjs']
    for path in paths:
        data = subprocess.check_output(['git', 'show', commit + ':' + path], cwd=ROOT)
        sha = hashlib.sha256(data).hexdigest()
        records.append(dict(name='source/' + label + '/' + path, original=None,
                            git_commit=commit, git_path=path, sha256=sha, bytes=len(data), external=False))
        payloads[sha] = base64.b64encode(data).decode()
archive = gzip.compress(json.dumps(payloads, sort_keys=True, separators=(',', ':')).encode(), mtime=0)
(HERE / 'records.json.gz').write_bytes(archive)
manifest = dict(schema='incant-core-cursor-inspector-result-v1', source_commit=SOURCE,
                baseline_commit=BASE, archive_sha256=hashlib.sha256(archive).hexdigest(),
                records=records, result='not accepted for production: all three2ms gates fail; pair1 frame regresses >5%',
                no_native_execution_by_collector=True,
                external_scope='Assets, authoring histories, binaries, symbols, PCM and >8MiB logs remain hash-bound external files. Portable reports/receipts prove their declared identities, not external availability. Use --external to rehash them.')
(HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps(dict(records=len(records), payloads=len(payloads), archive_bytes=len(archive))))
