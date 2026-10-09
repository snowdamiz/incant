#!/usr/bin/env python3
"""Record expected binding refusal separately from successful target builds."""
import argparse
import json
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--cargo', default='cargo')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    manifest = Path(__file__).resolve().parent / 'Cargo.toml'
    records = []
    targets = {
        'aarch64-apple-ios-sim': 'aarch64-apple-ios-sim is not supported yet',
        'wasm32-unknown-unknown': 'wasm32-unknown-unknown is not supported: only 64-bit targets are',
    }
    for target, refusal in targets.items():
        for backend in ['jolt', 'rapier']:
            command = [args.cargo, 'build', '--manifest-path', str(manifest), '--release', '--locked',
                       '--no-default-features', '--features', backend, '--target', target]
            log = args.output / f'{backend}-{target}.log'
            with log.open('w') as output:
                run = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT)
            expected = run.returncode == 0 if backend == 'rapier' else run.returncode != 0 and refusal in log.read_text()
            record = {'target': target, 'backend': backend, 'exit_code': run.returncode,
                      'expected_result': 'build succeeds' if backend == 'rapier' else refusal,
                      'matches_expectation': expected}
            records.append(record)
            (args.output / 'results.json').write_text(json.dumps(records, indent=2) + '\n')
            print(record, flush=True)
            if not expected:
                raise RuntimeError(f'unexpected {backend} result on {target}: inspect {log}')
    print('Portability evidence recorded. Expected Jolt binding refusal is not target support.')


if __name__ == '__main__':
    main()
