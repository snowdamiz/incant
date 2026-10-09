#!/usr/bin/env python3
"""Run isolated physics candidates; compare complete position streams, not timings."""
import argparse
import hashlib
import json
import platform
import statistics
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--trials', type=int, default=3, choices=range(2, 6))
    parser.add_argument('--backend', choices=['all', 'jolt', 'rapier'], default='all')
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    binary = root / 'target/release' / ('incant-physics-probe.exe' if platform.system() == 'Windows' else 'incant-physics-probe')
    modes = {'all': ['jolt1', 'rapier', 'jolt4'], 'jolt': ['jolt1', 'jolt4'], 'rapier': ['rapier']}[args.backend]
    # Do not overwrite earlier evidence or compare against stale files.
    args.output.mkdir(parents=True, exist_ok=False)
    records = []
    baseline = {}
    for trial in range(args.trials):
        order = modes if trial % 2 == 0 else list(reversed(modes))
        for mode in order:
            positions = args.output / f'{mode}-{trial}.positions.bin'
            response = subprocess.run([str(binary), mode, str(positions.resolve())], text=True, capture_output=True, check=True)
            row = json.loads(response.stdout)
            values = positions.read_bytes()
            if len(values) != 512 * 600 * 3 * 4:
                raise RuntimeError(f'{mode}: incomplete position stream')
            if mode in baseline and baseline[mode] != values:
                raise RuntimeError(f'{mode}: position stream differs across processes')
            baseline[mode] = values
            row.update(mode=mode, trial=trial + 1, positions_sha256=hashlib.sha256(values).hexdigest())
            records.append(row)
            print(f'{mode}, trial {trial + 1}: {row["median_ms"]:.4f} ms', flush=True)
    if 'jolt1' in baseline and 'jolt4' in baseline and baseline['jolt1'] != baseline['jolt4']:
        raise RuntimeError('Jolt one/four-worker position streams differ')
    summary = [{
        'mode': mode,
        'median_of_trial_medians_ms': statistics.median(row['median_ms'] for row in records if row['mode'] == mode),
        'positions_sha256': hashlib.sha256(baseline[mode]).hexdigest(),
    } for mode in modes]
    report = {
        'host': {'system': platform.system(), 'machine': platform.machine()},
        'scope': '512 boxes, 600 fixed steps; only position sequences compared; no cross-platform claim from one host',
        'candidate_versions': {'oxijolt': '1.0.1+jolt-5.6.0', 'rapier': '0.36.0'},
        'summary': summary,
        'records': records,
    }
    (args.output / 'results.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Repeated full position streams match exactly.')


if __name__ == '__main__':
    main()
