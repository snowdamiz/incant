#!/usr/bin/env python3
"""Assert the product's target-specific Rapier feature policy, without claiming execution."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
TARGETS = ['aarch64-apple-darwin', 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu',
           'aarch64-apple-ios', 'aarch64-apple-ios-sim', 'aarch64-linux-android', 'wasm32-unknown-unknown']


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    cargo = 'cargo' if sys.platform == 'win32' else str(ROOT / 'tools/cargo')
    records = []
    for target in TARGETS:
        result = subprocess.run([cargo, 'tree', '--locked', '-p', 'incant_physics', '--target', target,
                                 '-e', 'normal', '--prefix', 'none', '--format', '{p}|{f}'],
                                cwd=ROOT, check=True, text=True, capture_output=True)
        matches = [line for line in result.stdout.splitlines() if line.startswith('rapier3d v0.36.0|')]
        if not matches:
            raise ValueError(f'{target}: missing pinned Rapier dependency')
        features = set(matches[0].split('|')[1].replace(' (*)', '').split(','))
        parallel = target != 'wasm32-unknown-unknown'
        if ('parallel' in features) != parallel or 'enhanced-determinism' not in features:
            raise ValueError(f'{target}: unexpected features: {sorted(features)}')
        records.append({'target': target, 'rapier_features': sorted(features), 'parallel': parallel,
                        'evidence': 'resolved feature graph only, not target execution'})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(records, indent=2) + '\n')
    print(f'Rapier feature policy passed for {len(records)} targets.')


if __name__ == '__main__':
    main()
