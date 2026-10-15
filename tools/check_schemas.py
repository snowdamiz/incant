#!/usr/bin/env python3
"""Compare fresh headless schemas without changing the published registry."""
import argparse
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def check_schemas(generator, published):
    """Return exact file/byte mismatches; never generate into the source tree."""
    with tempfile.TemporaryDirectory(prefix='incant-schema-check-') as temporary:
        generated = Path(temporary)
        subprocess.run([*generator, 'schema', str(generated)], check=True, timeout=60)
        expected = {path.name: path for path in published.iterdir()}
        actual = {path.name: path for path in generated.iterdir()}
        problems = []
        for name in sorted(expected.keys() | actual.keys()):
            if name not in expected:
                problems.append(f'Unpublished generated schema: {name}')
            elif name not in actual:
                problems.append(f'Published schema no longer generated: {name}')
            elif not expected[name].is_file() or not actual[name].is_file():
                problems.append(f'Expected a schema file: {name}')
            elif expected[name].read_bytes() != actual[name].read_bytes():
                problems.append(f'Stale published schema: {name}')
        if not expected and not actual:
            problems.append('Schema registry is empty')
        return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    problems = check_schemas([str(args.binary.resolve(strict=True))], ROOT / 'schemas')
    if problems:
        print('\n'.join(problems))
        print('Regenerate the published registry with incant_headless schema schemas, then review its diff.')
        return 1
    print('Fresh headless schemas match every published file byte for byte.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
