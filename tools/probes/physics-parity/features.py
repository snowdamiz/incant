#!/usr/bin/env python3
"""Assert the product's target-specific Rapier feature policy, without claiming execution."""
import argparse
import json
import re
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
TARGETS = ['aarch64-apple-darwin', 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu',
           'aarch64-apple-ios', 'aarch64-apple-ios-sim', 'aarch64-linux-android', 'wasm32-unknown-unknown']



def solver_features(text, solver, target):
    """Require every selected unit to name the exact pinned repository patch."""
    expected = (ROOT / 'third_party' / solver).resolve()
    found = []
    for line in text.splitlines():
        if not line.startswith(solver + ' '):
            continue
        match = re.fullmatch(rf'{solver} v0\.36\.0 \((.+)\)\|([^|]*?)(?: \(\*\))?', line)
        if not match or Path(match[1]).resolve() != expected:
            raise ValueError(f'{target}: unexpected pinned {solver} package/version/path')
        flags = set(filter(None, match[2].split(',')))
        if found and flags != found[0]:
            raise ValueError(f'{target}: conflicting {solver} feature units')
        found.append(flags)
    if not found:
        raise ValueError(f'{target}: missing pinned {solver} dependency')
    features = found[0]
    parallel = target != 'wasm32-unknown-unknown'
    if ('parallel' in features) != parallel or 'enhanced-determinism' not in features:
        raise ValueError(f'{target}: unexpected features: {sorted(features)}')
    return features


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    cargo = 'cargo' if sys.platform == 'win32' else str(ROOT / 'tools/cargo')
    records = []
    roots = {'incant_physics': ['rapier3d'], 'incant_physics_runtime': ['rapier3d'],
             'incant_physics_2d_runtime': ['rapier2d'],
             'incant_runtime': ['rapier3d', 'rapier2d'], 'incant_script_player': ['rapier3d', 'rapier2d']}
    for root, solvers in roots.items():
        for target in TARGETS:
            result = subprocess.run([cargo, 'tree', '--locked', '-p', root, '--target', target,
                                     '-e', 'normal', '--prefix', 'none', '--format', '{p}|{f}'],
                                    cwd=ROOT, check=True, text=True, capture_output=True, encoding='utf-8')
            for solver in solvers:
                features = solver_features(result.stdout, solver, target)
                parallel = target != 'wasm32-unknown-unknown'
                records.append({'root': root, 'solver': solver, 'target': target, 'rapier_features': sorted(features), 'parallel': parallel,
                                'evidence': 'resolved feature graph only, not target execution'})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(records, indent=2) + '\n')
    print(f'Rapier feature policy passed for {len(records)} targets.')


if __name__ == '__main__':
    main()
