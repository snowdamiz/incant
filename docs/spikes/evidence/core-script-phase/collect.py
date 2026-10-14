#!/usr/bin/env python3
"""Collect the already executed experiment; never launches a native workload."""
import base64
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
E = Path('/Users/andreyyurlov/.codex/incant-evidence/core-script-phase-001')
paths = ['run-001.py', 'run-001.started.json', 'run-001.json', 'run-001.stdout',
         'run-001.stderr', 'build-inputs-001.json', 'analysis-001.json',
         'prepare-001.stdout', 'prepare-001.stderr']
for directory in ['source-001', 'prepared-001', 'frozen-binaries']:
    paths.extend(str(p.relative_to(E)) for p in (E / directory).rglob('*') if p.is_file())
records = []
for relative in sorted(paths):
    path = E / relative
    data = path.read_bytes()
    row = dict(path=relative, original=str(path), bytes=len(data),
               sha256=hashlib.sha256(data).hexdigest())
    if not relative.startswith('frozen-binaries/'):
        row['payload'] = base64.b64encode(data).decode()
    records.append(row)
archive = gzip.compress(json.dumps(records, separators=(',', ':')).encode(), mtime=0)
(HERE / 'records.json.gz').write_bytes(archive)
manifest = dict(version=1, source_commit='13b178c81998b6b74bb4db2b06b0fc343cea0eb0',
                base_commit='03bc4b987f3e553249846c4cfce991793d4d2c48',
                archive_sha256=hashlib.sha256(archive).hexdigest(),
                records=len(records), portable_payloads=sum('payload' in r for r in records),
                scope='One explicitly selected CI native ScriptHost diagnostic, not shipping or quiet timing',
                external_scope='One immediate 0755 test executable is hash-bound, not embedded. Original paths optional for --external replay.',
                test='tick_probe::tests::actual_native_stress_regions_128',
                ticks=128, rows_per_tick=9999, omitted_write_controls=3,
                build='Rust1.99.0, explicit aarch64-apple-darwin, ci, offline+locked, two jobs, Rayon2; compiler/profile/physics-fast-simd8',
                failures=[], production_algorithm_changed=False,
                limits=['Only the unchanged stress script and 9999 rows; no Core controller/assets/UI/audio/physics.',
                        'All intervals include the first tick; oracle, compilation and fixture setup are outside each interval.',
                        'Region means are additive; percentile values are not additive or shipping gate evidence.',
                        'Parent reported rendering stopped; a peer was allowed bounded raw CPU analysis.',
                        'One exact test passed; no full suite or strict Clippy qualification claimed.',
                        'No player Frame guard: frame histograms are empty, while script zones and heap have128 samples.'])
(HERE / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(json.dumps({k:v for k,v in manifest.items() if k in ['records','portable_payloads','archive_sha256','source_commit']}))
