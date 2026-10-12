#!/usr/bin/env python3
"""Select an explicitly requested local Cargo target owned by one checkout.

This developer helper does not change Cargo or CI defaults, migrate old targets,
clean artifacts, or grant a build lease. Keep a checkout stable during builds.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

MARKER = '.incant-checkout.json'


def select(root: Path, checkout: Path) -> Path:
    checkout = checkout.expanduser().resolve(strict=True)
    if not (checkout / '.git').exists():
        raise ValueError('checkout must be a Git checkout root')
    root = root.expanduser().resolve()
    key = hashlib.sha256(os.fsencode(checkout)).hexdigest()
    target = root / ('incant-' + key)
    owner = {'version': 1, 'checkout': str(checkout)}
    root.mkdir(parents=True, exist_ok=True)
    target.mkdir(exist_ok=True)
    if target.is_symlink():
        raise ValueError('target must not be a symbolic link')
    marker = target / MARKER
    if not marker.exists():
        if any(target.iterdir()) and not marker.exists():
            raise ValueError('refusing an unowned nonempty target; choose a fresh root')
        # Publish complete metadata once. Parallel selectors for the same
        # checkout can see either no marker or the entire marker, never a prefix.
        name = None
        try:
            with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', dir=root,
                                             prefix='.incant-owner-', delete=False) as file:
                name = Path(file.name)
                json.dump(owner, file, sort_keys=True)
                file.write('\n')
                file.flush()
                os.fsync(file.fileno())
            try:
                os.link(name, marker)
            except FileExistsError:
                pass
        finally:
            if name is not None:
                name.unlink()
    if marker.is_symlink() or not marker.is_file():
        raise ValueError('invalid target ownership marker')
    try:
        actual = json.loads(marker.read_text(encoding='utf-8'))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError('invalid target ownership marker') from error
    if type(actual) is not dict or type(actual.get('version')) is not int or actual != owner:
        raise ValueError('target ownership does not match this checkout')
    return target


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True,
                        help='explicit parent directory for per-checkout targets')
    parser.add_argument('--checkout', type=Path,
                        default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    try:
        target = select(args.root, args.checkout)
    except (OSError, ValueError) as error:
        print(f'Cargo target selection refused: {error}', file=sys.stderr)
        return 1
    print(target)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
