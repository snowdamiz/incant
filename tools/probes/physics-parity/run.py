#!/usr/bin/env python3
"""Compare actual full-state serial/native-parallel/browser execution, never timings."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
FIELDS = ['stable_body_id', 'position.x', 'position.y', 'position.z',
          'quaternion.x', 'quaternion.y', 'quaternion.z', 'quaternion.w',
          'linear_velocity.x', 'linear_velocity.y', 'linear_velocity.z',
          'angular_velocity.x', 'angular_velocity.y', 'angular_velocity.z', 'sleeping']


def compare(reference, candidate):
    """Compare bytes, naming the first public field mismatch instead of trusting a hash."""
    if len(reference) != len(candidate):
        raise ValueError(f'stream sizes differ: {len(reference)} != {len(candidate)}')
    if reference == candidate:
        return
    bodies = struct.unpack_from('<I', reference, 12)[0]
    for offset in range(0, len(reference), 4):
        before, after = reference[offset:offset + 4], candidate[offset:offset + 4]
        if before != after:
            word = offset // 4
            if word < 16:
                location = f'header word {word}'
            else:
                record, field = divmod(word - 16, 15)
                frame, body = divmod(record, bodies)
                location = f'frame {frame}, body {body}, {FIELDS[field]}'
            raise ValueError(f'{location}: {before.hex()} != {after.hex()}')


def inspect(data, scenario):
    if len(data) < 64:
        raise ValueError('missing state-stream header')
    header = struct.unpack_from('<16I', data)
    if (scenario not in (0, 1) or header[:3] != (0x50415231, 1, scenario)
            or header[3] != (273 if scenario == 0 else 217) or header[4:6] != (641, 15)
            or header[6:8] != (0x3C800000, 0xC1000000) or any(header[12:])):
        raise ValueError('unexpected stream format/scenario/step count')
    if len(data) != 64 + header[3] * header[4] * header[5] * 4:
        raise ValueError('missing frames or bodies')
    if header[8] < 20 or (scenario == 0 and min(header[9:11]) < 16) or (scenario == 1 and header[11] != 192):
        raise ValueError('contact/sleep/wake/joint coverage is missing')
    return {'scenario': scenario, 'bodies': header[3], 'frames': header[4],
            'max_contacts': header[8], 'max_sleeping': header[9], 'wake_transitions': header[10],
            'joints': header[11], 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}


def command(args, *, capture=False):
    print(' '.join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), cwd=ROOT, check=True, text=True,
                          capture_output=capture).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--trials', type=int, default=3)
    parser.add_argument('--browser', action='store_true')
    parser.add_argument('--cargo', default='cargo' if sys.platform == 'win32' else str(ROOT / 'tools/cargo'))
    parser.add_argument('--wasm-bindgen', default='wasm-bindgen')
    args = parser.parse_args()
    if args.trials < 2:
        parser.error('at least two independent process trials are required')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    manifest = HERE / 'Cargo.toml'
    target = HERE / 'target'
    suffix = '.exe' if sys.platform == 'win32' else ''
    build = [args.cargo, 'build', '-j2', '--manifest-path', manifest, '--release', '--locked', '--no-default-features', '--target-dir', target]
    binaries = {}
    for mode, features in [('serial', []), ('parallel', ['--features', 'parallel'])]:
        command(build + ['--bin', 'incant-physics-parity'] + features)
        binaries[mode] = output / f'{mode}{suffix}'
        shutil.copy2(target / 'release' / f'incant-physics-parity{suffix}', binaries[mode])
    references = {}
    records = []
    for trial in range(args.trials):
        modes = [('serial', 0), ('parallel', 1), ('parallel', 4)]
        modes = modes[trial % 3:] + modes[:trial % 3]
        for mode, workers in modes:
            for scenario in [0, 1]:
                file = output / f'{mode}-{workers}-{trial}-{scenario}.bin'
                execution = json.loads(command([binaries[mode], scenario, workers, file], capture=True))
                if execution['workers'] != workers:
                    raise ValueError('worker pool execution mismatch')
                data = file.read_bytes()
                record = inspect(data, scenario)
                references.setdefault(scenario, data)
                try:
                    compare(references[scenario], data)
                except ValueError as error:
                    raise ValueError(f'{file.name}: {error}') from error
                records.append(dict(record, trial=trial, mode=mode, workers=workers, file=file.name))
    if args.browser:
        command(build + ['--lib', '--target', 'wasm32-unknown-unknown'])
        command([args.wasm_bindgen, '--target', 'web', '--out-dir', output / 'pkg',
                 target / 'wasm32-unknown-unknown/release/incant_physics_parity.wasm'])
        command(['node', HERE / 'browser.mjs', output, args.trials])
        for trial in range(args.trials):
            for scenario in [0, 1]:
                file = output / f'browser-{trial}-{scenario}.bin'
                data = file.read_bytes()
                record = inspect(data, scenario)
                try:
                    compare(references[scenario], data)
                except ValueError as error:
                    raise ValueError(f'{file.name}: {error}') from error
                records.append(dict(record, trial=trial, mode='browser-wasm-serial', workers=0, file=file.name))
    report = {'status': 'passed', 'host': {'system': platform.system(), 'machine': platform.machine()},
              'rustc': command([command(['rustup', 'which', 'rustc'], capture=True).strip(), '-vV'], capture=True),
              'cpu_count': os.cpu_count(), 'trials': args.trials,
              'browser_executed': args.browser,
              'browser': json.loads((output / 'browser.json').read_text()) if args.browser else [],
              'source_sha256': {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                                for path in [ROOT / 'Cargo.lock', ROOT / 'crates/incant_physics/Cargo.toml',
                                             HERE / 'Cargo.toml', HERE / 'Cargo.lock', HERE / 'src/lib.rs',
                                             HERE / 'src/main.rs', HERE / 'run.py', HERE / 'browser.mjs']},
              'records': records,
              'reference': [inspect(references[scenario], scenario) for scenario in [0, 1]],
              'scope': 'public body-state parity; no timing, device, backend-adoption or hidden solver-state claim'}
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': 'passed', 'streams': len(records), 'browser_executed': args.browser}))


if __name__ == '__main__':
    main()
