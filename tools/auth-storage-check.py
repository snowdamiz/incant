#!/usr/bin/env python3
"""Verify synthetic credential persistence across changed native executables.

Never opens an account file or reads provider credentials. The Rust probe uses a
fresh UUID namespace, a hardcoded fake record and the application's real backend.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('artifacts/auth-storage.json'))
    args = parser.parse_args()
    cargo = subprocess.check_output(['rustup', 'which', 'cargo'], cwd=ROOT, text=True).strip()
    env = os.environ.copy()
    env['PATH'] = str(Path(cargo).parent) + os.pathsep + env['PATH']
    probe_id = str(uuid.uuid4())
    binary = ROOT / 'target/debug/examples' / ('credential_probe.exe' if sys.platform == 'win32' else 'credential_probe')
    fingerprints = []
    saved = False

    def probe(mode):
        subprocess.run([str(binary), mode, probe_id], cwd=ROOT, env=env, check=True, timeout=60)

    try:
        for revision in ('before', 'after'):
            env['INCANT_PROBE_BUILD'] = f'{probe_id}-{revision}'
            subprocess.run([cargo, 'build', '-p', 'incant_agent', '--example', 'credential_probe', '--locked'], cwd=ROOT, env=env, check=True, timeout=900)
            fingerprints.append(hashlib.sha256(binary.read_bytes()).hexdigest())
            if revision == 'before':
                # Cleanup is attempted even if saving fails after partial writes.
                saved = True
                probe('save')
            else:
                if fingerprints[0] == fingerprints[1]:
                    raise RuntimeError('The probe executable did not change')
                probe('verify')
    finally:
        if saved:
            probe('delete')

    report = {
        'platform': sys.platform,
        'storage': 'private-local-file' if sys.platform == 'darwin' else 'os-keychain',
        'synthetic_data_only': True,
        'changed_build_persistence': True,
        'cleanup_verified': True,
        'executable_sha256': fingerprints,
        'live_oauth': False,
    }
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
