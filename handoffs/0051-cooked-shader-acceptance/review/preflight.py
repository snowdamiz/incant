#!/usr/bin/env python3
"""CPU-only advisory check run before each capture.py child; never launches a binary.

Re-hashes the child's frozen binary against every manifest that names it, checks
the root gate and RELEASED latch, and snapshots any process already running a
frozen 0051 binary. Writes one create-new record per child and exits nonzero on
any mismatch so the helper is not invoked. The helper repeats its own checks
under its lock; this script does not replace or modify them.
"""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
PACKET = ROOT / 'handoffs/0051-cooked-shader-acceptance'
TOOLS = ROOT / 'artifacts/tools/0051'
RECORDS = ROOT / 'artifacts/0051-cooked-shader-acceptance/preflight'
SUFFIX = {'output': 'incant-render-tests', 'lookdev': 'model_gpu',
          'masks': 'model_gpu', 'diagnostic': 'incant_render'}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_hashes(case, mode):
    """Every independent statement of the binary's SHA256, keyed by source."""
    staged = json.loads((PACKET / 'staged-inputs.json').read_text())['binaries']
    prepared = json.loads((PACKET / 'prepared-inputs.json').read_text())
    suffix = SUFFIX[case]
    if case == 'output':
        relative = f'output-v1/{mode}-{suffix}'
        manifest = json.loads((TOOLS / 'output-v1/manifest.json').read_text())
        pairing = json.loads((TOOLS / 'output-v1/source-pairing.json').read_text())
        return relative, {
            'staged-inputs.json': staged[relative],
            'output-v1/manifest.json': manifest['binaries'][mode]['sha256'],
            'output-v1/source-pairing.json': pairing['binaries'][mode]['sha256'],
        }
    group = 'cooked-geometry' if mode == 'cooked' else 'default'
    relative = f'geometry-v2/{group}-{suffix}'
    manifest = json.loads((TOOLS / 'geometry-v2/manifest.json').read_text())
    return relative, {
        'staged-inputs.json': staged[relative],
        'prepared-inputs.json': prepared['geometry_binaries'][group][suffix]['sha256'],
        'geometry-v2/manifest.json': manifest['runs'][group]['binaries'][suffix]['sha256'],
    }


def main():
    case, mode = sys.argv[1], sys.argv[2]
    assert case in SUFFIX and mode in ('default', 'cooked'), 'unknown case/mode'
    prepared = json.loads((PACKET / 'prepared-inputs.json').read_text())
    relative, expected = expected_hashes(case, mode)
    actual = digest(TOOLS / relative)
    gate = TOOLS / 'gpu-review-ready.json'
    grant = json.loads(gate.read_text()) if gate.exists() else None
    manifests = {
        'geometry-v2/manifest.json': (digest(TOOLS / 'geometry-v2/manifest.json'),
                                      prepared['geometry_manifest']['sha256']),
        'output-v1/source-pairing.json': (digest(TOOLS / 'output-v1/source-pairing.json'),
                                          prepared['output_pairing']['sha256']),
    }
    processes = subprocess.run(['ps', '-axo', 'pid=,command='], capture_output=True,
                               text=True, check=True).stdout.splitlines()
    frozen = [line.strip() for line in processes if str(TOOLS) in line
              and ('geometry-v2/' in line or 'output-v1/' in line)]
    checks = {
        'released_latch_absent': not (TOOLS / 'RELEASED.json').exists(),
        'gate_present': grant is not None,
        'root_grant_true': bool(grant) and grant.get('root_grant') is True,
        'gate_prepared_inputs_match': bool(grant) and grant.get('prepared_inputs_sha256') == digest(PACKET / 'prepared-inputs.json'),
        'gate_helper_match': bool(grant) and grant.get('capture_helper_sha256') == digest(PACKET / 'capture.py'),
        'binary_matches_every_manifest': all(actual == value for value in expected.values()),
        'pairing_manifests_match_prepared_inputs': all(a == b for a, b in manifests.values()),
        'no_frozen_binary_running': not frozen,
    }
    record = {
        'name': f'{case}-{mode}',
        'checked_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'binary': relative,
        'binary_sha256': actual,
        'expected_sha256_by_source': expected,
        'manifest_hashes': {k: {'actual': a, 'prepared_inputs': b} for k, (a, b) in manifests.items()},
        'gate_sha256': digest(gate) if gate.exists() else None,
        'frozen_binary_processes': frozen,
        'checks': checks,
        'passed': all(checks.values()),
    }
    RECORDS.mkdir(parents=True, exist_ok=True)
    with (RECORDS / f'{case}-{mode}.json').open('x') as out:
        out.write(json.dumps(record, indent=2) + '\n')
    print(json.dumps({'name': record['name'], 'passed': record['passed'], 'checks': checks}))
    if not record['passed']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
